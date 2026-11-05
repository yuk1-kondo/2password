import Foundation
import Security

enum KeychainError: Error, LocalizedError {
    case unexpectedStatus(OSStatus)

    var errorDescription: String? {
        switch self {
        case .unexpectedStatus(let status):
            return "Keychain error: \(status)"
        }
    }
}

/// Secret Key storage (spec §8.2). Device-only, never synced via iCloud
/// Keychain — losing the device means restoring from the emergency kit.
enum KeychainHelper {
    private static let service = "dev.twopassword.secretkey"
    private static let account = "main"

    private static var baseQuery: [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
    }

    static func save(_ secretKey: String) throws {
        var status = SecItemUpdate(
            baseQuery as CFDictionary,
            [kSecValueData as String: Data(secretKey.utf8)] as CFDictionary
        )
        if status == errSecItemNotFound {
            var attrs = baseQuery
            attrs[kSecValueData as String] = Data(secretKey.utf8)
            attrs[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
            status = SecItemAdd(attrs as CFDictionary, nil)
        }
        guard status == errSecSuccess else {
            throw KeychainError.unexpectedStatus(status)
        }
    }

    static func load() throws -> String {
        var query = baseQuery
        query[kSecReturnData as String] = true
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        guard status == errSecSuccess,
              let data = item as? Data,
              let key = String(data: data, encoding: .utf8)
        else {
            throw KeychainError.unexpectedStatus(status)
        }
        return key
    }
}
