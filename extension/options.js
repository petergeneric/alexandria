// Options page logic

(function () {
  var autosaveEl = document.getElementById("autosave");
  var httpsOnlyEl = document.getElementById("https-only");
  var blocklistEl = document.getElementById("blocklist");
  var saveBtn = document.getElementById("save-btn");
  var pingBtn = document.getElementById("ping-btn");
  var statusEl = document.getElementById("status");

  // Load saved settings
  browser.storage.local.get(
    ["options-autosave", "options-https-only", "options-blocklist"],
    function (result) {
      if (result["options-autosave"] !== undefined) {
        autosaveEl.checked = result["options-autosave"];
      }
      if (result["options-https-only"] !== undefined) {
        httpsOnlyEl.checked = result["options-https-only"];
      }
      if (result["options-blocklist"]) {
        blocklistEl.value = result["options-blocklist"].join("\n");
      }
    }
  );

  saveBtn.addEventListener("click", function () {
    var blocklist = blocklistEl.value
      .split("\n")
      .map(function (s) { return s.trim(); })
      .filter(function (s) { return s.length > 0; });

    browser.storage.local.set({
      "options-autosave": autosaveEl.checked,
      "options-https-only": httpsOnlyEl.checked,
      "options-blocklist": blocklist,
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
