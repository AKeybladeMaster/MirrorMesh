import 'dart:io';
import 'dart:math';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'src/rust/bridge_api.dart';
import 'src/rust/frb_generated.dart';

/// Narrow access to one local Rust session and its state-only event stream.
///
/// Demo commands stop at Pairing and never grant identity, capture permission,
/// or media-stream authorization.
class RustSessionStatus {
  RustSessionStatus._({
    required this.isAvailable,
    required this.initialState,
    this._controller,
    this._sessionId,
  });

  factory RustSessionStatus.unavailable() =>
      RustSessionStatus._(isAvailable: false, initialState: 'Unavailable');

  final bool isAvailable;
  final String initialState;
  final BridgeController? _controller;
  final String? _sessionId;

  Stream<String> get events {
    final controller = _controller;
    final sessionId = _sessionId;
    if (controller == null || sessionId == null) {
      return const Stream<String>.empty();
    }
    return controller.sessionStateEvents(sessionId: sessionId);
  }

  String currentState() {
    final controller = _controller;
    final sessionId = _sessionId;
    if (controller == null || sessionId == null) return 'Unavailable';
    return controller.currentState(sessionId: sessionId);
  }

  String discoverDemoReceiver() => _runDemoTransition(
    (controller, sessionId) =>
        controller.discoverDemoReceiver(sessionId: sessionId),
  );

  String beginDemoPairing() => _runDemoTransition(
    (controller, sessionId) =>
        controller.beginDemoPairing(sessionId: sessionId),
  );

  String disconnectDemoReceiver() => _runDemoTransition(
    (controller, sessionId) =>
        controller.disconnectDemoReceiver(sessionId: sessionId),
  );

  static Future<RustSessionStatus> initialize({String? libraryPath}) async {
    try {
      final library = libraryPath == null && Platform.isIOS
          ? ExternalLibrary.process(iKnowHowToUseIt: true)
          : ExternalLibrary.open(libraryPath ?? _defaultLibraryPath());
      await RustLib.init(externalLibrary: library);

      final controller = BridgeController();
      final sessionId = _newUuidV4();
      controller.createSession(sessionId: sessionId);

      return RustSessionStatus._(
        isAvailable: true,
        initialState: controller.currentState(sessionId: sessionId),
        controller: controller,
        sessionId: sessionId,
      );
    } on Object {
      return RustSessionStatus.unavailable();
    }
  }

  String stopSession() {
    final controller = _controller;
    final sessionId = _sessionId;
    if (controller == null || sessionId == null) return 'Unavailable';
    return controller.stopSession(sessionId: sessionId);
  }

  String _runDemoTransition(
    String Function(BridgeController controller, String sessionId) transition,
  ) {
    final controller = _controller;
    final sessionId = _sessionId;
    if (controller == null || sessionId == null) return 'Unavailable';
    return transition(controller, sessionId);
  }

  static String _defaultLibraryPath() {
    if (Platform.isAndroid) return 'libmm_ffi.so';

    final executableDirectory = File(Platform.resolvedExecutable).parent.path;
    if (Platform.isLinux) {
      return '$executableDirectory${Platform.pathSeparator}lib'
          '${Platform.pathSeparator}libmm_ffi.so';
    }
    final libraryName = switch (Platform.operatingSystem) {
      'windows' => 'mm_ffi.dll',
      'macos' => 'libmm_ffi.dylib',
      _ => throw UnsupportedError(
        'Rust bridge is unavailable on this platform.',
      ),
    };
    return '$executableDirectory${Platform.pathSeparator}$libraryName';
  }

  static String _newUuidV4() {
    final random = Random.secure();
    final bytes = List<int>.generate(16, (_) => random.nextInt(256));
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    final hex = bytes
        .map((byte) => byte.toRadixString(16).padLeft(2, '0'))
        .join();
    return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-'
        '${hex.substring(12, 16)}-${hex.substring(16, 20)}-'
        '${hex.substring(20)}';
  }
}
