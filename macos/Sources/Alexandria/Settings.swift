import Foundation

class AppSettings: ObservableObject {
    static let shared = AppSettings()

    static let defaultStorePath: String = {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        return appSupport.appendingPathComponent("works.peter.alexandria/pages.db").path
    }()

    private enum Keys {
        static let storePath = "storePath"
        static let indexOnBattery = "indexOnBattery"
    }

    @Published var storePath: String {
        didSet { UserDefaults.standard.set(storePath, forKey: Keys.storePath) }
    }

    @Published var indexOnBattery: Bool {
        didSet { UserDefaults.standard.set(indexOnBattery, forKey: Keys.indexOnBattery) }
    }

    init() {
        self.storePath = UserDefaults.standard.string(forKey: Keys.storePath) ?? Self.defaultStorePath
        self.indexOnBattery = UserDefaults.standard.bool(forKey: Keys.indexOnBattery) // false by default
    }
}
