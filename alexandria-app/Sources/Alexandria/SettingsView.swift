import SwiftUI

struct SettingsView: View {
    @ObservedObject var settings = AppSettings.shared
    @State private var showFolderPicker = false
    @State private var pageCount: UInt64 = 0
    @State private var indexSize: String = ""
    @State private var showClearConfirm = false

    private let engine: SearchEngineWrapper? = {
        let path = SearchViewModel.resolveIndexPath()
        return SearchEngineWrapper(indexPath: path)
    }()

    var body: some View {
        Form {
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Recoll Webcache Folder")
                        .font(.headline)
                    Text("Pages saved by the Recoll Firefox extension will be automatically indexed.")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    HStack {
                        if settings.webcachePath.isEmpty {
                            Text("Not configured")
                                .foregroundColor(.secondary)
                        } else {
                            Text(settings.webcachePath)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                        Spacer()
                        Button("Choose...") {
                            showFolderPicker = true
                        }
                        if !settings.webcachePath.isEmpty {
                            Button(role: .destructive) {
                                settings.webcachePath = ""
                            } label: {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.secondary)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                }
            }

            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Index")
                        .font(.headline)

                    HStack {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("Pages indexed: \(pageCount)")
                            Text("Index size: \(indexSize)")
                        }
                        .foregroundColor(.secondary)
                        .font(.callout)

                        Spacer()

                        Button(role: .destructive) {
                            showClearConfirm = true
                        } label: {
                            Text("Clear Index")
                        }
                        .disabled(pageCount == 0)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .frame(width: 480, height: 220)
        .fileImporter(
            isPresented: $showFolderPicker,
            allowedContentTypes: [.folder]
        ) { result in
            if case .success(let url) = result {
                settings.webcachePath = url.path
            }
        }
        .alert("Clear Index?", isPresented: $showClearConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Clear", role: .destructive) {
                clearIndex()
            }
        } message: {
            Text("Are you sure? Your entire browsing history index will be erased. This cannot be undone.")
        }
        .onAppear {
            refreshStats()
        }
    }

    private func refreshStats() {
        pageCount = engine?.docCount() ?? 0
        indexSize = Self.directorySize(at: SearchViewModel.resolveIndexPath())
    }

    private func clearIndex() {
        _ = engine?.clearIndex()
        refreshStats()
    }

    static func directorySize(at path: String) -> String {
        let fm = FileManager.default
        guard let enumerator = fm.enumerator(atPath: path) else { return "0 KB" }
        var totalBytes: UInt64 = 0
        while let file = enumerator.nextObject() as? String {
            let fullPath = (path as NSString).appendingPathComponent(file)
            if let attrs = try? fm.attributesOfItem(atPath: fullPath),
               let size = attrs[.size] as? UInt64 {
                totalBytes += size
            }
        }
        let formatter = ByteCountFormatter()
        formatter.allowedUnits = [.useKB, .useMB, .useGB]
        formatter.countStyle = .file
        return formatter.string(fromByteCount: Int64(totalBytes))
    }
}
