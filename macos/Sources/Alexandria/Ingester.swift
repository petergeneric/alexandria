import Combine
import Foundation
import IOKit.ps

class Ingester: ObservableObject {
    private static let lowBatteryThreshold = 20

    @Published private(set) var isRunning = false

    private let engine: SearchEngineWrapper
    private let settings: AppSettings
    private var timer: Timer?
    private var isLowPowerMode: Bool
    private var powerSourceRunLoopSource: CFRunLoopSource?

    init(engine: SearchEngineWrapper, settings: AppSettings = .shared) {
        self.engine = engine
        self.settings = settings
        self.isLowPowerMode = ProcessInfo.processInfo.isLowPowerModeEnabled
        scheduleHourlyTimer()

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(powerStateDidChange),
            name: NSNotification.Name.NSProcessInfoPowerStateDidChange,
            object: nil
        )

        installPowerSourceObserver()
    }

    deinit {
        timer?.invalidate()
        NotificationCenter.default.removeObserver(self)
        if let source = powerSourceRunLoopSource {
            CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .defaultMode)
            Unmanaged.passUnretained(self).release()
        }
    }

    private func installPowerSourceObserver() {
        let context = Unmanaged.passRetained(self).toOpaque()
        guard let source = IOPSNotificationCreateRunLoopSource({ context in
            guard let context = context else { return }
            let ingester = Unmanaged<Ingester>.fromOpaque(context).takeUnretainedValue()
            ingester.handlePowerSourceChange()
        }, context)?.takeRetainedValue() else {
            Unmanaged<Ingester>.fromOpaque(context).release()
            return
        }
        powerSourceRunLoopSource = source
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .defaultMode)
    }

    private func handlePowerSourceChange() {
        let battery = batteryState()
        if let level = battery.level {
            if level <= Self.lowBatteryThreshold {
                print("Battery at \(level)% — pausing automatic indexing")
            } else if !battery.onBattery {
                // Plugged back in — try to catch up
                ingestIfNeeded()
            }
        }
    }

    @objc private func powerStateDidChange() {
        isLowPowerMode = ProcessInfo.processInfo.isLowPowerModeEnabled
        if isLowPowerMode {
            print("Low Power Mode enabled — pausing automatic indexing")
        } else {
            print("Low Power Mode disabled — resuming automatic indexing")
            ingestIfNeeded()
        }
    }

    private struct BatteryState {
        var onBattery: Bool
        var level: Int? // 0–100, nil if no battery
    }

    private func batteryState() -> BatteryState {
        guard let snapshot = IOPSCopyPowerSourcesInfo()?.takeRetainedValue(),
              let sources = IOPSCopyPowerSourcesList(snapshot)?.takeRetainedValue() as? [Any],
              let first = sources.first,
              let desc = IOPSGetPowerSourceDescription(snapshot, first as CFTypeRef)?
                  .takeUnretainedValue() as? [String: Any] else {
            return BatteryState(onBattery: false, level: nil)
        }
        let onBattery = (desc[kIOPSPowerSourceStateKey] as? String) == kIOPSBatteryPowerValue
        let level = desc[kIOPSCurrentCapacityKey] as? Int
        return BatteryState(onBattery: onBattery, level: level)
    }

    /// Run ingestion from all configured sources. Safe to call frequently — skips if already running.
    func ingestIfNeeded() {
        guard !isLowPowerMode else { return }

        let battery = batteryState()
        if battery.onBattery {
            if !settings.indexOnBattery { return }
            if let level = battery.level, level <= Self.lowBatteryThreshold { return }
        }

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

            DispatchQueue.main.async {
                self.isRunning = false
            }
        }
    }

    private func scheduleHourlyTimer() {
        timer = Timer.scheduledTimer(withTimeInterval: 3600, repeats: true) { [weak self] _ in
            self?.ingestIfNeeded()
        }
    }
}
