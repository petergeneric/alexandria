import SwiftUI

struct SettingsView: View {
    @ObservedObject var settings = AppSettings.shared
    @State private var showFolderPicker = false

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
        }
        .formStyle(.grouped)
        .frame(width: 480, height: 160)
        .fileImporter(
            isPresented: $showFolderPicker,
            allowedContentTypes: [.folder]
        ) { result in
            if case .success(let url) = result {
                settings.webcachePath = url.path
            }
        }
    }
}
