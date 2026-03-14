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
                if let when = result.visitedAt {
                    Text(formatRelativeTime(when))
                        .font(.caption2)
                        .foregroundColor(.gray)
                }
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

private func formatRelativeTime(_ date: Date) -> String {
    let now = Date()
    let seconds = now.timeIntervalSince(date)
    let calendar = Calendar.current

    if seconds < 0 {
        return formatTime(date)
    }

    // Less than 5 minutes: "just now"
    if seconds < 300 {
        return "just now"
    }

    // Last hour
    if seconds < 3600 {
        let mins = Int(seconds / 60)
        return "\(mins) minutes ago"
    }

    // Today: show time
    if calendar.isDateInToday(date) {
        return formatTime(date)
    }

    // Yesterday
    if calendar.isDateInYesterday(date) {
        return "yesterday \(formatTime(date))"
    }

    // This week (within last 7 days)
    let days = calendar.dateComponents([.day], from: calendar.startOfDay(for: date), to: calendar.startOfDay(for: now)).day ?? 0
    if days < 7 {
        let dayName = date.formatted(.dateTime.weekday(.wide))
        return "\(dayName) \(formatTime(date))"
    }

    // Last week
    if days < 14 {
        return "last week"
    }

    // Last 10 months: "4 Jan"
    let months = calendar.dateComponents([.month], from: date, to: now).month ?? 0
    if months < 10 {
        return date.formatted(.dateTime.day().month(.abbreviated))
    }

    // Older: "4 Jan 2025"
    return date.formatted(.dateTime.day().month(.abbreviated).year())
}

private func formatTime(_ date: Date) -> String {
    date.formatted(.dateTime.hour().minute())
}

/// Highlight query keywords in text using a yellow/orange background tint.
/// Falls back to bold when the user has enabled high-contrast accessibility.
private func highlightKeywords(in text: String, query: String) -> AttributedString {
    var attributed = AttributedString(text)
    attributed.foregroundColor = .secondary

    let keywords = query
        .split(separator: " ")
        .map { $0.lowercased() }
        .filter { !["and", "or", "not"].contains($0) }

    guard !keywords.isEmpty else { return attributed }

    let useHighContrast = NSWorkspace.shared.accessibilityDisplayShouldIncreaseContrast

    let lower = text.lowercased()
    for keyword in keywords {
        var searchStart = lower.startIndex
        while let range = lower.range(of: keyword, range: searchStart..<lower.endIndex) {
            let attrStart = AttributedString.Index(range.lowerBound, within: attributed)
            let attrEnd = AttributedString.Index(range.upperBound, within: attributed)
            if let attrStart, let attrEnd {
                attributed[attrStart..<attrEnd].foregroundColor = .primary
                if useHighContrast {
                    attributed[attrStart..<attrEnd].inlinePresentationIntent = .stronglyEmphasized
                } else {
                    attributed[attrStart..<attrEnd].backgroundColor = .yellow.opacity(0.3)
                }
            }
            searchStart = range.upperBound
        }
    }

    return attributed
}
