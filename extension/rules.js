// URL filtering rules for auto-save decisions
// AUTO-GENERATED from shared/blocklist.json — do not edit by hand.
// Regenerate with: python3 shared/generate_rules.py

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

  // Internal blacklist: sensitive account, payment, and auth pages.
  // Partitioned into Sets by dot-component count for efficient lookup:
  // only extract and test the suffix lengths that actually appear in the list.
  var INTERNAL_BLOCKED_LIST = [
    "account.ui.com",
    "1password.com",
    "accounts.google.com",
    "accounts.google.co.uk",
    "accounts.google.ie",
    "accounts.youtube.com",
    "admin.google.com",
    "admin.google.co.uk",
    "admin.google.ie",
    "account.apple.com",
    "appleid.apple.com",
    "account.microsoft.com",
    "dash.cloudflare.com",
    "pay.stripe.com",
    "hooks.stripe.com",
    "billing.stripe.com",
    "dashboard.stripe.com",
    "accounts.firefox.com",
    "paypal.com",
    "venmo.com",
    "cash.app",
    "wise.com",
    "revolut.com",
    "id.atlassian.com",
    "login.microsoftonline.com",
    "auth0.com",
    "bankofamerica.com",
    "chase.com",
    "wellsfargo.com",
    "citibankonline.com",
    "usbank.com",
    "capitalone.com",
    "ally.com",
    "discover.com",
    "schwab.com",
    "fidelity.com",
    "vanguard.com",
    "tdameritrade.com",
    "etrade.com",
    "marcus.com",
    "sofi.com",
    "lloydsbank.co.uk",
    "hsbc.co.uk",
    "barclays.co.uk",
    "my.crunch.co.uk",
    "caterallen.co.uk",
    "caterallenonline.co.uk",
    "natwest.com",
    "nationwide.co.uk",
    "santander.co.uk",
    "tsb.co.uk",
    "monzo.com",
    "starlingbank.com",
    "metrobankonline.co.uk",
    "365online.com",
    "bankofireland.com",
    "aib.ie",
    "ptsb.ie",
    "ulsterbank.ie",
    "kbc.ie",
    "n26.com",
    "avant.ie",
    "ebs.ie",
    "an-post.com",
  ];

  // Build Sets keyed by component count: { 2: Set["chase.com", ...], 3: Set["accounts.google.com", ...] }
  var BLOCKED_BY_DEPTH = {};
  for (var i = 0; i < INTERNAL_BLOCKED_LIST.length; i++) {
    var d = INTERNAL_BLOCKED_LIST[i];
    var depth = d.split(".").length;
    if (!BLOCKED_BY_DEPTH[depth]) BLOCKED_BY_DEPTH[depth] = new Set();
    BLOCKED_BY_DEPTH[depth].add(d);
  }

  // Extract the last N dot-components from a hostname.
  // e.g. suffix("www.accounts.google.com", 3) → "accounts.google.com"
  function domainSuffix(hostname, n) {
    var dot = hostname.length;
    for (var i = 0; i < n; i++) {
      dot = hostname.lastIndexOf(".", dot - 1);
      if (dot === -1) return n === i + 1 ? hostname : null;
    }
    return hostname.substring(dot + 1);
  }

  function isInternalBlocked(hostname) {
    for (var depth in BLOCKED_BY_DEPTH) {
      var suffix = domainSuffix(hostname, +depth);
      if (suffix && BLOCKED_BY_DEPTH[depth].has(suffix)) return true;
    }
    return false;
  }

  // login.(any).(tld) pattern
  var LOGIN_DOMAIN_RE = /^login\.[^.]+\.[^.]+$/;

  // Paths that indicate checkout, payment, or auth flows
  var BLOCKED_PATH_PREFIXES = [
    "/en/checkout",
    "/checkout",
    "/checkoutnow",
    "/oauth",
    "/authorize",
    "/callback",
    "/reset-password",
    "/forgot-password",
    "/2fa",
    "/mfa",
    "/verify",
  ];

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
    if (pattern.indexOf(".") !== -1 && hostname.endsWith("." + pattern)) return true;
    return false;
  }

  // Parse URL and check against internal hard blocks (banks, auth, etc.).
  // Returns { hostname, pathname } if safe, or null if hard-blocked.
  function parseAndCheck(url) {
    var parsed;
    try {
      parsed = new URL(url);
    } catch (e) {
      return null;
    }
    var hostname = parsed.hostname;
    if (isInternalBlocked(hostname)) return null;
    if (LOGIN_DOMAIN_RE.test(hostname)) return null;
    var lowerPath = parsed.pathname.toLowerCase();
    for (var i = 0; i < BLOCKED_PATH_PREFIXES.length; i++) {
      if (lowerPath === BLOCKED_PATH_PREFIXES[i] ||
          lowerPath.startsWith(BLOCKED_PATH_PREFIXES[i] + "/")) return null;
    }
    return { hostname: hostname };
  }

  function matchesAnyDomain(hostname, list) {
    for (var i = 0; i < list.length; i++) {
      var pattern = list[i].trim();
      if (!pattern || pattern.startsWith("#")) continue;
      if (matchesDomain(hostname, pattern)) return true;
    }
    return false;
  }

  // Decide whether a URL should be auto-saved given the current mode.
  // disabledDomains always takes preference over enabledDomains.
  function shouldAutoSave(url, mode, enabledDomains, disabledDomains) {
    var info = parseAndCheck(url);
    if (!info) return false;
    if (disabledDomains && disabledDomains.length > 0) {
      if (matchesAnyDomain(info.hostname, disabledDomains)) return false;
    }
    if (mode === "enabled") {
      if (!enabledDomains || enabledDomains.length === 0) return false;
      return matchesAnyDomain(info.hostname, enabledDomains);
    }
    return true;
  }

  return {
    isSpecialPage: isSpecialPage,
    shouldSkipMime: shouldSkipMime,
    shouldAutoSave: shouldAutoSave,
    matchesDomain: matchesDomain,
  };
})();

if (typeof globalThis !== "undefined") {
  globalThis.AlexRules = AlexRules;
}
