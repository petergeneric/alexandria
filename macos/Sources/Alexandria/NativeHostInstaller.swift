import Foundation

struct NativeHostInstaller {
    private static let hostName = "alexandria"
    private static let firefoxExtensionId = "alexandria@works.peter"
    // To find your Chrome extension ID: load the unpacked extension in chrome://extensions
    // and copy the 32-character ID shown beneath the extension name.
    private static let chromeExtensionId = "TODO_PASTE_CHROME_EXTENSION_ID_HERE"

    /// Path to the native host binary bundled inside the .app
    private static var bundledBinaryPath: String? {
        // Contents/Helpers/alexandria-native-host
        Bundle.main.executableURL?
            .deletingLastPathComponent()            // Contents/MacOS/
            .deletingLastPathComponent()            // Contents/
            .appendingPathComponent("Helpers")
            .appendingPathComponent("alexandria-native-host")
            .path
    }

    /// Install native messaging manifests for all supported browsers.
    /// Safe to call on every launch — overwrites with current path.
    static func installManifests() {
        guard let binaryPath = bundledBinaryPath else {
            print("NativeHostInstaller: could not resolve bundled binary path")
            return
        }

        guard FileManager.default.fileExists(atPath: binaryPath) else {
            print("NativeHostInstaller: native host binary not found at \(binaryPath)")
            return
        }

        installFirefoxManifest(binaryPath: binaryPath)
        installChromeManifest(binaryPath: binaryPath)
    }

    private static func installFirefoxManifest(binaryPath: String) {
        let dir = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Mozilla/NativeMessagingHosts")
        let manifest: [String: Any] = [
            "name": hostName,
            "description": "Alexandria page capture native host",
            "path": binaryPath,
            "type": "stdio",
            "allowed_extensions": [firefoxExtensionId],
        ]
        writeManifest(manifest, to: dir.appendingPathComponent("\(hostName).json"))
    }

    private static func installChromeManifest(binaryPath: String) {
        let dir = FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent("Library/Application Support/Google/Chrome/NativeMessagingHosts")
        let manifest: [String: Any] = [
            "name": hostName,
            "description": "Alexandria page capture native host",
            "path": binaryPath,
            "type": "stdio",
            "allowed_origins": ["chrome-extension://\(chromeExtensionId)/"],
        ]
        writeManifest(manifest, to: dir.appendingPathComponent("\(hostName).json"))
    }

    private static func writeManifest(_ manifest: [String: Any], to url: URL) {
        let fm = FileManager.default
        let dir = url.deletingLastPathComponent()

        do {
            try fm.createDirectory(at: dir, withIntermediateDirectories: true)
            let data = try JSONSerialization.data(withJSONObject: manifest, options: [.prettyPrinted, .sortedKeys])
            try data.write(to: url, options: .atomic)
            print("NativeHostInstaller: wrote \(url.path)")
        } catch {
            print("NativeHostInstaller: failed to write \(url.path): \(error)")
        }
    }
}
