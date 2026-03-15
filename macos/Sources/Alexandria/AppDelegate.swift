import AppKit
import SwiftUI

class AppDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem!
    private var searchPanel: SearchPanel?
    private var settingsWindow: NSWindow?

    private var ingester: Ingester?
    private var engine: SearchEngineWrapper?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Hide dock icon — menu bar only
        NSApp.setActivationPolicy(.accessory)

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem.button {
            if let iconURL = Bundle.module.url(forResource: "icon", withExtension: "svg"),
               let image = NSImage(contentsOf: iconURL) {
                image.isTemplate = true
                image.size = NSSize(width: 18, height: 18)
                button.image = image
            }
            button.action = #selector(statusItemClicked)
            button.target = self
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }

        // Install/update native messaging host manifests
        NativeHostInstaller.installManifests()

        // Set up shared engine and background ingester
        let indexPath = SearchViewModel.resolveIndexPath()
        engine = SearchEngineWrapper(indexPath: indexPath, appDbPath: AppSettings.defaultAppDbPath)
        if let engine = engine {
            ingester = Ingester(engine: engine)
            // Ingest on launch
            ingester?.ingestIfNeeded()
        }

    }

    @objc private func statusItemClicked() {
        guard let event = NSApp.currentEvent else { return }
        if event.type == .rightMouseUp {
            showMenu()
        } else {
            toggleSearchPanel()
        }
    }

    private func toggleSearchPanel() {
        if let panel = searchPanel, panel.isVisible {
            panel.close()
        } else {
            showSearchPanel()
        }
    }

    private func showSearchPanel() {
        if searchPanel == nil {
            searchPanel = SearchPanel()
        }
        searchPanel?.makeKeyAndOrderFront(nil)
        searchPanel?.center()
        NSApp.activate(ignoringOtherApps: true)

        // Focus the search field after the view hierarchy is laid out
        DispatchQueue.main.async { [weak self] in
            self?.searchPanel?.focusSearchField()
        }

        // Trigger background ingest when panel is opened
        ingester?.ingestIfNeeded()
    }

    private func showMenu() {
        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Settings...", action: #selector(openSettings), keyEquivalent: ""))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Quit Alexandria", action: #selector(NSApplication.terminate(_:)), keyEquivalent: ""))
        statusItem.menu = menu
        statusItem.button?.performClick(nil)
        // Clear menu so left-click goes back to action-based handling
        statusItem.menu = nil
    }

    @objc private func openSettings() {
        if settingsWindow == nil {
            let hostingView = NSHostingView(rootView: SettingsView(engine: engine))
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 480, height: 220),
                styleMask: [.titled, .closable],
                backing: .buffered,
                defer: false
            )
            window.title = "Alexandria Settings"
            window.contentView = hostingView
            window.center()
            settingsWindow = window
        }
        searchPanel?.close()
        settingsWindow?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationWillTerminate(_ notification: Notification) {
    }
}
