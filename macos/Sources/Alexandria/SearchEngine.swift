import Foundation

struct SearchResult: Identifiable {
    let id = UUID()
    let url: String
    let title: String
    let snippet: String
    let domain: String
    let score: Float
    let visitedAt: Date?
}

class SearchEngineWrapper {
    private let engine: AlexandriaEngine

    init?(indexPath: String) {
        guard let eng = try? AlexandriaEngine.open(indexPath: indexPath) else {
            return nil
        }
        engine = eng
    }

    func ingestFromStore(storePath: String) -> Int {
        guard let count = try? engine.ingestFromStore(storePath: storePath) else {
            return 0
        }
        return Int(count)
    }

    func pendingStatus(storePath: String) -> (count: UInt64, oldestCapturedAt: Date?) {
        guard let status = try? engine.pendingStatus(storePath: storePath) else {
            return (0, nil)
        }
        let date = status.oldestCapturedAtSecs.map { Date(timeIntervalSince1970: TimeInterval($0)) }
        return (status.count, date)
    }

    func docCount() -> UInt64 {
        guard let count = try? engine.docCount() else { return 0 }
        return count
    }

    func deleteHistory(storePath: String) -> Bool {
        guard let _ = try? engine.deleteHistory(storePath: storePath) else { return false }
        return true
    }

    func reindex(storePath: String) -> Int {
        guard let count = try? engine.reindex(storePath: storePath) else { return 0 }
        return Int(count)
    }

    func search(query: String, limit: Int = 20, offset: Int = 0, storePath: String = "") -> [SearchResult] {
        guard !query.isEmpty else { return [] }

        guard let results = try? engine.search(
            query: query,
            limit: UInt32(limit),
            offset: UInt32(offset),
            storePath: storePath
        ) else {
            return []
        }

        return results.map { r in
            SearchResult(
                url: r.url,
                title: r.title,
                snippet: r.contentSnippet,
                domain: r.domain,
                score: r.score,
                visitedAt: r.visitedAtSecs.map { Date(timeIntervalSince1970: TimeInterval($0)) }
            )
        }
    }
}
