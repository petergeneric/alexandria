import Foundation

class AppSettings: ObservableObject {
    static let shared = AppSettings()

    static let defaultStorePath: String = {
        let appSupport = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
        return appSupport.appendingPathComponent("works.peter.alexandria/pages.db").path
    }()

    private enum Keys {
        static let webcachePath = "webcachePath"
        static let storePath = "storePath"
    }

    @Published var webcachePath: String {
        didSet { UserDefaults.standard.set(webcachePath, forKey: Keys.webcachePath) }
    }

    @Published var storePath: String {
        didSet { UserDefaults.standard.set(storePath, forKey: Keys.storePath) }
    }

    init() {
        self.webcachePath = UserDefaults.standard.string(forKey: Keys.webcachePath) ?? ""
        self.storePath = UserDefaults.standard.string(forKey: Keys.storePath) ?? Self.defaultStorePath
    }
}
