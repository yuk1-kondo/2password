import Foundation

/// Unix-domain-socket bridge for the browser native-messaging host
/// (spec §8.3). Listens only while the vault is unlocked; the socket lives
/// at `~/Library/Application Support/2password/app.sock`.
///
/// Protocol: one JSON object per line, request and reply.
final class AppBridge {
    private let socketPath: String
    private let store: VaultStore
    private var listenerFD: Int32 = -1
    private var running = false
    private let queue = DispatchQueue(label: "dev.twopassword.bridge", qos: .utility)

    init(store: VaultStore, socketDir: URL) {
        self.store = store
        self.socketPath = socketDir.appendingPathComponent("app.sock").path
    }

    func start() {
        guard !running else { return }
        running = true
        queue.async { self.listen() }
    }

    func stop() {
        running = false
        if listenerFD >= 0 {
            Darwin.close(listenerFD)
            listenerFD = -1
        }
        unlink(socketPath)
    }

    private func listen() {
        guard listenerFD < 0 else { return }
        try? FileManager.default.createDirectory(
            atPath: (socketPath as NSString).deletingLastPathComponent,
            withIntermediateDirectories: true
        )
        unlink(socketPath)

        listenerFD = socket(AF_UNIX, SOCK_STREAM, 0)
        guard listenerFD >= 0 else { return }

        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)
        addr.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
        _ = withUnsafeMutablePointer(to: &addr.sun_path) {
            $0.withMemoryRebound(to: CChar.self, capacity: 104) { dest in
                socketPath.withCString { strncpy(dest, $0, 104) }
            }
        }
        let bound = withUnsafePointer(to: &addr) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                Darwin.bind(listenerFD, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
            }
        }
        guard bound == 0, Darwin.listen(listenerFD, 8) == 0 else {
            Darwin.close(listenerFD)
            listenerFD = -1
            return
        }
        // Socket file must be owner-only (it can serve decrypted secrets).
        chmod(socketPath, 0o600)

        while running {
            let client = Darwin.accept(listenerFD, nil, nil)
            guard client >= 0 else { continue }
            Task { await self.serve(client) }
        }
    }

    private func serve(_ fd: Int32) async {
        defer { Darwin.close(fd) }
        var data = Data()
        var buf = [UInt8](repeating: 0, count: 4096)
        while true {
            let n = Darwin.read(fd, &buf, buf.count)
            guard n > 0 else { return }
            data.append(contentsOf: buf[0..<n])
            guard let nl = data.firstIndex(of: 0x0A) else { continue }
            let line = data[..<nl]
            guard let req = try? JSONSerialization.jsonObject(with: line) as? [String: Any] else { return }
            let resp = await store.bridgeHandle(req)
            guard let out = try? JSONSerialization.data(withJSONObject: resp)
            else { return }
            var payload = out
            payload.append(0x0A)
            payload.withUnsafeBytes { bytes in
                _ = Darwin.write(fd, bytes.baseAddress, bytes.count)
            }
            return
        }
    }
}
