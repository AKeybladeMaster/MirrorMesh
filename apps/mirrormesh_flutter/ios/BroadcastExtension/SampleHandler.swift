import ReplayKit
import CoreImage
import ImageIO
import Darwin

/// ReplayKit -> app-group Unix socket -> native WebRTC; no frames pass through Dart.
final class SampleHandler: RPBroadcastSampleHandler {
  private let imageContext = CIContext(options: [.cacheIntermediates: false])
  private var socketFD: Int32 = -1
  private var lastFrameTime = 0.0
  private let socketLock = NSRecursiveLock()

  override func broadcastStarted(withSetupInfo setupInfo: [String: NSObject]?) {
    socketLock.lock()
    defer { socketLock.unlock() }
    guard let group = Bundle.main.object(forInfoDictionaryKey: "RTCAppGroupIdentifier") as? String,
          let folder = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: group) else {
      fail("The broadcast app group is unavailable."); return
    }
    let path = folder.appendingPathComponent("rtc_SSFD").path
    var address = sockaddr_un()
    guard path.utf8.count < MemoryLayout.size(ofValue: address.sun_path) else {
      fail("The broadcast socket path is too long."); return
    }
    socketFD = Darwin.socket(AF_UNIX, SOCK_STREAM, 0)
    guard socketFD >= 0 else { fail("The broadcast socket could not open."); return }
    var noSignal: Int32 = 1
    setsockopt(socketFD, SOL_SOCKET, SO_NOSIGPIPE, &noSignal, socklen_t(MemoryLayout<Int32>.size))
    var timeout = timeval(tv_sec: 0, tv_usec: 150_000)
    setsockopt(socketFD, SOL_SOCKET, SO_SNDTIMEO, &timeout, socklen_t(MemoryLayout<timeval>.size))
    address.sun_family = sa_family_t(AF_UNIX)
    address.sun_len = UInt8(MemoryLayout<sockaddr_un>.size)
    withUnsafeMutableBytes(of: &address.sun_path) { target in
      target.initializeMemory(as: UInt8.self, repeating: 0)
      _ = path.utf8CString.withUnsafeBytes { source in memcpy(target.baseAddress!, source.baseAddress!, source.count) }
    }
    let result = withUnsafePointer(to: &address) { pointer in
      pointer.withMemoryRebound(to: sockaddr.self, capacity: 1) {
        Darwin.connect(socketFD, $0, socklen_t(MemoryLayout<sockaddr_un>.size))
      }
    }
    if result != 0 { fail("Open MirrorMesh and start sharing before starting the broadcast.") }
  }

  override func processSampleBuffer(_ sampleBuffer: CMSampleBuffer, with sampleBufferType: RPSampleBufferType) {
    socketLock.lock()
    defer { socketLock.unlock() }
    guard sampleBufferType == .video, socketFD >= 0,
          let buffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { return }
    let now = ProcessInfo.processInfo.systemUptime
    guard now - lastFrameTime >= 1.0 / 15.0 else { return }
    lastFrameTime = now
    autoreleasepool {
      let original = CIImage(cvPixelBuffer: buffer)
      let scale = min(1.0, 1280.0 / max(original.extent.width, original.extent.height))
      let image = original.transformed(by: CGAffineTransform(scaleX: scale, y: scale))
      guard var jpeg = imageContext.jpegRepresentation(of: image,
          colorSpace: CGColorSpaceCreateDeviceRGB(), options: [:]),
          jpeg.count <= 2 * 1024 * 1024 else { return }
      // The pinned native reader starts each message with a 10 KiB read. A valid,
      // ignored JPEG APP15 segment keeps even flat images larger than that read,
      // so a stream read cannot consume the following frame's header.
      if jpeg.count < 12 * 1024 {
        guard jpeg.count >= 2, jpeg[0] == 0xff, jpeg[1] == 0xd8 else { return }
        let padding = max(4, 12 * 1024 - jpeg.count)
        let segmentLength = padding - 2
        var padded = Data([0xff, 0xd8, 0xff, 0xef, UInt8(segmentLength >> 8), UInt8(segmentLength & 255)])
        padded.append(Data(repeating: 0, count: padding - 4))
        padded.append(jpeg.dropFirst(2))
        jpeg = padded
      }
      let orientation = (CMGetAttachment(sampleBuffer, key: RPVideoSampleOrientationKey as CFString,
          attachmentModeOut: nil) as? NSNumber)?.intValue ?? 1
      let header = "HTTP/1.1 200 OK\r\nContent-Length: \(jpeg.count)\r\nBuffer-Width: \(Int(image.extent.width))\r\nBuffer-Height: \(Int(image.extent.height))\r\nBuffer-Orientation: \(orientation)\r\n\r\n"
      var packet = Data(header.utf8)
      packet.append(jpeg)
      let expires = ProcessInfo.processInfo.systemUptime + 0.25
      let complete = packet.withUnsafeBytes { bytes -> Bool in
        var offset = 0
        while offset < bytes.count && ProcessInfo.processInfo.systemUptime < expires {
          let written = Darwin.send(socketFD, bytes.baseAddress!.advanced(by: offset), bytes.count - offset, 0)
          guard written > 0 else { return false }
          offset += written
        }
        return offset == bytes.count
      }
      if !complete { fail("The MirrorMesh screen-sharing session ended or could not keep up.") }
    }
  }

  override func broadcastFinished() { closeSocket() }
  private func closeSocket() {
    socketLock.lock()
    defer { socketLock.unlock() }
    if socketFD >= 0 { Darwin.close(socketFD); socketFD = -1 }
  }
  private func fail(_ message: String) {
    closeSocket()
    finishBroadcastWithError(NSError(domain: "MirrorMeshBroadcast", code: 1,
                                    userInfo: [NSLocalizedDescriptionKey: message]))
  }
}
