import AppKit
import SwiftUI

class AppDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem!
    private var searchPanel: SearchPanel?
    private var settingsWindow: NSWindow?
    private var globalMonitor: Any?
    private var ingester: Ingester?

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Hide dock icon — menu bar only
        NSApp.setActivationPolicy(.accessory)

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem.button {
            button.image = NSImage(
                systemSymbolName: "building.columns",
                accessibilityDescription: "Alexandria"
            )
            button.action = #selector(statusItemClicked)
            button.target = self
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }

        // Set up background ingester
        let indexPath = SearchViewModel.resolveIndexPath()
        if let engine = SearchEngineWrapper(indexPath: indexPath) {
            ingester = Ingester(engine: engine)
            // Ingest on launch
            ingester?.ingestIfNeeded()
        }

        // Global hotkey: Cmd+Shift+Space
        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            if event.modifierFlags.contains([.command, .shift]) && event.keyCode == 49 {
                DispatchQueue.main.async {
                    self?.showSearchPanel()
                }
            }
        }

        // Local monitor for when the app is focused
        NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            if event.modifierFlags.contains([.command, .shift]) && event.keyCode == 49 {
                DispatchQueue.main.async {
                    self?.showSearchPanel()
                }
                return nil
            }
            return event
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

        // Trigger background ingest when panel is opened
        ingester?.ingestIfNeeded()
    }

    private func showMenu() {
        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Settings...", action: #selector(openSettings), keyEquivalent: ","))
        menu.addItem(NSMenuItem.separator())
        menu.addItem(NSMenuItem(title: "Quit Alexandria", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
        statusItem.menu = menu
        statusItem.button?.performClick(nil)
        // Clear menu so left-click goes back to action-based handling
        statusItem.menu = nil
    }

    @objc private func openSettings() {
        if settingsWindow == nil {
            let hostingView = NSHostingView(rootView: SettingsView())
            let window = NSWindow(
                contentRect: NSRect(x: 0, y: 0, width: 480, height: 160),
                styleMask: [.titled, .closable],
                backing: .buffered,
                defer: false
            )
            window.title = "Alexandria Settings"
            window.contentView = hostingView
            window.center()
            settingsWindow = window
        }
        settingsWindow?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }

    func applicationWillTerminate(_ notification: Notification) {
        if let monitor = globalMonitor {
            NSEvent.removeMonitor(monitor)
        }
    }
}
