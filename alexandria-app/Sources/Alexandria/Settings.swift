import Foundation

class AppSettings: ObservableObject {
    static let shared = AppSettings()

    private enum Keys {
        static let webcachePath = "webcachePath"
    }

    @Published var webcachePath: String {
        didSet { UserDefaults.standard.set(webcachePath, forKey: Keys.webcachePath) }
    }

    init() {
        self.webcachePath = UserDefaults.standard.string(forKey: Keys.webcachePath) ?? ""
    }
}
