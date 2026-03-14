import Foundation

struct SearchResult: Identifiable {
    let id = UUID()
    let url: String
    let title: String
    let snippet: String
    let domain: String
    let score: Float
}

class SearchEngineWrapper {
    private let engine: AlexandriaEngine

    init?(indexPath: String) {
        guard let eng = try? AlexandriaEngine.open(indexPath: indexPath) else {
            return nil
        }
        engine = eng
    }

    func ingest(sourceDir: String) -> Int {
        guard let count = try? engine.ingest(sourceDir: sourceDir) else {
            return 0
        }
        return Int(count)
    }

    func search(query: String, limit: Int = 20, offset: Int = 0) -> [SearchResult] {
        guard !query.isEmpty else { return [] }

        guard let results = try? engine.search(
            query: query,
            limit: UInt32(limit),
            offset: UInt32(offset)
        ) else {
            return []
        }

        return results.map { r in
            SearchResult(
                url: r.url,
                title: r.title,
                snippet: r.contentSnippet,
                domain: r.domain,
                score: r.score
            )
        }
    }
}
