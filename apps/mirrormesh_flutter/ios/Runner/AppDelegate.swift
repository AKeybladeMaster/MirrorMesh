import Flutter
import Security
import UIKit

private let pairingKeychainService = "org.mirrormesh.pairing-identity.v1"
private let pairingKeychainAccount = "ed25519-seed"
private let iosPairingChannelName = "com.mirrormesh/ios_pairing"

private enum PairingRuntimeBootstrapError: Error {
  case unavailable(String)

  var message: String {
    switch self {
    case .unavailable(let message): return message
    }
  }
}

@_cdecl("mirrormesh_ios_keychain_read_seed")
func mirrormeshIOSKeychainReadSeed(
  _ output: UnsafeMutablePointer<UInt8>?,
  _ capacity: UInt
) -> Int32 {
  guard let output, capacity >= 32 else { return -2 }
  output.update(repeating: 0, count: 32)

  let query: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: pairingKeychainService,
    kSecAttrAccount as String: pairingKeychainAccount,
    kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
    kSecReturnData as String: true,
    kSecMatchLimit as String: kSecMatchLimitOne,
  ]
  var item: CFTypeRef?
  let status = SecItemCopyMatching(query as CFDictionary, &item)
  if status == errSecItemNotFound { return 1 }
  guard status == errSecSuccess, let secret = item as? Data, secret.count == 32 else {
    return -2
  }
  secret.withUnsafeBytes { source in
    guard let baseAddress = source.baseAddress else { return }
    output.update(from: baseAddress.assumingMemoryBound(to: UInt8.self), count: 32)
  }
  return 0
}

@_cdecl("mirrormesh_ios_keychain_write_seed")
func mirrormeshIOSKeychainWriteSeed(
  _ seed: UnsafePointer<UInt8>?,
  _ length: UInt
) -> Int32 {
  guard let seed, length == 32 else { return -2 }

  let secret = NSMutableData(length: 32)!
  secret.mutableBytes.copyMemory(from: UnsafeRawPointer(seed), byteCount: 32)
  defer { secret.resetBytes(in: NSRange(location: 0, length: secret.length)) }

  let identityQuery: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: pairingKeychainService,
    kSecAttrAccount as String: pairingKeychainAccount,
    kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
  ]
  var addQuery = identityQuery
  addQuery[kSecValueData as String] = secret
  addQuery[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
  let addStatus = SecItemAdd(addQuery as CFDictionary, nil)
  if addStatus == errSecSuccess { return 0 }
  guard addStatus == errSecDuplicateItem else { return -1 }

  let updateStatus = SecItemUpdate(
    identityQuery as CFDictionary,
    [kSecValueData as String: secret] as CFDictionary
  )
  return updateStatus == errSecSuccess ? 0 : -1
}

@_cdecl("mirrormesh_ios_keychain_delete_seed")
func mirrormeshIOSKeychainDeleteSeed() -> Int32 {
  let query: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: pairingKeychainService,
    kSecAttrAccount as String: pairingKeychainAccount,
    kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
  ]
  let status = SecItemDelete(query as CFDictionary)
  return status == errSecSuccess || status == errSecItemNotFound ? 0 : -1
}

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  static var pendingInvitation: String?
  static func acceptInvitation(_ url: URL) {
    guard url.scheme == "mirrormesh", ["p", "pair"].contains(url.host ?? ""),
          url.absoluteString.utf8.count <= 1024 else { return }
    pendingInvitation = url.absoluteString
  }

  override func application(_ application: UIApplication, open url: URL,
      options: [UIApplication.OpenURLOptionsKey: Any] = [:]) -> Bool {
    Self.acceptInvitation(url)
    return url.scheme == "mirrormesh" || super.application(application, open: url, options: options)
  }
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    let pairingChannel = FlutterMethodChannel(
      name: iosPairingChannelName,
      binaryMessenger: engineBridge.applicationRegistrar.messenger()
    )
    pairingChannel.setMethodCallHandler { call, result in
      if call.method == "hasBroadcastExtension" {
        let identifier = Bundle.main.object(forInfoDictionaryKey: "RTCScreenSharingExtension") as? String
        let plugins = Bundle.main.builtInPlugInsURL.flatMap {
          try? FileManager.default.contentsOfDirectory(at: $0, includingPropertiesForKeys: nil)
        } ?? []
        result(plugins.contains { Bundle(url: $0)?.bundleIdentifier == identifier && identifier != nil })
        return
      }
      if call.method == "takeInvitation" {
        result(Self.pendingInvitation)
        Self.pendingInvitation = nil
        return
      }
      guard call.method == "preparePairingRuntime" else {
        result(FlutterMethodNotImplemented)
        return
      }
      DispatchQueue.global(qos: .userInitiated).async {
        let response = Self.preparePairingRuntime()
        DispatchQueue.main.async {
          switch response {
          case .success(let deviceID): result(deviceID)
          case .failure(let error):
            result(FlutterError(
              code: "pairing_runtime_unavailable",
              message: error.message,
              details: nil
            ))
          }
        }
      }
    }
  }

  private static func preparePairingRuntime() -> Result<String, PairingRuntimeBootstrapError> {
    #if targetEnvironment(simulator)
    return .failure(.unavailable("Local pairing support requires an iOS device build."))
    #else
    do {
      let supportDirectory = try FileManager.default.url(
        for: .applicationSupportDirectory,
        in: .userDomainMask,
        appropriateFor: nil,
        create: true
      )
      try FileManager.default.createDirectory(
        at: supportDirectory,
        withIntermediateDirectories: true
      )
      var resourceValues = URLResourceValues()
      resourceValues.isExcludedFromBackup = true
      var protectedDirectory = supportDirectory
      try protectedDirectory.setResourceValues(resourceValues)

      let databaseURL = supportDirectory.appendingPathComponent(
        "mirrormesh-trusted-peers-v1.sqlite",
        isDirectory: false
      )
      var deviceIDBytes = [UInt8](repeating: 0, count: 16)
      let status = databaseURL.path.withCString { databasePath in
        mirrormesh_ios_prepare_pairing_runtime(
          databasePath,
          &deviceIDBytes,
          deviceIDBytes.count
        )
      }
      guard status == 0 else {
        return .failure(.unavailable("iOS could not open local pairing support (code \(status))."))
      }
      let uuid = UUID(uuid: (
        deviceIDBytes[0], deviceIDBytes[1], deviceIDBytes[2], deviceIDBytes[3],
        deviceIDBytes[4], deviceIDBytes[5], deviceIDBytes[6], deviceIDBytes[7],
        deviceIDBytes[8], deviceIDBytes[9], deviceIDBytes[10], deviceIDBytes[11],
        deviceIDBytes[12], deviceIDBytes[13], deviceIDBytes[14], deviceIDBytes[15]
      ))
      return .success(uuid.uuidString.lowercased())
    } catch {
      return .failure(.unavailable("iOS could not prepare local pairing support."))
    }
    #endif
  }
}
