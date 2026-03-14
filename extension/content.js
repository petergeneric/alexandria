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

  // Auto-save on page load if enabled
  function tryAutoSave() {
    if (AlexRules.isSpecialPage(location.href)) return;
    if (AlexRules.shouldSkipMime(document.contentType)) return;

    browser.storage.local.get(
      ["options-autosave", "options-blocklist"],
      function (result) {
        // Default: autosave on
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

  // Run auto-save after the page is fully loaded
  if (document.readyState === "complete") {
    tryAutoSave();
  } else {
    window.addEventListener("load", tryAutoSave);
  }
})();
