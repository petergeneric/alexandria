import Foundation
import Combine

enum DateRangeFilter: String, CaseIterable, Identifiable {
    case all = "All Time"
    case today = "Today"
    case yesterday = "Yesterday"
    case thisWeek = "This Week"
    case thisMonth = "This Month"
    case thisYear = "This Year"

    var id: String { rawValue }

    func matches(_ date: Date?) -> Bool {
        guard let date = date else { return self == .all }
        let calendar = Calendar.current
        let now = Date()
        switch self {
        case .all: return true
        case .today: return calendar.isDateInToday(date)
        case .yesterday: return calendar.isDateInYesterday(date)
        case .thisWeek:
            return calendar.isDate(date, equalTo: now, toGranularity: .weekOfYear)
        case .thisMonth:
            return calendar.isDate(date, equalTo: now, toGranularity: .month)
        case .thisYear:
            return calendar.isDate(date, equalTo: now, toGranularity: .year)
        }
    }
}

enum SiteFacetMode: String, CaseIterable {
    case siteGroup = "Site"
    case domain = "Domain"
}

struct DomainFacet: Identifiable {
    let domain: String
    let count: Int
    var id: String { domain }
}

class SearchViewModel: ObservableObject {
    @Published var query: String = ""
    @Published var results: [SearchResult] = []
    @Published var isSearching = false
    @Published var selectedDateRange: DateRangeFilter = .all
    @Published var selectedDomains: Set<String> = []
    @Published var siteFacetMode: SiteFacetMode = .siteGroup
    @Published var pendingCount: UInt64 = 0
    @Published var pendingOldest: Date? = nil
    @Published var isIndexing = false

    private func facetKey(_ result: SearchResult) -> String {
        siteFacetMode == .siteGroup ? result.siteGroup : result.domain
    }

    var filteredResults: [SearchResult] {
        results.filter { result in
            let dateMatch = selectedDateRange.matches(result.visitedAt)
            let domainMatch = selectedDomains.isEmpty || selectedDomains.contains(facetKey(result))
            return dateMatch && domainMatch
        }
    }

    var domainFacets: [DomainFacet] {
        // Count by facet key from date-filtered results only
        let dateFiltered = results.filter { selectedDateRange.matches($0.visitedAt) }
        var counts: [String: Int] = [:]
        for r in dateFiltered {
            counts[facetKey(r), default: 0] += 1
        }
        return counts.map { DomainFacet(domain: $0.key, count: $0.value) }
            .sorted { $0.count > $1.count }
    }

    var dateRangeCounts: [DateRangeFilter: Int] {
        // Count results per date range (independent of date filter selection)
        let domainFiltered = selectedDomains.isEmpty
            ? results
            : results.filter { selectedDomains.contains(facetKey($0)) }
        var counts: [DateRangeFilter: Int] = [:]
        for filter in DateRangeFilter.allCases {
            counts[filter] = domainFiltered.filter { filter.matches($0.visitedAt) }.count
        }
        return counts
    }

    private(set) var lastSearchTime: Date?
    private let engine: SearchEngineWrapper?
    private let storePath: String
    private var debounceTask: Task<Void, Never>?

    init() {
        let indexPath = Self.resolveIndexPath()
        engine = SearchEngineWrapper(indexPath: indexPath, appDbPath: AppSettings.defaultAppDbPath)
        storePath = AppSettings.shared.storePath
        if engine == nil {
            print("Warning: could not open index at \(indexPath)")
        }

        // Debounced search on query change
        $query
            .removeDuplicates()
            .sink { [weak self] newQuery in
                self?.debounceTask?.cancel()
                guard !newQuery.isEmpty else {
                    self?.results = []
                    self?.selectedDateRange = .all
                    self?.selectedDomains.removeAll()
                    return
                }
                self?.debounceTask = Task { @MainActor [weak self] in
                    try? await Task.sleep(nanoseconds: 150_000_000) // 150ms
                    guard !Task.isCancelled else { return }
                    self?.performSearch()
                }
            }
            .store(in: &cancellables)

        // Clear domain selections when facet mode changes
        $siteFacetMode
            .dropFirst()
            .sink { [weak self] _ in
                self?.selectedDomains.removeAll()
            }
            .store(in: &cancellables)
    }

    private var cancellables = Set<AnyCancellable>()

    func performSearch() {
        guard let engine = engine, !query.isEmpty else {
            results = []
            return
        }

        isSearching = true
        lastSearchTime = Date()
        let q = query
        let sp = storePath

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let searchResults = engine.search(query: q, limit: 50, storePath: sp)
            let status = engine.pendingStatus(storePath: sp)
            DispatchQueue.main.async {
                guard let self = self, self.query == q else { return }
                self.results = searchResults
                self.pendingCount = status.count
                self.pendingOldest = status.oldestCapturedAt
                self.isSearching = false
            }
        }
    }

    func indexNow() {
        guard let engine = engine, !isIndexing else { return }
        isIndexing = true
        let sp = storePath

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let _ = engine.ingestFromStore(storePath: sp)
            let status = engine.pendingStatus(storePath: sp)
            DispatchQueue.main.async {
                guard let self = self else { return }
                self.pendingCount = status.count
                self.pendingOldest = status.oldestCapturedAt
                self.isIndexing = false
                // Re-run search to include newly indexed pages
                if !self.query.isEmpty {
                    self.performSearch()
                }
            }
        }
    }

    func clearIfStale() {
        guard let lastSearch = lastSearchTime else { return }
        if Date().timeIntervalSince(lastSearch) > 600 {
            query = ""
            results = []
            selectedDateRange = .all
            selectedDomains.removeAll()
        }
    }

    static func resolveIndexPath() -> String {
        let fm = FileManager.default
        let appSupport = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        let indexDir = appSupport.appendingPathComponent("works.peter.alexandria/index")
        let path = indexDir.path

        if !fm.fileExists(atPath: path) {
            try? fm.createDirectory(at: indexDir, withIntermediateDirectories: true)
        }

        return path
    }
}
