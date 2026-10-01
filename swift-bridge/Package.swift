// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "NetworkFrameworkBridge",
    platforms: [
        .macOS(.v14),
        .iOS("26.0"),
    ],
    products: [
        .library(
            name: "NetworkFrameworkBridge",
            type: .static,
            targets: ["NetworkFrameworkBridge"])
    ],
    targets: [
        .target(
            name: "NetworkFrameworkCShim",
            path: "Sources/NetworkFrameworkCShim",
            publicHeadersPath: "include"),
        .target(
            name: "NetworkFrameworkBridge",
            dependencies: ["NetworkFrameworkCShim"],
            path: "Sources/NetworkFrameworkBridge")
    ]
)
