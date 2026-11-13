import Foundation

#if os(macOS)
import AppKit
#endif

enum AppError: Error, LocalizedError {
    case locked

    var errorDescription: String? {
        switch self {
        case .locked: return "Vaultがロックされています"
        }
    }
}

enum VaultLocation {
    /// `~/Library/Mobile Documents/com~apple~CloudDocs/2password/vault` —
    /// the user's visible iCloud Drive folder (no entitlement required).
    case iCloud
    /// Application Support fallback when iCloud Drive is unavailable.
    case local
}

/// Global app state. Owns the unlocked `Vault` handle; dropping it on lock
/// releases the keyset, which is zeroized by the Rust core (spec §8.2).
@Observable
@MainActor
final class VaultStore {
    enum State {
        /// No vault exists yet.
        case setup
        /// Vault exists on disk; password + Secret Key required.
        case locked
        case unlocked
    }

    /// Idle auto-lock delay (spec §8.2). Reset on unlock; user-activity
    /// resetting is a follow-up.
    static let autoLockDelay: Duration = .seconds(600)

    private(set) var state: State
    private(set) var items: [ItemSummary] = []
    private(set) var location: VaultLocation

    private var vault: Vault?
    private var autoLockTask: Task<Void, Never>?
    private var refreshTask: Task<Void, Never>?
    private var bridge: AppBridge?
    private let vaultDir: URL
    /// Socket dir always outside iCloud (spec §8.3 — never sync a socket).
    private let bridgeDir: URL

    /// Touch ID / Face ID can be offered.
    var biometricsAvailable: Bool {
        BiometricStore.hardwareAvailable && BiometricStore.isEnrolled
    }

    var syncDescription: String {
        switch location {
        case .iCloud: return "iCloud Drive"
        case .local: return "ローカルのみ"
        }
    }

    init() {
        let fm = FileManager.default
        let localVault = fm
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("2password/vault", isDirectory: true)
        var dir = localVault
        var loc: VaultLocation = .local
        // Bridge socket always lives in Application Support — never inside
        // the synced vault dir (spec §8.3).
        bridgeDir = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("2password", isDirectory: true)
        #if os(macOS)
        // iCloud Drive path access exists only on macOS. On iOS the same
        // folder needs a ubiquity-container entitlement (paid account), so
        // iOS stays on the local vault for now.
        let cloudDocs = fm.homeDirectoryForCurrentUser.appendingPathComponent(
            "Library/Mobile Documents/com~apple~CloudDocs", isDirectory: true
        )
        let cloudVault = cloudDocs.appendingPathComponent(
            "2password/vault", isDirectory: true
        )
        let localHasVault = fm.fileExists(
            atPath: localVault.appendingPathComponent("profile.json").path
        )
        let cloudHasVault = fm.fileExists(
            atPath: cloudVault.appendingPathComponent("profile.json").path
        )
        if fm.fileExists(atPath: cloudDocs.path) {
            if localHasVault && !cloudHasVault {
                // One-time migration of an existing local vault (spec §7).
                do {
                    try fm.createDirectory(
                        at: cloudVault.deletingLastPathComponent(),
                        withIntermediateDirectories: true
                    )
                    try fm.moveItem(at: localVault, to: cloudVault)
                } catch {
                    // Stay local rather than orphan the existing vault.
                    dir = localVault
                    loc = .local
                    vaultDir = dir
                    location = loc
                    state = .locked
                    registerScreenLockObserver()
                    return
                }
            }
            dir = cloudVault
            loc = .iCloud
        }
        #endif
        vaultDir = dir
        location = loc
        state = Self.isUsableVault(vaultDir) ? .locked : .setup
        registerScreenLockObserver()
    }

    /// A vault can be unlocked only if both manifest files exist; a
    /// half-written vault (setup interrupted) can't open, so treat it as
    /// needing (re)setup rather than a permanently locked door.
    private static func isUsableVault(_ dir: URL) -> Bool {
        let fm = FileManager.default
        return fm.fileExists(atPath: dir.appendingPathComponent("profile.json").path)
            && fm.fileExists(atPath: dir.appendingPathComponent("keyset.json").path)
    }

    private func registerScreenLockObserver() {
        #if os(macOS)
        DistributedNotificationCenter.default().addObserver(
            forName: Notification.Name("com.apple.screenIsLocked"),
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.lock()
        }
        #endif
    }

    /// Create a new vault. Returns the generated Secret Key for one-time
    /// display so the user can back it up (spec §7).
    ///
    /// Order matters: the Secret Key is persisted to the keychain *before*
    /// the vault is created. A failure in `createVault` is then safe to
    /// retry — the key is already stored.
    @discardableResult
    func setup(userId: String, password: String) throws -> String {
        let secretKey = newSecretKey()
        try KeychainHelper.save(secretKey)
        try FileManager.default.createDirectory(at: vaultDir, withIntermediateDirectories: true)
        try createVault(
            dir: vaultDir.path, userId: userId,
            password: password, secretKey: secretKey
        )
        vault = try unlockVault(dir: vaultDir.path, password: password, secretKey: secretKey)
        wrapAukIfPossible(password: password, secretKey: secretKey)
        state = .unlocked
        reload()
        scheduleAutoLock()
        return secretKey
    }

    /// Argon2id stretch runs off the main actor (spec §3.2, ~1s on target).
    /// Restore path for a device that lost its Secret Key (spec §7 —
    /// Emergency Kit entry point). Only persists the key after a
    /// successful unlock so a typo can't poison the Keychain.
    nonisolated func unlockWithSecretKey(password: String, secretKey: String) async throws {
        let path = await MainActor.run { vaultDir.path }
        let unlocked = try await Task.detached(priority: .userInitiated) {
            try unlockVault(dir: path, password: password, secretKey: secretKey)
        }.value
        try KeychainHelper.save(secretKey)
        let auk = try? await Task.detached(priority: .userInitiated) {
            try deriveAukB64(dir: path, password: password, secretKey: secretKey)
        }.value
        if let auk {
            try? BiometricStore.save(auk)
        }
        await finishUnlock(unlocked)
    }

    nonisolated func unlock(password: String) async throws {
        let secretKey = try KeychainHelper.load()
        let path = await MainActor.run { vaultDir.path }
        let unlocked = try await Task.detached(priority: .userInitiated) {
            try unlockVault(dir: path, password: password, secretKey: secretKey)
        }.value
        // Best-effort: wrap the AUK behind biometry for next time (spec §8.2).
        let auk = try? await Task.detached(priority: .userInitiated) {
            try deriveAukB64(dir: path, password: password, secretKey: secretKey)
        }.value
        if let auk {
            try? BiometricStore.save(auk)
        }
        await finishUnlock(unlocked)
    }

    /// If the vault directory vanished while we were running (files deleted,
    /// iCloud eviction), fall back to the setup flow instead of sitting on a
    /// permanently failing unlock screen.
    func noteUnlockFailure() {
        if !Self.isUsableVault(vaultDir) {
            lock()
            state = .setup
        }
    }

    /// Touch ID / Face ID path — reads the biometry-gated AUK (spec §8.2).
    nonisolated func unlockWithBiometrics() async throws {
        let auk = try BiometricStore.load()
        let path = await MainActor.run { vaultDir.path }
        let unlocked = try await Task.detached(priority: .userInitiated) {
            try unlockVaultWithAuk(dir: path, aukB64: auk)
        }.value
        await finishUnlock(unlocked)
    }

    private func finishUnlock(_ unlocked: Vault) {
        vault = unlocked
        state = .unlocked
        reload()
        scheduleAutoLock()
        // Serve browser autofill while unlocked (spec §8.3).
        bridge = AppBridge(store: self, socketDir: bridgeDir)
        bridge?.start()
    }

    private func wrapAukIfPossible(password: String, secretKey: String) {
        guard BiometricStore.hardwareAvailable else { return }
        if let auk = try? deriveAukB64(
            dir: vaultDir.path, password: password, secretKey: secretKey
        ) {
            try? BiometricStore.save(auk)
        }
    }

    private func scheduleAutoLock() {
        autoLockTask?.cancel()
        autoLockTask = Task { [weak self] in
            try? await Task.sleep(for: Self.autoLockDelay)
            guard !Task.isCancelled, let self else { return }
            self.lock()
        }
    }

    func lock() {
        autoLockTask?.cancel()
        autoLockTask = nil
        refreshTask?.cancel()
        refreshTask = nil
        bridge?.stop()
        bridge = nil
        vault = nil
        items = []
        state = .locked
    }

    /// Native-messaging bridge request handler (spec §8.3). Runs on the
    /// bridge's background tasks; marshals onto the main actor implicitly.
    func bridgeHandle(_ req: [String: Any]) -> [String: Any] {
        guard state == .unlocked, let vault else {
            return ["error": "2Password is locked"]
        }
        guard let type = req["type"] as? String else {
            return ["error": "bad request"]
        }
        switch type {
        case "findLogins":
            guard let host = req["url"] as? String else {
                return ["error": "bad url"]
            }
            do {
                let matched = try vault.listItems().filter {
                    domainMatches(itemUrl: $0.url, host: host)
                }
                return ["items": matched.map {
                    ["uuid": $0.uuid, "title": $0.title, "username": $0.username ?? ""]
                }]
            } catch {
                return ["error": "list failed"]
            }
        case "getPassword":
            guard let uuid = req["uuid"] as? String,
                  let host = req["url"] as? String else {
                return ["error": "bad request"]
            }
            do {
                let item = try vault.getItem(uuid: uuid)
                // Re-verify the domain before releasing the secret.
                guard domainMatches(itemUrl: item.url, host: host) else {
                    return ["error": "domain mismatch"]
                }
                return ["username": item.username ?? "", "password": item.password ?? ""]
            } catch {
                return ["error": "not found"]
            }
        default:
            return ["error": "unknown type"]
        }
    }

    func reload() {
        listNow()
        if location == .iCloud {
            ensureCloudItemsAvailable()
            startRefreshLoop()
        }
    }

    private func listNow() {
        items = (try? vault?.listItems()) ?? []
    }

    /// Trigger downloads for evicted iCloud items (".x.icloud" placeholders).
    private func ensureCloudItemsAvailable() {
        let fm = FileManager.default
        try? fm.startDownloadingUbiquitousItem(at: vaultDir)
        guard let enumerator = fm.enumerator(
            at: vaultDir, includingPropertiesForKeys: nil
        ) else { return }
        for case let url as URL in enumerator {
            guard url.pathExtension == "icloud",
                  var name = url.deletingPathExtension().lastPathComponent
                as String?,
                  name.hasPrefix(".")
            else { continue }
            name.removeFirst()
            try? fm.startDownloadingUbiquitousItem(
                at: url.deletingLastPathComponent().appendingPathComponent(name)
            )
        }
    }

    /// Poll briefly after unlock so items still downloading appear without a
    /// manual refresh.
    private func startRefreshLoop() {
        refreshTask?.cancel()
        refreshTask = Task { [weak self] in
            for _ in 0..<8 {
                try? await Task.sleep(for: .seconds(2))
                guard let self, !Task.isCancelled, self.state == .unlocked else { return }
                self.ensureCloudItemsAvailable()
                self.listNow()
            }
        }
    }

    func add(_ item: NewItem) throws {
        guard let vault else { throw AppError.locked }
        _ = try vault.addItem(item: item)
        reload()
    }

    func update(uuid: String, item: NewItem) throws {
        guard let vault else { throw AppError.locked }
        try vault.updateItem(uuid: uuid, item: item)
        reload()
    }

    func remove(uuid: String) throws {
        guard let vault else { throw AppError.locked }
        try vault.removeItem(uuid: uuid)
        reload()
    }

    func detail(uuid: String) throws -> ItemDetail {
        guard let vault else { throw AppError.locked }
        return try vault.getItem(uuid: uuid)
    }

    /// Returns number of items imported.
    @discardableResult
    func importCsv(url: URL) throws -> Int {
        guard let vault else { throw AppError.locked }
        let n = try Int(vault.importCsv(path: url.path))
        reload()
        return n
    }

    /// Returns number of items exported (plaintext CSV — spec §9).
    @discardableResult
    func exportCsv(url: URL) throws -> Int {
        guard let vault else { throw AppError.locked }
        return try Int(vault.exportCsv(path: url.path))
    }

    /// Verify `current`, then re-wrap the keyset under `new` (spec §5) and
    /// refresh the biometric-wrapped AUK.
    nonisolated func changePassword(current: String, new: String) async throws {
        let sk = try KeychainHelper.load()
        let path = await MainActor.run { vaultDir.path }
        // Verify the current password first (spec §5 note).
        _ = try await Task.detached(priority: .userInitiated) {
            try unlockVault(dir: path, password: current, secretKey: sk)
        }.value
        try await MainActor.run {
            guard let vault else { throw AppError.locked }
            try vault.changePassword(newPassword: new, secretKey: sk)
        }
        // Re-wrap AUK for biometrics (best effort).
        let auk = try? await Task.detached(priority: .userInitiated) {
            try deriveAukB64(dir: path, password: new, secretKey: sk)
        }.value
        if let auk {
            try? BiometricStore.save(auk)
        }
    }
}
