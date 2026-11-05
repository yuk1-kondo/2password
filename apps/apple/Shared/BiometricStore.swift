import Foundation
import LocalAuthentication
import Security

/// Stores the AUK behind device-owner authentication (spec §8.2).
/// The Keychain item carries a `SecAccessControl` with `.userPresence`, so
/// reading it automatically prompts Touch ID (or the login password as
/// fallback). The AUK itself never touches disk in plain form.
enum BiometricStore {
    private static let service = "dev.twopassword.auk"
    private static let account = "main"
    private static let enrolledKey = "dev.twopassword.aukStored"

    /// Hardware biometry (Touch ID / Face ID) present on this device.
    static var hardwareAvailable: Bool {
        var error: NSError?
        return LAContext().canEvaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics, error: &error
        )
    }

    /// An AUK has been wrapped into the keychain.
    static var isEnrolled: Bool {
        UserDefaults.standard.bool(forKey: enrolledKey)
    }

    static func save(_ aukB64: String) throws {
        var cfError: Unmanaged<CFError>?
        guard let access = SecAccessControlCreateWithFlags(
            nil,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            .userPresence,
            &cfError
        ) else {
            throw KeychainError.unexpectedStatus(errSecParam)
        }
        var query = baseQuery
        SecItemDelete(query as CFDictionary)
        query[kSecValueData as String] = Data(aukB64.utf8)
        query[kSecAttrAccessControl as String] = access
        let status = SecItemAdd(query as CFDictionary, nil)
        guard status == errSecSuccess else {
            throw KeychainError.unexpectedStatus(status)
        }
        UserDefaults.standard.set(true, forKey: enrolledKey)
    }

    /// Reading the item triggers the biometric prompt automatically.
    static func load() throws -> String {
        var query = baseQuery
        query[kSecReturnData as String] = true
        query[kSecUseOperationPrompt as String] = "2Passwordのロックを解除"
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess,
              let data = item as? Data,
              let auk = String(data: data, encoding: .utf8)
        else {
            throw KeychainError.unexpectedStatus(status)
        }
        return auk
    }

    static func delete() {
        SecItemDelete(baseQuery as CFDictionary)
        UserDefaults.standard.set(false, forKey: enrolledKey)
    }

    private static var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }
}
