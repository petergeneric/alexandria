// Options page logic

(function () {
  var autosaveEl = document.getElementById("autosave");
  var modeEnabledEl = document.getElementById("mode-enabled");
  var modeAllEl = document.getElementById("mode-all");
  var enabledSection = document.getElementById("enabled-section");
  var enabledEl = document.getElementById("enabled-domains");
  var disabledEl = document.getElementById("disabled-domains");
  var saveBtn = document.getElementById("save-btn");
  var pingBtn = document.getElementById("ping-btn");
  var statusEl = document.getElementById("status");

  function updateSections() {
    enabledSection.style.display = modeEnabledEl.checked ? "" : "none";
  }

  modeEnabledEl.addEventListener("change", updateSections);
  modeAllEl.addEventListener("change", updateSections);

  // Load saved settings
  browser.storage.local.get(
    ["options-autosave", "options-mode", "options-enabled-domains", "options-disabled-domains"],
    function (result) {
      if (result["options-autosave"] !== undefined) {
        autosaveEl.checked = result["options-autosave"];
      }
      if (result["options-mode"] === "all") {
        modeAllEl.checked = true;
      } else {
        modeEnabledEl.checked = true;
      }
      if (result["options-enabled-domains"]) {
        enabledEl.value = result["options-enabled-domains"].join("\n");
      }
      if (result["options-disabled-domains"]) {
        disabledEl.value = result["options-disabled-domains"].join("\n");
      }
      updateSections();
    }
  );

  function parseList(el) {
    return el.value
      .split("\n")
      .map(function (s) { return s.trim(); })
      .filter(function (s) { return s.length > 0; });
  }

  saveBtn.addEventListener("click", function () {
    var mode = modeEnabledEl.checked ? "enabled" : "all";

    browser.storage.local.set({
      "options-autosave": autosaveEl.checked,
      "options-mode": mode,
      "options-enabled-domains": parseList(enabledEl),
      "options-disabled-domains": parseList(disabledEl),
    });

    statusEl.className = "status ok";
    statusEl.textContent = "Settings saved.";
    setTimeout(function () {
      statusEl.textContent = "";
      statusEl.className = "status";
    }, 2000);
  });

  pingBtn.addEventListener("click", function () {
    statusEl.className = "status pending";
    statusEl.textContent = "Connecting...";

    try {
      var port = browser.runtime.connectNative("alexandria");
      port.postMessage({ type: "ping" });
      port.onMessage.addListener(function (response) {
        if (response.status === "ok") {
          statusEl.className = "status ok";
          statusEl.textContent =
            "Connected to native host" +
            (response.version ? " v" + response.version : "");
        } else {
          statusEl.className = "status error";
          statusEl.textContent = "Error: " + (response.message || "unknown");
        }
        port.disconnect();
      });
      port.onDisconnect.addListener(function () {
        if (statusEl.className === "status pending") {
          statusEl.className = "status error";
          statusEl.textContent =
            "Connection failed. Is the native host installed?";
        }
      });
    } catch (e) {
      statusEl.className = "status error";
      statusEl.textContent = "Failed to connect: " + e.message;
    }
  });

  // Initial ping
  pingBtn.click();
})();
