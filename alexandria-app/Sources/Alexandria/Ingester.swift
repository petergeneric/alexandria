import Foundation

class Ingester {
    private let engine: SearchEngineWrapper
    private let settings: AppSettings
    private var timer: Timer?
    private var isRunning = false

    init(engine: SearchEngineWrapper, settings: AppSettings = .shared) {
        self.engine = engine
        self.settings = settings
        scheduleHourlyTimer()
    }

    deinit {
        timer?.invalidate()
    }

    /// Run ingestion from all configured sources. Safe to call frequently — skips if already running.
    func ingestIfNeeded() {
        guard !isRunning else { return }

        isRunning = true
        DispatchQueue.global(qos: .utility).async { [weak self] in
            guard let self = self else { return }

            // Ingest from page store (browser extension captures)
            let storePath = self.settings.storePath
            if !storePath.isEmpty {
                let storeCount = self.engine.ingestFromStore(storePath: storePath)
                if storeCount > 0 {
                    print("Ingested \(storeCount) new pages from store")
                }
            }

            // Ingest from Recoll webcache (legacy)
            let webcachePath = self.settings.webcachePath
            if !webcachePath.isEmpty {
                let count = self.engine.ingest(sourceDir: webcachePath)
                if count > 0 {
                    print("Ingested \(count) new pages from \(webcachePath)")
                }
            }

            self.isRunning = false
        }
    }

    private func scheduleHourlyTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 3600, repeats: true) { [weak self] _ in
            self?.ingestIfNeeded()
        }
    }
}
