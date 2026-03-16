// Background service worker: manages native messaging connection and page capture

const NATIVE_HOST = "alexandria";
const CHUNK_SIZE = 900 * 1024; // ~900KB per chunk to stay under 1MB native messaging limit
const MAX_PAGE_SIZE = 5 * 1024 * 1024; // 5MB max page size

let port = null;

function connectNative() {
  if (port) return port;
  try {
    port = browser.runtime.connectNative(NATIVE_HOST);
    port.onDisconnect.addListener(function () {
      console.log("Alexandria: native host disconnected", port?.error || "");
      port = null;
    });
    port.onMessage.addListener(function (response) {
      if (response.status === "error") {
        console.error("Alexandria: native host error:", response.message);
      }
      if (response.blocked_domains) {
        AlexRules.mergeBlockedDomains(response.blocked_domains);
      }
    });
    // Request the latest blocklist from the native host
    port.postMessage({ type: "get_blocklist" });
  } catch (e) {
    console.error("Alexandria: failed to connect to native host:", e);
    port = null;
  }
  return port;
}

function sendToNative(data) {
  var p = connectNative();
  if (!p) {
    console.error("Alexandria: no native connection");
    return;
  }

  var htmlByteLength = new TextEncoder().encode(data.html || "").length;
  if (htmlByteLength > MAX_PAGE_SIZE) {
    console.warn("Alexandria: page too large (" + (htmlByteLength / 1024 / 1024).toFixed(1) + "MB), skipping:", data.url);
    return;
  }

  var jsonByteLength = new TextEncoder().encode(JSON.stringify(data)).length;
  if (jsonByteLength <= CHUNK_SIZE) {
    p.postMessage(data);
  } else {
    // Chunk the HTML
    sendChunked(p, data);
  }
}

function sendChunked(p, data) {
  var html = data.html;
  var id = crypto.randomUUID();
  var chunks = [];
  var i = 0;
  while (i < html.length) {
    var end = Math.min(i + CHUNK_SIZE, html.length);
    // Don't split a UTF-16 surrogate pair
    if (end < html.length && html.charCodeAt(end - 1) >= 0xd800 && html.charCodeAt(end - 1) <= 0xdbff) {
      end++;
    }
    chunks.push(html.substring(i, end));
    i = end;
  }
  var total = chunks.length;

  for (var seq = 0; seq < total; seq++) {
    var msg = {
      type: "chunk",
      id: id,
      seq: seq,
      total: total,
      data: chunks[seq],
    };
    if (seq === total - 1) {
      msg.meta = {
        url: data.url,
        title: data.title,
        content_type: data.contentType || "text/html",
        timestamp: data.timestamp,
      };
    }
    p.postMessage(msg);
  }
}

function captureTab(tab) {
  if (!tab || !tab.id || tab.id < 0) return;
  if (AlexRules.isSpecialPage(tab.url)) return;

  setBadge(tab.id, "SAVE", "#4285f4");

  Promise.race([
    browser.tabs.sendMessage(tab.id, { action: "performAction" }),
    new Promise(function (_, reject) {
      setTimeout(function () { reject(new Error("timeout")); }, 5000);
    }),
  ])
    .then(function (response) {
      if (response && response.html) {
        sendToNative({
          type: "snapshot",
          url: response.url,
          title: response.title,
          html: response.html,
          content_type: response.contentType || "text/html",
          timestamp: Math.floor(Date.now() / 1000),
        });
      }
      clearBadge(tab.id);
    })
    .catch(function (err) {
      console.error("Alexandria: capture failed:", err);
      setBadge(tab.id, "ERR", "#cc0000");
      setTimeout(function () {
        clearBadge(tab.id);
      }, 2000);
    });
}

function setBadge(tabId, text, color) {
  browser.action.setBadgeText({ text: text, tabId: tabId });
  browser.action.setBadgeBackgroundColor({ color: color, tabId: tabId });
}

function clearBadge(tabId) {
  browser.action.setBadgeText({ text: "", tabId: tabId });
}

// Toolbar button click: manual save
browser.action.onClicked.addListener(function (tab) {
  captureTab(tab);
});

// Context menus
browser.contextMenus.create({
  id: "save-page",
  title: "Save this page to Alexandria",
  contexts: ["page"],
});

browser.contextMenus.onClicked.addListener(function (info, tab) {
  if (info.menuItemId === "save-page") {
    captureTab(tab);
  }
});

// Handle auto-save messages from content script
browser.runtime.onMessage.addListener(function (message, sender) {
  if (message.type === "autosave" && message.data) {
    sendToNative({
      type: "snapshot",
      url: message.data.url,
      title: message.data.title,
      html: message.data.html,
      content_type: message.data.contentType || "text/html",
      timestamp: Math.floor(Date.now() / 1000),
    });
  }
});

// Update action state when tabs change
browser.tabs.onActivated.addListener(function (activeInfo) {
  browser.tabs.get(activeInfo.tabId).then(function (tab) {
    updateActionState(tab);
  });
});

browser.tabs.onUpdated.addListener(function (tabId, changeInfo, tab) {
  if (changeInfo.status === "complete") {
    updateActionState(tab);
  }
});

function updateActionState(tab) {
  if (!tab || !tab.url) return;
  if (AlexRules.isSpecialPage(tab.url)) {
    browser.action.disable(tab.id);
  } else {
    browser.action.enable(tab.id);
  }
}
