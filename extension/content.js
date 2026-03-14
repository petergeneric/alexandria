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
      html: document.documentElement.outerHTML,
      title: document.title || "",
      url: location.href,
      contentType: document.contentType || "text/html",
      charset: document.characterSet || "UTF-8",
    };
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
