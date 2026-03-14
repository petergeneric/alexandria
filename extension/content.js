// Content script: captures page HTML and responds to background messages

(function () {
  // Listen for capture requests from the background script
  browser.runtime.onMessage.addListener(function (message, sender, sendResponse) {
    if (message.action === "performAction") {
      var data = capturePage();
      sendResponse(data);
    }
    return false;
  });

  function capturePage() {
    return {
      html: sanitizedOuterHTML(),
      title: document.title || "",
      url: location.href,
      contentType: document.contentType || "text/html",
      charset: document.characterSet || "UTF-8",
    };
  }

  var SKIP_TAGS = { INPUT: true, SELECT: true, STYLE: true };

  // Serialize the live DOM, skipping input state and sensitive elements.
  function sanitizedOuterHTML() {
    var parts = [];
    serializeNode(document.documentElement, parts);
    return parts.join("");
  }

  function serializeNode(node, parts) {
    if (node.nodeType === Node.TEXT_NODE) {
      parts.push(escapeHTML(node.textContent));
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) return;

    var tag = node.tagName;

    // Skip <input> and <select> entirely
    if (SKIP_TAGS[tag]) return;

    // Skip <textarea> with fewer than 5 words
    if (tag === "TEXTAREA") {
      var text = (node.textContent || "").trim();
      var wordCount = text ? text.split(/\s+/).length : 0;
      if (wordCount < 5) return;
    }

    parts.push("<");
    parts.push(tag.toLowerCase());

    // Serialize attributes, filtering sensitive ones
    for (var i = 0; i < node.attributes.length; i++) {
      var attr = node.attributes[i];
      // Strip form action URLs
      if (tag === "FORM" && attr.name === "action") continue;
      parts.push(" ");
      parts.push(attr.name);
      parts.push('="');
      parts.push(escapeAttr(attr.value));
      parts.push('"');
    }
    parts.push(">");

    // For contenteditable, skip children (user-entered content)
    var ce = node.getAttribute("contenteditable");
    if (ce === "true" || ce === "") {
      parts.push("</" + tag.toLowerCase() + ">");
      return;
    }

    var children = node.childNodes;
    for (var i = 0; i < children.length; i++) {
      serializeNode(children[i], parts);
    }

    parts.push("</" + tag.toLowerCase() + ">");
  }

  function escapeHTML(text) {
    return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  function escapeAttr(value) {
    return value.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
  }

  // --- Auto-save with engagement gating ---

  function getResponseStatus() {
    try {
      var nav = performance.getEntriesByType("navigation");
      if (nav.length > 0 && nav[0].responseStatus !== undefined) {
        return nav[0].responseStatus;
      }
    } catch (e) {}
    // API unavailable — assume 200 so we don't block on older browsers
    return 200;
  }

  var saved = false;

  function doAutoSave() {
    if (saved) return;
    saved = true;
    teardownEngagement();

    browser.storage.local.get(
      ["options-autosave", "options-blocklist"],
      function (result) {
        var autosave =
          result["options-autosave"] === undefined
            ? true
            : result["options-autosave"];
        if (!autosave) return;

        var blocklist = result["options-blocklist"] || [];
        if (AlexRules.isBlocked(location.href, blocklist)) return;

        var data = capturePage();
        browser.runtime.sendMessage({ type: "autosave", data: data });
      }
    );
  }

  // Engagement tracking: user must interact or stay focused for >5 seconds
  var focusTimer = null;
  var FOCUS_DELAY = 5000;

  function onInteraction() {
    doAutoSave();
  }

  function onFocus() {
    if (focusTimer) return;
    focusTimer = setTimeout(doAutoSave, FOCUS_DELAY);
  }

  function onBlur() {
    if (focusTimer) {
      clearTimeout(focusTimer);
      focusTimer = null;
    }
  }

  var INTERACTION_EVENTS = ["click", "scroll", "keydown"];

  function setupEngagement() {
    for (var i = 0; i < INTERACTION_EVENTS.length; i++) {
      window.addEventListener(INTERACTION_EVENTS[i], onInteraction, {
        once: true,
        passive: true,
      });
    }
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    // Start focus timer immediately if page already has focus
    if (document.hasFocus()) {
      onFocus();
    }
  }

  function teardownEngagement() {
    for (var i = 0; i < INTERACTION_EVENTS.length; i++) {
      window.removeEventListener(INTERACTION_EVENTS[i], onInteraction);
    }
    window.removeEventListener("focus", onFocus);
    window.removeEventListener("blur", onBlur);
    if (focusTimer) {
      clearTimeout(focusTimer);
      focusTimer = null;
    }
  }

  function initAutoSave() {
    if (AlexRules.isSpecialPage(location.href)) return;
    if (AlexRules.shouldSkipMime(document.contentType)) return;
    if (getResponseStatus() !== 200) return;

    setupEngagement();
  }

  if (document.readyState === "complete") {
    initAutoSave();
  } else {
    window.addEventListener("load", initAutoSave);
  }
})();
