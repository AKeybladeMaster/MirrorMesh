import 'dart:async';
import 'dart:convert';

import 'package:flutter/services.dart';

import 'webrtc_engine.dart';
import 'windows_pairing_bootstrap.dart';

const _maxBufferedIceCandidates = 128;
const _maxBufferedIceBytes = 64 * 1024;

enum WindowsSenderSessionState {
  idle,
  negotiating,
  transportConnected,
  stopped,
  revoked,
  failed,
}

/// Coordinates a real libwebrtc offer with the Windows authenticated WSS bridge.
///
/// The user selects a desktop source before this controller opens native capture;
/// the selected native video track is attached before the SDP offer is created.
final class WindowsSenderSessionController {
  WindowsSenderSessionController({
    required this.pairing,
    WebRtcPeerConnectionFactory? factory,
    this.statsPollInterval = const Duration(seconds: 1),
  }) : _engineFactory = factory ?? const FlutterWebRtcPeerConnectionFactory(),
       assert(statsPollInterval.inMicroseconds > 0);

  final WindowsPairingBootstrap pairing;
  final WebRtcPeerConnectionFactory _engineFactory;
  final Duration statsPollInterval;
  final StreamController<WindowsSenderSessionState> _states =
      StreamController<WindowsSenderSessionState>.broadcast();
  final StreamController<WebRtcOutboundVideoStats> _videoStats =
      StreamController<WebRtcOutboundVideoStats>.broadcast();
  final StreamController<WebRtcEngineEvent> _rtcEvents =
      StreamController<WebRtcEngineEvent>.broadcast();
  final List<WebRtcIceCandidate> _pendingLocalIce = [];
  final List<WebRtcIceCandidate> _pendingRemoteIce = [];

  WindowsSenderSessionState _state = WindowsSenderSessionState.idle;
  FlutterWebRtcEngine? _engine;
  StreamSubscription<WebRtcEngineEvent>? _engineEvents;
  Timer? _pollTimer;
  Timer? _statsTimer;
  Completer<void>? _localIceDrainComplete;
  String? _sessionId;
  int? _requestId;
  int _pendingLocalIceBytes = 0;
  int _pendingRemoteIceBytes = 0;
  bool _remoteDescriptionSet = false;
  bool _polling = false;
  bool _statsPolling = false;
  bool _sessionStartInFlight = false;
  bool _sessionStartQueued = false;
  bool _transportReady = false;
  bool _starting = false;
  bool _closed = false;
  Object? _lastError;

  WindowsSenderSessionState get state => _state;
  Object? get lastError => _lastError;
  Stream<WindowsSenderSessionState> get states => _states.stream;
  Stream<WebRtcOutboundVideoStats> get videoStats => _videoStats.stream;
  Stream<WebRtcEngineEvent> get rtcEvents => _rtcEvents.stream;

  Future<List<WebRtcDesktopCaptureSource>> availableCaptureSources() {
    if (_closed || _starting || _engine != null) {
      throw StateError(
        'Capture sources cannot be listed while a session is active.',
      );
    }
    return _engineFactory.listDesktopCaptureSources();
  }

  Future<String> start({
    required int requestId,
    required String sourceId,
  }) async {
    if (_closed || _starting || _sessionId != null || _engine != null) {
      throw StateError('Windows sender session is already active or closed.');
    }
    _starting = true;
    _requestId = requestId;
    _sessionStartInFlight = false;
    _sessionStartQueued = false;
    _transportReady = false;
    _lastError = null;
    _setState(WindowsSenderSessionState.negotiating);
    final engine = FlutterWebRtcEngine(factory: _engineFactory);
    _engine = engine;

    try {
      final pairingStatus = await pairing.pollSenderPairing(requestId);
      if (pairingStatus.state != WindowsSenderPairingState.approved) {
        throw StateError('The Android TV has not approved this pairing.');
      }
      await engine.initialize();
      await engine.startDesktopCapture(sourceId: sourceId);
      _engineEvents = engine.events.listen(
        _onRtcEvent,
        onError: (Object error, StackTrace stackTrace) {
          unawaited(_fail(error));
        },
      );
      final offer = await engine.createOffer();
      await engine.setLocalDescription(
        type: WebRtcDescriptionType.offer,
        sdp: offer,
      );
      _sessionId = await pairing.beginSenderSession(
        requestId: requestId,
        sdp: offer,
      );
      await _drainLocalIce();
      if (_transportReady) await _sendSessionStart();
      _pollTimer = Timer.periodic(
        const Duration(milliseconds: 100),
        (_) => unawaited(_pollSessionEvent()),
      );
      _statsTimer = Timer.periodic(
        statsPollInterval,
        (_) => unawaited(_pollOutboundVideoStats()),
      );
      return _sessionId!;
    } catch (error) {
      await _closePeer();
      await _closePairingChannel();
      _lastError = error;
      _setState(WindowsSenderSessionState.failed);
      rethrow;
    } finally {
      _starting = false;
    }
  }

  Future<void> stop() async {
    if (_closed) return;
    await _closePeer();
    await _closePairingChannel();
    _setState(WindowsSenderSessionState.stopped);
  }

  Future<void> dispose() async {
    if (_closed) return;
    await stop();
    _closed = true;
    await _states.close();
    await _videoStats.close();
    await _rtcEvents.close();
  }

  Future<void> _pollOutboundVideoStats() async {
    final engine = _engine;
    if (_statsPolling || _closed || engine == null) return;
    _statsPolling = true;
    try {
      final stats = await engine.getOutboundVideoStats();
      if (stats != null &&
          identical(_engine, engine) &&
          !_videoStats.isClosed) {
        _videoStats.add(stats);
      }
    } on Object {
      // Stats are diagnostic only; a native stats failure must not stop video.
    } finally {
      _statsPolling = false;
    }
  }

  void _onRtcEvent(WebRtcEngineEvent event) {
    if (_closed) return;
    _rtcEvents.add(event);
    if (event is WebRtcLocalIceCandidateEvent) {
      _queueLocalIce(event.candidate);
    } else if (event is WebRtcConnectionStateEvent) {
      switch (event.state) {
        case WebRtcConnectionState.connected:
          _transportReady = true;
          _setState(WindowsSenderSessionState.transportConnected);
          unawaited(_sendSessionStart());
        case WebRtcConnectionState.failed:
        case WebRtcConnectionState.closed:
          unawaited(_fail(StateError('The WebRTC peer connection failed.')));
        case WebRtcConnectionState.newConnection:
        case WebRtcConnectionState.connecting:
        case WebRtcConnectionState.disconnected:
          break;
      }
    }
  }

  Future<void> _sendSessionStart() async {
    if (!_transportReady ||
        _closed ||
        _sessionStartQueued ||
        _sessionStartInFlight ||
        _sessionId == null) {
      return;
    }
    _sessionStartInFlight = true;
    try {
      await _drainLocalIce();
      if (_closed || _sessionId == null || _sessionStartQueued) return;
      await pairing.startSenderSession(
        requestId: _requestId!,
        sourceId: 'desktop-capture',
      );
      _sessionStartQueued = true;
    } catch (error) {
      await _fail(error);
    } finally {
      _sessionStartInFlight = false;
    }
  }

  void _queueLocalIce(WebRtcIceCandidate candidate) {
    if (candidate.sdpMid == null || candidate.sdpMLineIndex == null) {
      unawaited(
        _fail(StateError('Local ICE candidate is missing its media section.')),
      );
      return;
    }
    final bytes = utf8.encode(candidate.candidate).length;
    if (_pendingLocalIce.length >= _maxBufferedIceCandidates ||
        _pendingLocalIceBytes + bytes > _maxBufferedIceBytes) {
      unawaited(_fail(StateError('The local ICE candidate buffer is full.')));
      return;
    }
    _pendingLocalIce.add(candidate);
    _pendingLocalIceBytes += bytes;
    unawaited(_drainLocalIce());
  }

  Future<void> _drainLocalIce() {
    if (_sessionId == null || _closed) return Future<void>.value();
    final activeDrain = _localIceDrainComplete;
    if (activeDrain != null) return activeDrain.future;
    final completion = Completer<void>();
    _localIceDrainComplete = completion;
    unawaited(_runLocalIceDrain(completion));
    return completion.future;
  }

  Future<void> _runLocalIceDrain(Completer<void> completion) async {
    try {
      while (_pendingLocalIce.isNotEmpty && !_closed) {
        final candidate = _pendingLocalIce.removeAt(0);
        _pendingLocalIceBytes -= utf8.encode(candidate.candidate).length;
        await pairing.sendSenderSessionIce(
          requestId: _requestId!,
          candidate: candidate.candidate,
          mediaId: candidate.sdpMid!,
          mediaLineIndex: candidate.sdpMLineIndex!,
        );
      }
    } catch (error) {
      await _fail(error);
    } finally {
      if (identical(_localIceDrainComplete, completion)) {
        _localIceDrainComplete = null;
      }
      if (!completion.isCompleted) completion.complete();
    }
  }

  Future<void> _pollSessionEvent() async {
    if (_polling || _closed || _sessionId == null) return;
    _polling = true;
    try {
      final event = await pairing.pollSenderSessionEvent(_requestId!);
      switch (event.kind) {
        case WindowsSenderSessionEventKind.pending:
          break;
        case WindowsSenderSessionEventKind.answer:
          if (event.sessionId != _sessionId) {
            throw StateError('The TV answer belongs to a different session.');
          }
          await _engine!.setRemoteDescription(
            type: WebRtcDescriptionType.answer,
            sdp: event.sdp!,
          );
          _remoteDescriptionSet = true;
          await _drainRemoteIce();
        case WindowsSenderSessionEventKind.ice:
          if (event.sessionId != _sessionId) {
            throw StateError(
              'The TV ICE candidate belongs to a different session.',
            );
          }
          final candidate = WebRtcIceCandidate(
            candidate: event.candidate!,
            sdpMid: event.mediaId,
            sdpMLineIndex: event.mediaLineIndex,
          );
          if (_remoteDescriptionSet) {
            await _engine!.addRemoteIceCandidate(candidate);
          } else {
            _queueRemoteIce(candidate);
          }
        case WindowsSenderSessionEventKind.stopped:
          if (event.sessionId != _sessionId) {
            throw StateError('The TV stop belongs to a different session.');
          }
          await _closePeer();
          _setState(WindowsSenderSessionState.stopped);
        case WindowsSenderSessionEventKind.revoked:
          await _closePeer();
          _setState(WindowsSenderSessionState.revoked);
      }
    } catch (error) {
      await _fail(error);
    } finally {
      _polling = false;
    }
  }

  void _queueRemoteIce(WebRtcIceCandidate candidate) {
    final bytes = utf8.encode(candidate.candidate).length;
    if (_pendingRemoteIce.length >= _maxBufferedIceCandidates ||
        _pendingRemoteIceBytes + bytes > _maxBufferedIceBytes) {
      throw StateError('The remote ICE candidate buffer is full.');
    }
    _pendingRemoteIce.add(candidate);
    _pendingRemoteIceBytes += bytes;
  }

  Future<void> _drainRemoteIce() async {
    while (_pendingRemoteIce.isNotEmpty && !_closed) {
      final candidate = _pendingRemoteIce.removeAt(0);
      _pendingRemoteIceBytes -= utf8.encode(candidate.candidate).length;
      await _engine!.addRemoteIceCandidate(candidate);
    }
  }

  Future<void> _fail(Object error) async {
    if (_closed || _state == WindowsSenderSessionState.failed) return;
    await _closePeer();
    await _closePairingChannel();
    _lastError = error;
    _setState(WindowsSenderSessionState.failed);
  }

  Future<void> _closePairingChannel() async {
    final requestId = _requestId;
    if (requestId == null) return;
    try {
      await pairing.cancelSenderPairing(requestId);
    } on PlatformException {
      // A remote revoke or closed channel has already completed this cleanup.
    }
  }

  Future<void> _closePeer() async {
    _pollTimer?.cancel();
    _pollTimer = null;
    _statsTimer?.cancel();
    _statsTimer = null;
    await _engineEvents?.cancel();
    _engineEvents = null;
    final engine = _engine;
    _engine = null;
    _sessionId = null;
    _pendingLocalIce.clear();
    _pendingRemoteIce.clear();
    _pendingLocalIceBytes = 0;
    _pendingRemoteIceBytes = 0;
    _remoteDescriptionSet = false;
    if (engine != null) await engine.close();
  }

  void _setState(WindowsSenderSessionState state) {
    if (_state == state || _states.isClosed) return;
    _state = state;
    _states.add(state);
  }
}
