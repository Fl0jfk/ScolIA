// swift-tools-version:5.9
// ScolIA — appli iOS unique (D-003). Le cœur (`ScolIACore`) est en Foundation pur :
// il compile et se teste aussi sous Linux (`swift test`). L'interface (`ScolIAUI`,
// SwiftUI + AuthenticationServices + LocalAuthentication) ne compile que pour iOS.
// L'appli elle-même (`App/`) est générée par XcodeGen depuis `project.yml`.
import PackageDescription

let package = Package(
    name: "ScolIA",
    defaultLocalization: "fr",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "ScolIACore", targets: ["ScolIACore"]),
        .library(name: "ScolIAUI", targets: ["ScolIAUI"]),
    ],
    targets: [
        .target(name: "ScolIACore"),
        .target(name: "ScolIAUI", dependencies: ["ScolIACore"]),
        .testTarget(name: "ScolIACoreTests", dependencies: ["ScolIACore"]),
    ]
)
