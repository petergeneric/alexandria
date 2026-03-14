import SwiftUI

struct SearchView: View {
    @StateObject private var viewModel = SearchViewModel()

    var body: some View {
        VStack(spacing: 0) {
            // Search field
            HStack(spacing: 8) {
                Image(systemName: "magnifyingglass")
                    .foregroundColor(.secondary)
                    .font(.title3)

                TextField("Search your browsing history...", text: $viewModel.query)
                    .textFieldStyle(.plain)
                    .font(.title3)
                    .onSubmit { viewModel.performSearch() }

                if !viewModel.query.isEmpty {
                    Button(action: { viewModel.query = ""; viewModel.results = [] }) {
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(.secondary)
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(12)

            Divider()

            // Results
            if viewModel.results.isEmpty && !viewModel.query.isEmpty && !viewModel.isSearching {
                Spacer()
                Text("No results found")
                    .foregroundColor(.secondary)
                    .font(.body)
                Spacer()
            } else if viewModel.results.isEmpty && viewModel.query.isEmpty {
                Spacer()
                VStack(spacing: 8) {
                    Image(systemName: "building.columns")
                        .font(.system(size: 40))
                        .foregroundColor(.secondary)
                    Text("Search your browsing history")
                        .foregroundColor(.secondary)
                        .font(.body)
                    Text("⌘⇧Space to open from anywhere")
                        .foregroundColor(.gray)
                        .font(.caption)
                }
                Spacer()
            } else {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 0) {
                        ForEach(viewModel.results) { result in
                            ResultRow(result: result, query: viewModel.query)
                                .onTapGesture {
                                    openURL(result.url)
                                }
                        }
                    }
                }
            }
        }
        .frame(minWidth: 500, minHeight: 300)
    }

    private func openURL(_ urlString: String) {
        guard let url = URL(string: urlString) else { return }
        NSWorkspace.shared.open(url)
    }
}

struct ResultRow: View {
    let result: SearchResult
    var query: String = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(result.title.isEmpty ? result.url : result.title)
                .font(.headline)
                .lineLimit(1)

            Text(result.url)
                .font(.caption)
                .foregroundColor(.accentColor)
                .lineLimit(1)

            if !result.snippet.isEmpty {
                Text(highlightKeywords(in: result.snippet, query: query))
                    .font(.body)
                    .lineLimit(3)
            }

            HStack {
                Text(result.domain)
                    .font(.caption2)
                    .foregroundColor(.gray)
                Spacer()
                Text(String(format: "%.1f", result.score))
                    .font(.caption2)
                    .foregroundColor(.gray)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .contentShape(Rectangle())
        .background(Color.clear)
        .onHover { hovering in
            if hovering {
                NSCursor.pointingHand.push()
            } else {
                NSCursor.pop()
            }
        }

        Divider()
            .padding(.leading, 12)
    }
}

/// Highlight query keywords in text using bold + primary color against secondary base.
private func highlightKeywords(in text: String, query: String) -> AttributedString {
    var attributed = AttributedString(text)
    attributed.foregroundColor = .secondary

    let keywords = query
        .split(separator: " ")
        .map { $0.lowercased() }
        .filter { !["and", "or", "not"].contains($0) }

    guard !keywords.isEmpty else { return attributed }

    let lower = text.lowercased()
    for keyword in keywords {
        var searchStart = lower.startIndex
        while let range = lower.range(of: keyword, range: searchStart..<lower.endIndex) {
            let attrStart = AttributedString.Index(range.lowerBound, within: attributed)
            let attrEnd = AttributedString.Index(range.upperBound, within: attributed)
            if let attrStart, let attrEnd {
                attributed[attrStart..<attrEnd].inlinePresentationIntent = .stronglyEmphasized
                attributed[attrStart..<attrEnd].foregroundColor = .primary
            }
            searchStart = range.upperBound
        }
    }

    return attributed
}
