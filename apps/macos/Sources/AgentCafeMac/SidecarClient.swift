import Foundation

struct SidecarRunResult {
    let report: DiagnosticReport?
    let errorCode: String?
    let errorMessage: String?

    static func success(_ report: DiagnosticReport) -> SidecarRunResult {
        SidecarRunResult(report: report, errorCode: nil, errorMessage: nil)
    }

    static func failure(_ code: String, _ message: String) -> SidecarRunResult {
        SidecarRunResult(report: nil, errorCode: code, errorMessage: message)
    }
}

final class SidecarClient {
    private let sidecarPath: String
    private let decoder: JSONDecoder
    private var nextId = 0

    init(sidecarPath: String? = nil) {
        self.sidecarPath = sidecarPath
            ?? ProcessInfo.processInfo.environment["AGENTCAFE_SIDECAR"]
            ?? SidecarClient.defaultSidecarPath()
        self.decoder = JSONDecoder()
        self.decoder.dateDecodingStrategy = .iso8601
    }

    func runDoctor() async -> SidecarRunResult {
        if let fixture = ProcessInfo.processInfo.environment["AGENTCAFE_UI_FIXTURE"], !fixture.isEmpty {
            return await loadFixture(path: fixture)
        }

        guard FileManager.default.fileExists(atPath: sidecarPath) else {
            return .failure("sidecar_missing", "Sidecar not found at \(sidecarPath).")
        }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: sidecarPath)

        let input = Pipe()
        let output = Pipe()
        let error = Pipe()
        process.standardInput = input
        process.standardOutput = output
        process.standardError = error

        do {
            try process.run()
            try writeRequest(
                method: "ipc.handshake",
                params: [
                    "protocol_version": "1.0",
                    "ui_name": "agentcafe-macos-swiftui",
                    "ui_version": "0.1.0",
                    "ui_platform": "macos",
                    "ui_capabilities": [],
                    "nonce": UUID().uuidString
                ],
                pipe: input
            )

            let handshake = try readEnvelope(from: output)
            if let error = handshake.error {
                terminate(process)
                return .failure(error.data?.code ?? "handshake_failed", error.message)
            }

            try writeRequest(method: "doctor.run", params: [:], pipe: input)
            let doctor = try readEnvelope(from: output)
            input.fileHandleForWriting.closeFile()

            if let error = doctor.error {
                terminate(process)
                return .failure(error.data?.code ?? "doctor_failed", error.message)
            }

            guard let result = doctor.result else {
                terminate(process)
                return .failure("doctor_failed", "doctor.run returned no result.")
            }

            let report = try decoder.decode(DiagnosticReport.self, from: result)
            return .success(report)
        } catch {
            terminate(process)
            return .failure("sidecar_crash", error.localizedDescription)
        }
    }

    private func loadFixture(path: String) async -> SidecarRunResult {
        do {
            let data = try Data(contentsOf: URL(fileURLWithPath: path))
            let report = try decoder.decode(DiagnosticReport.self, from: data)
            return .success(report)
        } catch {
            return .failure("fixture_invalid", error.localizedDescription)
        }
    }

    private func writeRequest(method: String, params: [String: Any], pipe: Pipe) throws {
        nextId += 1
        let request: [String: Any] = [
            "jsonrpc": "2.0",
            "id": String(nextId),
            "method": method,
            "params": params
        ]
        let data = try JSONSerialization.data(withJSONObject: request)
        pipe.fileHandleForWriting.write(data)
        pipe.fileHandleForWriting.write(Data("\n".utf8))
    }

    private func readEnvelope(from pipe: Pipe) throws -> RpcEnvelope {
        guard let line = pipe.fileHandleForReading.readLine() else {
            throw SidecarClientError.closedStream
        }
        return try JSONDecoder().decode(RpcEnvelope.self, from: line)
    }

    private func terminate(_ process: Process) {
        if process.isRunning {
            process.terminate()
        }
    }

    private static func defaultSidecarPath() -> String {
        let root = findRepositoryRoot(start: FileManager.default.currentDirectoryPath)
        return URL(fileURLWithPath: root)
            .appendingPathComponent("target")
            .appendingPathComponent("debug")
            .appendingPathComponent("agentcafe-sidecar")
            .path
    }

    private static func findRepositoryRoot(start: String) -> String {
        var url = URL(fileURLWithPath: start)
        while url.path != "/" {
            let cargo = url.appendingPathComponent("Cargo.toml").path
            let core = url.appendingPathComponent("core").path
            if FileManager.default.fileExists(atPath: cargo)
                && FileManager.default.fileExists(atPath: core) {
                return url.path
            }
            url.deleteLastPathComponent()
        }
        return start
    }
}

private struct RpcEnvelope: Decodable {
    let result: Data?
    let error: RpcError?

    enum CodingKeys: String, CodingKey {
        case result
        case error
    }

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if container.contains(.result) {
            let object = try container.decode(AnyDecodable.self, forKey: .result)
            result = try JSONSerialization.data(withJSONObject: object.value)
        } else {
            result = nil
        }
        error = try container.decodeIfPresent(RpcError.self, forKey: .error)
    }
}

private struct RpcError: Decodable {
    let code: Int
    let message: String
    let data: RpcErrorData?
}

private struct RpcErrorData: Decodable {
    let code: String?
    let stage: String?
}

private struct AnyDecodable: Decodable {
    let value: Any

    init(from decoder: Decoder) throws {
        if let container = try? decoder.singleValueContainer() {
            if container.decodeNil() {
                value = NSNull()
            } else if let bool = try? container.decode(Bool.self) {
                value = bool
            } else if let int = try? container.decode(Int.self) {
                value = int
            } else if let double = try? container.decode(Double.self) {
                value = double
            } else if let string = try? container.decode(String.self) {
                value = string
            } else if let array = try? container.decode([AnyDecodable].self) {
                value = array.map(\.value)
            } else if let dictionary = try? container.decode([String: AnyDecodable].self) {
                value = dictionary.mapValues(\.value)
            } else {
                throw DecodingError.dataCorruptedError(
                    in: container,
                    debugDescription: "Unsupported JSON value."
                )
            }
        } else {
            throw SidecarClientError.invalidEnvelope
        }
    }
}

private enum SidecarClientError: Error {
    case closedStream
    case invalidEnvelope
}

private extension FileHandle {
    func readLine() -> Data? {
        var data = Data()
        while true {
            let byte = self.readData(ofLength: 1)
            if byte.isEmpty {
                return data.isEmpty ? nil : data
            }
            if byte == Data("\n".utf8) {
                return data
            }
            data.append(byte)
        }
    }
}
