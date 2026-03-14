// swift-tools-version: 5.9
import PackageDescription
import Foundation

// Resolve the workspace root relative to this Package.swift
let packageDir = URL(fileURLWithPath: #filePath).deletingLastPathComponent().path
let workspaceRoot = packageDir + "/.."

let package = Package(
    name: "Alexandria",
    platforms: [.macOS(.v13)],
    targets: [
        .systemLibrary(
            name: "alexandria_coreFFI",
            pkgConfig: nil
        ),
        .executableTarget(
            name: "Alexandria",
            dependencies: ["alexandria_coreFFI"],
            exclude: ["Info.plist"],
            resources: [.copy("Resources/icon.svg")],
            linkerSettings: [
                .unsafeFlags([
                    "-L\(workspaceRoot)/target/release",
                    "-L\(workspaceRoot)/target/debug",
                ]),
                .linkedLibrary("alexandria_core"),
                .linkedFramework("Security"),
                .linkedFramework("SystemConfiguration"),
            ]
        ),
    ]
)
