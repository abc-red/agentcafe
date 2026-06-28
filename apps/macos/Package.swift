// swift-tools-version: 5.10
import PackageDescription

let package = Package(
    name: "AgentCafeMac",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(name: "AgentCafeMac", targets: ["AgentCafeMac"])
    ],
    targets: [
        .executableTarget(
            name: "AgentCafeMac",
            path: "Sources/AgentCafeMac"
        )
    ]
)
