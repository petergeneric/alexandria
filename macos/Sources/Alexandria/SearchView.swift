import SwiftUI

struct SearchView: View {
    @ObservedObject var viewModel: SearchViewModel
    @State private var showFacets = true

    private var hasResults: Bool { !viewModel.results.isEmpty }
    private var hasActiveFilters: Bool {
        viewModel.selectedDateRange != .all || !viewModel.selectedDomains.isEmpty
    }

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

                if hasResults {
                    Button(action: { withAnimation(.easeInOut(duration: 0.2)) { showFacets.toggle() } }) {
                        Image(systemName: "sidebar.right")
                            .foregroundColor(showFacets ? .accentColor : .secondary)
                    }
                    .buttonStyle(.plain)
                    .help("Toggle filters")
                }
            }
            .padding(12)

            Divider()

            // Results + Facet sidebar
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
                }
                Spacer()
            } else {
                HStack(spacing: 0) {
                    // Results list
                    ScrollView {
                        LazyVStack(alignment: .leading, spacing: 0) {
                            if hasActiveFilters {
                                FilterBanner(viewModel: viewModel)
                            }
                            ForEach(viewModel.filteredResults) { result in
                                ResultRow(result: result, query: viewModel.query)
                                    .onTapGesture {
                                        openURL(result.url)
                                    }
                            }
                            if viewModel.filteredResults.isEmpty {
                                VStack {
                                    Spacer(minLength: 40)
                                    Text("No results match the current filters")
                                        .foregroundColor(.secondary)
                                        .font(.body)
                                        .frame(maxWidth: .infinity)
                                    Spacer(minLength: 40)
                                }
                            }
                        }
                    }

                    // Facet sidebar
                    if showFacets {
                        Divider()
                        FacetSidebar(viewModel: viewModel)
                            .frame(width: 200)
                            .transition(.move(edge: .trailing).combined(with: .opacity))
                    }
                }
            }
            // Pending pages status bar
            if viewModel.pendingCount > 0 {
                Divider()
                PendingStatusBar(viewModel: viewModel)
            }
        }
        .frame(minWidth: 600, minHeight: 300)
    }

    private func openURL(_ urlString: String) {
        guard let url = URL(string: urlString) else { return }
        NSWorkspace.shared.open(url)
    }
}

private struct PendingStatusBar: View {
    @ObservedObject var viewModel: SearchViewModel

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: "clock")
                .font(.caption2)
                .foregroundColor(.secondary)

            Text(statusText)
                .font(.caption)
                .foregroundColor(.secondary)

            Spacer()

            if viewModel.isIndexing {
                ProgressView()
                    .controlSize(.small)
                Text("Indexing...")
                    .font(.caption)
                    .foregroundColor(.secondary)
            } else {
                Button("Index Now") {
                    viewModel.indexNow()
                }
                .font(.caption)
                .buttonStyle(.plain)
                .foregroundColor(.accentColor)
            }
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(Color(nsColor: .windowBackgroundColor))
    }

    private var statusText: String {
        let count = viewModel.pendingCount
        let pages = count == 1 ? "page" : "pages"
        if let oldest = viewModel.pendingOldest {
            let ago = Self.relativeAge(oldest)
            return "Excludes \(count) \(pages) pending index from last \(ago)"
        }
        return "Excludes \(count) \(pages) pending index"
    }

    private static func relativeAge(_ date: Date) -> String {
        let seconds = Int(Date().timeIntervalSince(date))
        if seconds < 60 { return "minute" }
        let minutes = seconds / 60
        if minutes < 60 {
            return minutes == 1 ? "minute" : "\(minutes) minutes"
        }
        let hours = minutes / 60
        if hours < 24 {
            return hours == 1 ? "hour" : "\(hours) hours"
        }
        let days = hours / 24
        return days == 1 ? "day" : "\(days) days"
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

// MARK: - Facet Sidebar

private struct FacetSidebar: View {
    @ObservedObject var viewModel: SearchViewModel

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // Date range facet
                VStack(alignment: .leading, spacing: 6) {
                    Text("Date Range")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .textCase(.uppercase)

                    ForEach(DateRangeFilter.allCases) { filter in
                        let count = viewModel.dateRangeCounts[filter] ?? 0
                        Button(action: {
                            viewModel.selectedDateRange = (viewModel.selectedDateRange == filter) ? .all : filter
                        }) {
                            HStack {
                                Image(systemName: viewModel.selectedDateRange == filter ? "checkmark.circle.fill" : "circle")
                                    .font(.caption)
                                    .foregroundColor(viewModel.selectedDateRange == filter ? .accentColor : .secondary)
                                Text(filter.rawValue)
                                    .font(.caption)
                                    .foregroundColor(.primary)
                                Spacer()
                                Text("\(count)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                        }
                        .buttonStyle(.plain)
                        .disabled(count == 0 && filter != .all)
                    }
                }

                Divider()

                // Domain facet
                VStack(alignment: .leading, spacing: 6) {
                    HStack {
                        Picker("", selection: $viewModel.siteFacetMode) {
                            ForEach(SiteFacetMode.allCases, id: \.self) { mode in
                                Text(mode.rawValue)
                            }
                        }
                        .pickerStyle(.segmented)
                        .labelsHidden()
                        .frame(maxWidth: 120)
                        Spacer()
                        if !viewModel.selectedDomains.isEmpty {
                            Button("Clear") {
                                viewModel.selectedDomains.removeAll()
                            }
                            .font(.caption2)
                            .buttonStyle(.plain)
                            .foregroundColor(.accentColor)
                        }
                    }

                    ForEach(viewModel.domainFacets) { facet in
                        Button(action: {
                            if viewModel.selectedDomains.contains(facet.domain) {
                                viewModel.selectedDomains.remove(facet.domain)
                            } else {
                                viewModel.selectedDomains.insert(facet.domain)
                            }
                        }) {
                            HStack {
                                Image(systemName: viewModel.selectedDomains.contains(facet.domain) ? "checkmark.square.fill" : "square")
                                    .font(.caption)
                                    .foregroundColor(viewModel.selectedDomains.contains(facet.domain) ? .accentColor : .secondary)
                                Text(facet.domain)
                                    .font(.caption)
                                    .foregroundColor(.primary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                Spacer()
                                Text("\(facet.count)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
            .padding(12)
        }
    }
}

// MARK: - Filter Banner

private struct FilterBanner: View {
    @ObservedObject var viewModel: SearchViewModel

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: "line.3.horizontal.decrease.circle.fill")
                .foregroundColor(.accentColor)
                .font(.caption)

            if viewModel.selectedDateRange != .all {
                FilterChip(label: viewModel.selectedDateRange.rawValue) {
                    viewModel.selectedDateRange = .all
                }
            }

            ForEach(Array(viewModel.selectedDomains).sorted(), id: \.self) { domain in
                FilterChip(label: domain) {
                    viewModel.selectedDomains.remove(domain)
                }
            }

            Spacer()

            Button("Clear All") {
                viewModel.selectedDateRange = .all
                viewModel.selectedDomains.removeAll()
            }
            .font(.caption2)
            .buttonStyle(.plain)
            .foregroundColor(.accentColor)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(Color.accentColor.opacity(0.05))
    }
}

private struct FilterChip: View {
    let label: String
    let onRemove: () -> Void

    var body: some View {
        HStack(spacing: 2) {
            Text(label)
                .font(.caption2)
                .lineLimit(1)
            Button(action: onRemove) {
                Image(systemName: "xmark")
                    .font(.system(size: 8, weight: .bold))
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 6)
        .padding(.vertical, 2)
        .background(Color.accentColor.opacity(0.1))
        .cornerRadius(4)
    }
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
