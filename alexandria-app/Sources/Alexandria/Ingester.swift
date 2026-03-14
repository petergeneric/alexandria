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

    /// Run ingestion if a webcache path is configured. Safe to call frequently — skips if already running.
    func ingestIfNeeded() {
        let path = settings.webcachePath
        guard !path.isEmpty, !isRunning else { return }

        isRunning = true
        DispatchQueue.global(qos: .utility).async { [weak self] in
            guard let self = self else { return }
            let count = self.engine.ingest(sourceDir: path)
            if count > 0 {
                print("Ingested \(count) new pages from \(path)")
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
