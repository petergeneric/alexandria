// URL filtering rules for auto-save decisions

var AlexRules = (function () {
  // Special URL schemes that should never be captured
  var SPECIAL_SCHEMES = [
    "about:",
    "chrome:",
    "chrome-extension:",
    "moz-extension:",
    "resource:",
    "data:",
    "blob:",
    "javascript:",
    "file:",
  ];

  // MIME types to skip during auto-save
  var SKIP_MIME_PREFIXES = ["video/", "audio/", "image/"];

  function isSpecialPage(url) {
    if (!url) return true;
    for (var i = 0; i < SPECIAL_SCHEMES.length; i++) {
      if (url.startsWith(SPECIAL_SCHEMES[i])) return true;
    }
    return false;
  }

  function shouldSkipMime(contentType) {
    if (!contentType) return false;
    var lower = contentType.toLowerCase();
    for (var i = 0; i < SKIP_MIME_PREFIXES.length; i++) {
      if (lower.startsWith(SKIP_MIME_PREFIXES[i])) return true;
    }
    return false;
  }

  function matchesDomain(hostname, pattern) {
    if (hostname === pattern) return true;
    if (hostname.endsWith("." + pattern)) return true;
    return false;
  }

  function globToRegex(glob) {
    var escaped = glob.replace(/[.+^${}()|[\]\\]/g, "\\$&");
    escaped = escaped.replace(/\*/g, ".*").replace(/\?/g, ".");
    return new RegExp("^" + escaped + "$");
  }

  // Evaluate URL against a blocklist of domain patterns.
  // Returns true if the URL should be blocked.
  function isBlocked(url, blocklist) {
    if (!blocklist || blocklist.length === 0) return false;
    var hostname;
    try {
      hostname = new URL(url).hostname;
    } catch (e) {
      return false;
    }
    for (var i = 0; i < blocklist.length; i++) {
      var pattern = blocklist[i].trim();
      if (!pattern || pattern.startsWith("#")) continue;
      if (matchesDomain(hostname, pattern)) return true;
    }
    return false;
  }

  return {
    isSpecialPage: isSpecialPage,
    shouldSkipMime: shouldSkipMime,
    isBlocked: isBlocked,
    matchesDomain: matchesDomain,
    globToRegex: globToRegex,
  };
})();

if (typeof globalThis !== "undefined") {
  globalThis.AlexRules = AlexRules;
}
