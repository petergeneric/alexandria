import AppKit
import SwiftUI

class SearchPanel: NSPanel {
    override var canBecomeKey: Bool { true }

    init() {
        super.init(
            contentRect: NSRect(x: 0, y: 0, width: 880, height: 480),
            styleMask: [.titled, .closable, .resizable, .nonactivatingPanel, .utilityWindow],
            backing: .buffered,
            defer: false
        )

        title = "Alexandria"
        isFloatingPanel = true
        level = .floating
        isMovableByWindowBackground = true
        hidesOnDeactivate = false
        titlebarAppearsTransparent = true
        titleVisibility = .hidden

        // Allow Esc to close
        _ = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            if event.keyCode == 53 {
                self?.close()
                return nil
            }
            return event
        }

        let hostingView = NSHostingView(rootView: SearchView())
        contentView = hostingView

        center()
    }

    func focusSearchField() {
        guard let contentView else { return }
        if let textField = findTextField(in: contentView) {
            makeFirstResponder(textField)
        }
    }

    private func findTextField(in view: NSView) -> NSTextField? {
        for subview in view.subviews {
            if let textField = subview as? NSTextField, textField.isEditable {
                return textField
            }
            if let found = findTextField(in: subview) {
                return found
            }
        }
        return nil
    }
}
