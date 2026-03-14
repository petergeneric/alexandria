import Foundation
import Combine

class SearchViewModel: ObservableObject {
    @Published var query: String = ""
    @Published var results: [SearchResult] = []
    @Published var isSearching = false

    private let engine: SearchEngineWrapper?
    private var debounceTask: Task<Void, Never>?

    init() {
        let indexPath = Self.resolveIndexPath()
        engine = SearchEngineWrapper(indexPath: indexPath)
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
                    return
                }
                self?.debounceTask = Task { @MainActor [weak self] in
                    try? await Task.sleep(nanoseconds: 150_000_000) // 150ms
                    guard !Task.isCancelled else { return }
                    self?.performSearch()
                }
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
        let q = query

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let searchResults = engine.search(query: q, limit: 20)
            DispatchQueue.main.async {
                guard let self = self, self.query == q else { return }
                self.results = searchResults
                self.isSearching = false
            }
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
