import SwiftUI

struct SettingsView: View {
    @ObservedObject var settings = AppSettings.shared
    @State private var pageCount: UInt64 = 0
    @State private var indexSize: String = ""
    @State private var showDeleteConfirm = false
    @State private var showReindexConfirm = false
    @State private var isReindexing = false

    private let engine: SearchEngineWrapper? = {
        let path = SearchViewModel.resolveIndexPath()
        return SearchEngineWrapper(indexPath: path)
    }()

    var body: some View {
        Form {
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Browser Extension")
                        .font(.headline)
                    Text("Pages captured by the Alexandria browser extension are automatically indexed.")
                        .font(.caption)
                        .foregroundColor(.secondary)

                    HStack {
                        Text(settings.storePath)
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .foregroundColor(.secondary)
                            .font(.callout)
                    }
                }
            }

            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Power")
                        .font(.headline)
                    Toggle("Index while on battery power", isOn: $settings.indexOnBattery)
                    if settings.indexOnBattery {
                        Text("Indexing will pause automatically when charge is low.")
                            .font(.caption)
                            .foregroundColor(.secondary)
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

                        Button("Reindex") {
                            showReindexConfirm = true
                        }
                        .disabled(pageCount == 0 || isReindexing)

                        Button(role: .destructive) {
                            showDeleteConfirm = true
                        } label: {
                            Text("Delete History")
                        }
                        .disabled(pageCount == 0 || isReindexing)
                    }
                }
            }
        }
        .formStyle(.grouped)
        .frame(width: 480, height: 280)
        .alert("Delete History?", isPresented: $showDeleteConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Delete", role: .destructive) {
                deleteHistory()
            }
        } message: {
            Text("Are you sure? Your entire browsing history will be permanently deleted. This cannot be undone.")
        }
        .alert("Reindex?", isPresented: $showReindexConfirm) {
            Button("Cancel", role: .cancel) {}
            Button("Reindex") {
                reindex()
            }
        } message: {
            Text("This will rebuild the search index from your stored pages. This may take a while.")
        }
        .onAppear {
            refreshStats()
        }
    }

    private func refreshStats() {
        pageCount = engine?.docCount() ?? 0
        indexSize = Self.directorySize(at: SearchViewModel.resolveIndexPath())
    }

    private func deleteHistory() {
        _ = engine?.deleteHistory(storePath: settings.storePath)
        refreshStats()
    }

    private func reindex() {
        isReindexing = true
        DispatchQueue.global(qos: .utility).async {
            _ = engine?.reindex(storePath: settings.storePath)
            DispatchQueue.main.async {
                isReindexing = false
                refreshStats()
            }
        }
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
