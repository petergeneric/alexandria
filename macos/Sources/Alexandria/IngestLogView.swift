import SwiftUI

extension IngestLogEntry: Identifiable {}

struct IngestLogView: View {
    let engine: SearchEngineWrapper?
    @State private var entries: [IngestLogEntry] = []
    @State private var selectedEntryId: Int64?

    private var selectedEntry: IngestLogEntry? {
        entries.first { $0.id == selectedEntryId }
    }

    var body: some View {
        VStack(spacing: 0) {
            if entries.isEmpty {
                Spacer()
                Text("No indexing failures")
                    .foregroundColor(.secondary)
                Spacer()
            } else {
                Table(entries, selection: $selectedEntryId, columns: {
                    TableColumn("Time") { entry in
                        Text(entry.timestamp)
                            .font(.callout.monospaced())
                    }
                    .width(min: 140, ideal: 170)

                    TableColumn("URL") { entry in
                        Text(entry.url)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    .width(min: 200, ideal: 400)
                })

                Divider()

                detailPane
                    .frame(height: 120)

                Divider()

                HStack {
                    Text("\(entries.count) failure\(entries.count == 1 ? "" : "s")")
                        .font(.callout)
                        .foregroundColor(.secondary)

                    Spacer()

                    Button("Copy All") {
                        copyAll()
                    }

                    Button("Clear Log") {
                        engine?.clearIngestLog()
                        entries = []
                        selectedEntryId = nil
                    }
                }
                .padding(12)
            }
        }
        .frame(minWidth: 640, minHeight: 400)
        .onAppear {
            entries = engine?.recentIngestFailures() ?? []
        }
    }

    @ViewBuilder
    private var detailPane: some View {
        if let entry = selectedEntry {
            ScrollView {
                VStack(alignment: .leading, spacing: 6) {
                    detailRow("Time", entry.timestamp)
                    detailRow("Page ID", String(entry.pageId))
                    detailRow("URL", entry.url)
                    detailRow("Domain", entry.domain)
                    detailRow("Reason", entry.reason)
                }
                .padding(12)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
        } else {
            Text("Select a row to view details")
                .foregroundColor(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }

    private func detailRow(_ label: String, _ value: String) -> some View {
        HStack(alignment: .top, spacing: 8) {
            Text(label)
                .font(.callout.bold())
                .frame(width: 60, alignment: .trailing)
            Text(value)
                .font(.callout.monospaced())
                .textSelection(.enabled)
        }
    }

    private func copyAll() {
        let text = entries.map { "\($0.timestamp)\t\($0.url)\t\($0.reason)" }.joined(separator: "\n")
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}
