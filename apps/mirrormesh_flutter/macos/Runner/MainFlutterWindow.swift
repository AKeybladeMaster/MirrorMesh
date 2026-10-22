import Cocoa
import FlutterMacOS
import Security
import Network
import Darwin

// Reuse the Apple adapter ABI; the desktop owns a separate Keychain namespace.
private let pairingKeychainService = "org.mirrormesh.desktop.pairing-identity.v1"
private let pairingKeychainAccount = "ed25519-seed"
@_silgen_name("mirrormesh_ios_prepare_pairing_runtime")
private func prepareAppleRuntime(_ path: UnsafePointer<CChar>, _ id: UnsafeMutablePointer<UInt8>, _ capacity: UInt) -> Int32

private final class DeviceContextQuery {
  private let result: FlutterResult
  private let monitor = NWPathMonitor()
  private var completed = false
  init(_ result: @escaping FlutterResult) { self.result = result }

  func start() {
    monitor.pathUpdateHandler = { [self] path in
      DispatchQueue.main.async { self.finish(path) }
    }
    monitor.start(queue: DispatchQueue(label: "com.mirrormesh.device-context"))
    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { self.finish(nil) }
  }

  private func finish(_ path: NWPath?) {
    guard !completed else { return }
    completed = true
    monitor.pathUpdateHandler = nil
    monitor.cancel()
    var size = 0
    sysctlbyname("hw.model", nil, &size, nil, 0)
    var modelBytes = [CChar](repeating: 0, count: max(1, min(size, 1024)))
    let model = modelBytes.withUnsafeMutableBufferPointer { buffer -> String in
      var count = buffer.count
      guard sysctlbyname("hw.model", buffer.baseAddress, &count, nil, 0) == 0 else { return "Mac" }
      return String(cString: buffer.baseAddress!)
    }
    var address = ""
    if let path, path.status == .satisfied {
      let names = path.availableInterfaces.filter {
        ($0.type == .wifi || $0.type == .wiredEthernet) && path.usesInterfaceType($0.type)
      }.map { $0.name }
      address = Self.lanAddress(names)
    }
    result(["name": Self.clean(Host.current().localizedName ?? "Mac"), "platform": "macOS",
      "manufacturer": "Apple", "model": Self.clean(model),
      "os_version": Self.clean(ProcessInfo.processInfo.operatingSystemVersionString), "lan_address": address])
  }

  private static func clean(_ value: String) -> String {
    let scalars = value.unicodeScalars.filter {
      !CharacterSet.controlCharacters.contains($0) && $0.value != 0x061c &&
      $0.value != 0x200e && $0.value != 0x200f && !(0x202a...0x202e).contains($0.value) &&
      !(0x2066...0x2069).contains($0.value)
    }
    return String(String.UnicodeScalarView(scalars.prefix(80)))
  }

  private static func lanAddress(_ preferred: [String]) -> String {
    var first: UnsafeMutablePointer<ifaddrs>?
    guard getifaddrs(&first) == 0, let first else { return "" }
    defer { freeifaddrs(first) }
    for name in preferred {
      var current: UnsafeMutablePointer<ifaddrs>? = first
      while let pointer = current {
        let entry = pointer.pointee
        current = entry.ifa_next
        guard String(cString: entry.ifa_name) == name, let socket = entry.ifa_addr,
          Int(socket.pointee.sa_family) == AF_INET, entry.ifa_flags & UInt32(IFF_UP) != 0 else { continue }
        var host = [CChar](repeating: 0, count: Int(NI_MAXHOST))
        if getnameinfo(socket, socklen_t(socket.pointee.sa_len), &host, socklen_t(NI_MAXHOST), nil, 0, NI_NUMERICHOST) == 0 {
          let address = String(cString: host)
          if !address.hasPrefix("127.") && !address.hasPrefix("169.254.") { return address }
        }
      }
    }
    return ""
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
    kSecUseDataProtectionKeychain as String: true,
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
    kSecUseDataProtectionKeychain as String: true,
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
    kSecUseDataProtectionKeychain as String: true,
    kSecAttrService as String: pairingKeychainService,
    kSecAttrAccount as String: pairingKeychainAccount,
    kSecAttrSynchronizable as String: kCFBooleanFalse as Any,
  ]
  let status = SecItemDelete(query as CFDictionary)
  return status == errSecSuccess || status == errSecItemNotFound ? 0 : -1
}

class MainFlutterWindow: NSWindow {
  private var receiverChannel: FlutterMethodChannel?
  private var pairingChannel: FlutterMethodChannel?
  override func awakeFromNib() {
    let controller = FlutterViewController()
    contentViewController = controller
    setContentSize(NSSize(width: 1280, height: 800))
    minSize = NSSize(width: 640, height: 480)
    title = "MirrorMesh"
    RegisterGeneratedPlugins(registry: controller)
    receiverChannel = FlutterMethodChannel(name: "com.mirrormesh/receiver_window", binaryMessenger: controller.engine.binaryMessenger)
    receiverChannel?.setMethodCallHandler { [weak self] call, result in
      guard let self else { result(FlutterError(code: "window_closed", message: "Window closed", details: nil)); return }
      switch call.method {
      case "device_context": DeviceContextQuery(result).start()
      case "fullscreen":
        guard let enabled = call.arguments as? Bool else { result(FlutterError(code: "invalid_argument", message: "Expected fullscreen setting", details: nil)); return }
        if self.styleMask.contains(.fullScreen) != enabled { self.toggleFullScreen(nil) }
        result(nil)
      default: result(FlutterMethodNotImplemented)
      }
    }
    pairingChannel = FlutterMethodChannel(name: "com.mirrormesh/desktop_pairing", binaryMessenger: controller.engine.binaryMessenger)
    pairingChannel?.setMethodCallHandler { call, result in
      guard call.method == "preparePairingRuntime" else { result(FlutterMethodNotImplemented); return }
      DispatchQueue.global(qos: .userInitiated).async {
        do {
          let manager = FileManager.default
          let support = try manager.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)
          let directory = support.appendingPathComponent("MirrorMesh", isDirectory: true)
          try manager.createDirectory(at: directory, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
          let path = directory.appendingPathComponent("trusted-peers-v1.sqlite").path
          var bytes = [UInt8](repeating: 0, count: 16)
          let status = path.withCString { pointer in
            bytes.withUnsafeMutableBufferPointer { buffer in prepareAppleRuntime(pointer, buffer.baseAddress!, UInt(buffer.count)) }
          }
          guard status == 0 else {
            DispatchQueue.main.async { result(FlutterError(code: "pairing_unavailable", message: "Unlock the macOS Keychain and try again.", details: nil)) }; return
          }
          let identity = UUID(uuid: (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15])).uuidString.lowercased()
          DispatchQueue.main.async { result(identity) }
        } catch {
          DispatchQueue.main.async { result(FlutterError(code: "storage_unavailable", message: "The local trust store could not be opened.", details: nil)) }
        }
      }
    }
    super.awakeFromNib()
  }
}
