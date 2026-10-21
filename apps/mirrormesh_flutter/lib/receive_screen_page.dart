import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_webrtc/flutter_webrtc.dart';
import 'package:qr_flutter/qr_flutter.dart';

import 'ios_pairing_bootstrap.dart';
import 'native_connection.dart';
import 'device_context.dart';
import 'connection_popups.dart';
import 'webrtc_engine.dart';
import 'receiver_window.dart';

/// The same Rust listener and approval flow used by the native Android receiver.
class ReceiveScreenPage extends StatefulWidget {
  const ReceiveScreenPage({super.key});
  @override
  State<ReceiveScreenPage> createState() => _ReceiveScreenPageState();
}

class _ReceiveScreenPageState extends State<ReceiveScreenPage> {
  final _renderer = RTCVideoRenderer();
  String? _invitation, _error;
  ConnectionPopup<bool>? _approval;
  String _status = 'Start receiving to create a QR invitation.';
  RTCPeerConnection? _peer;
  MediaStream? _remote;
  int? _connection;
  bool _active = false, _busy = false, _renderReady = false;
  int _generation = 0;
  final List<RTCIceCandidate> _pendingIce = [];
  bool _remoteDescriptionSet = false;
  bool _started = false;
  bool _ownsReceiver = false, _disposing = false;
  bool _fullscreen = false;
  Future<void>? _stopping;
  Future<Map<String, dynamic>>? _starting;

  @override
  void initState() {
    super.initState();
    unawaited(_initialize());
  }

  Future<void> _initialize() async {
    try {
      await _renderer.initialize();
      if (mounted) setState(() => _renderReady = true);
    } catch (error) {
      if (mounted) {
        setState(() => _error = 'Receiver operation failed.');
        await connectionNotice(
          context,
          'Receiver unavailable',
          'Check the network connection and restart the receiver.',
        );
      }
    }
  }

  Future<void> _start() async {
    if (_busy || _active || _stopping != null || _disposing) {
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    final generation = ++_generation;
    try {
      final result = await withConnectionProgress(
        context,
        'Starting receiver',
        () async {
          if (Platform.isIOS) await const IosPairingBootstrap().prepare();
          final address = await preferredLanAddress();
          if (!mounted || generation != _generation || _disposing) {
            throw StateError('Receiver start cancelled.');
          }
          _starting = nativeConnection({
            'op': 'receive_start',
            'address': '$address:8765',
          });
          return await _starting!;
        },
      );
      _ownsReceiver = true;
      if (!mounted || generation != _generation) {
        return;
      }
      setState(() {
        _active = true;
        _invitation = result['invitation'] as String;
        _status = 'Scan or copy this invitation on the device whose screen you want to view.';
      });
      // Start the accept loop immediately; a notice must never delay signaling.
      unawaited(_waitForRequest(generation));
    } catch (error) {
      if (mounted) {
        setState(() => _error = 'Receiver operation failed.');
        await connectionNotice(
          context,
          'Receiver unavailable',
          'Check the network connection and restart the receiver.',
        );
      }
    } finally {
      _starting = null;
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _waitForRequest(int generation) async {
    try {
      final request = await nativeConnection({'op': 'receive_wait'});
      if (!mounted || generation != _generation) return;
      final approved = await confirmIncomingDevice(
        context,
        request,
        onOpened: (popup) => _approval = popup,
      );
      _approval = null;
      if (!mounted || generation != _generation) return;
      final decision = await withConnectionProgress(
        context,
        approved ? 'Approving connection' : 'Rejecting connection',
        () => nativeConnection({
          'op': 'receive_decide',
          'id': request['request_id'],
          'approve': approved,
        }),
      );
      if (!mounted || generation != _generation || _disposing) {
        return;
      }
      if (approved != true) {
        await _stop();
        return;
      }
      _connection = decision['connection_id'] as int;
      if (mounted) {
        setState(() {
          _invitation = null;
          _status = 'Approved. Waiting for the sender to choose a screen.';
        });
      }
      unawaited(
        connectionNotice(
          context,
          'Connection approved',
          'Waiting for the sender to choose a screen.',
          brief: true,
        ),
      );
      await _poll(generation);
    } catch (error) {
      if (mounted && generation == _generation) {
        await _stop();
        if (mounted) {
          await connectionNotice(
            context,
            'Connection ended',
            'The request expired or the connection was interrupted. Start again for a fresh invitation.',
          );
        }
      }
    }
  }

  Future<void> _poll(int generation) async {
    while (mounted && generation == _generation && _connection != null) {
      final event = await nativeConnection({
        'op': 'receive_poll',
        'id': _connection,
      });
      if (!mounted || generation != _generation) return;
      if (event['pending'] == true) continue;
      switch (event['kind']) {
        case 'offer':
          await _answer(event['sdp'] as String, generation);
        case 'ice':
          final ice = RTCIceCandidate(
            event['candidate'] as String,
            event['media_id'] as String,
            event['media_line_index'] as int,
          );
          if (_remoteDescriptionSet) {
            await _peer?.addCandidate(ice);
          } else {
            _pendingIce.add(ice);
          }
        case 'start':
          _started = true;
          _renderer.srcObject = _remote;
          if (mounted) setState(() => _status = 'Receiving the shared screen');
        case 'stop' || 'revoked' || 'revoke':
          await _stop();
      }
    }
  }

  Future<void> _answer(String sdp, int generation) async {
    if (_peer != null) throw StateError('A video session is already active.');
    final peer = await createPeerConnection({
      'iceServers': <Object>[],
      'sdpSemantics': 'unified-plan',
    });
    if (!mounted || generation != _generation) {
      await peer.dispose();
      return;
    }
    _peer = peer;
    final connection = _connection;
    peer.onTrack = (event) async {
      if (!mounted || generation != _generation) return;
      if (event.track.kind != 'video') {
        event.track.enabled = false;
        return;
      }
      if (event.streams.isNotEmpty) {
        _remote = event.streams.first;
      } else {
        final remote = _remote ?? await createLocalMediaStream('received');
        if (!mounted || generation != _generation) {
          await remote.dispose();
          return;
        }
        _remote = remote;
        await remote.addTrack(event.track);
      }
      if (mounted && generation == _generation && _started) {
        _renderer.srcObject = _remote;
      }
    };
    peer.onIceCandidate = (ice) {
      if (generation == _generation &&
          _connection != null &&
          ice.candidate?.isNotEmpty == true) {
        unawaited(
          nativeConnection({
            'op': 'receive_ice',
            'id': _connection,
            'candidate': ice.candidate,
            'mid': ice.sdpMid ?? '0',
            'line': ice.sdpMLineIndex ?? 0,
          }).catchError((Object error) {
            if (mounted) setState(() => _error = error.toString());
            return <String, dynamic>{};
          }),
        );
      }
    };
    peer.onConnectionState = (state) {
      if (state == RTCPeerConnectionState.RTCPeerConnectionStateFailed &&
          generation == _generation) {
        unawaited(_stop());
      }
    };
    await peer.setRemoteDescription(RTCSessionDescription(sdp, 'offer'));
    if (generation != _generation || _disposing) return;
    final capabilities = await getRtpReceiverCapabilities('video');
    final codecs = h264VideoCodecPreferences(capabilities.codecs ?? []);
    if (codecs.isEmpty) {
      throw StateError('H.264 decoding is unavailable.');
    }
    for (final transceiver in await peer.getTransceivers()) {
      await transceiver.setCodecPreferences(codecs);
    }
    if (generation != _generation || _disposing) return;
    _remoteDescriptionSet = true;
    for (final ice in _pendingIce) {
      await peer.addCandidate(ice);
    }
    _pendingIce.clear();
    final answer = await peer.createAnswer();
    if (generation != _generation || _disposing) return;
    await peer.setLocalDescription(answer);
    if (generation != _generation || _disposing) return;
    await nativeConnection({
      'op': 'receive_answer',
      'id': connection,
      'sdp': answer.sdp,
    });
  }

  Future<void> _stop() =>
      _stopping ??= _performStop().whenComplete(() => _stopping = null);

  Future<void> _performStop() async {
    ++_generation;
    _approval?.close(false);
    _approval = null;
    if (mounted && !_disposing) setState(() => _busy = true);
    _active = false;
    _connection = null;
    _renderer.srcObject = null;
    final peer = _peer;
    _peer = null;
    final remote = _remote;
    _remote = null;
    _remoteDescriptionSet = false;
    _started = false;
    _pendingIce.clear();
    if (_fullscreen) {
      _fullscreen = false;
      try {
        await ReceiverWindow.setFullscreen(false);
      } catch (_) {
        /* media cleanup still proceeds */
      }
    }
    try {
      final starting = _starting;
      if (starting != null) {
        try {
          await starting;
          _ownsReceiver = true;
        } catch (_) {}
      }
      if (_ownsReceiver) {
        _ownsReceiver = false;
        await nativeConnection({'op': 'receive_stop'});
      }
    } catch (error) {
      if (mounted && !_disposing) setState(() => _error = error.toString());
    } finally {
      await peer?.close();
      await peer?.dispose();
      await remote?.dispose();
    }
    if (mounted && !_disposing) {
      setState(() {
        _busy = false;
        _invitation = null;
        _status = 'Receiver stopped. Start again for a new invitation.';
      });
    }
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('Receive another device’s screen'),
      actions: [
        if (Platform.isWindows && _connection != null)
          IconButton(
            tooltip: _fullscreen
                ? 'Exit full screen (Esc)'
                : 'Full screen (F11)',
            onPressed: _toggleFullscreen,
            icon: Icon(_fullscreen ? Icons.fullscreen_exit : Icons.fullscreen),
          ),
      ],
    ),
    body: _connection != null
        ? CallbackShortcuts(
            bindings: {
              const SingleActivator(LogicalKeyboardKey.f11): _toggleFullscreen,
              const SingleActivator(LogicalKeyboardKey.escape): () {
                if (_fullscreen) _toggleFullscreen();
              },
            },
            child: Focus(
              autofocus: true,
              child: SafeArea(
                child: Column(
                  children: [
                    Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      child: Row(
                        children: [
                          Expanded(
                            child: Text(
                              _error ?? _status,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                          const SizedBox(width: 16),
                          FilledButton.tonalIcon(
                            onPressed: _stop,
                            icon: const Icon(Icons.stop),
                            label: const Text('Stop receiving'),
                          ),
                        ],
                      ),
                    ),
                    Expanded(
                      child: ColoredBox(
                        color: Colors.black,
                        child: SizedBox.expand(
                          child: RTCVideoView(
                            _renderer,
                            objectFit: RTCVideoViewObjectFit
                                .RTCVideoViewObjectFitContain,
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          )
        : ListView(
            padding: const EdgeInsets.all(24),
            children: [
              Text(_status, style: Theme.of(context).textTheme.titleLarge),
              if (_error != null)
                SelectableText(
                  _error!,
                  style: TextStyle(color: Theme.of(context).colorScheme.error),
                ),
              if (!_active) ...[
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 16),
                  child: Text(
                    'Your Wi-Fi or Ethernet connection is selected automatically.',
                  ),
                ),
                FilledButton(
                  onPressed: _busy || !_renderReady ? null : _start,
                  child: const Text('Start receiving / show QR'),
                ),
              ],
              if (_invitation case final invitation?) ...[
                Center(
                  child: Semantics(
                    label: 'Receiver QR invitation. Expires after two minutes.',
                    child: QrImageView(
                      data: invitation,
                      size: 280,
                      backgroundColor: Colors.white,
                    ),
                  ),
                ),
                OutlinedButton.icon(
                  onPressed: () async {
                    await Clipboard.setData(ClipboardData(text: invitation));
                    if (context.mounted) {
                      await connectionNotice(
                        context,
                        'Invitation copied',
                        'Share it only with the device you want to connect.',
                        brief: true,
                      );
                    }
                  },
                  icon: const Icon(Icons.copy),
                  label: const Text('Copy invitation'),
                ),
                const Text(
                  'Use MirrorMesh on the sending device. QR pairing does not grant screen-capture permission.',
                ),
              ],
              if (_active)
                FilledButton.tonal(
                  onPressed: _stop,
                  child: const Text('Stop receiving'),
                ),
            ],
          ),
  );

  Future<void> _toggleFullscreen() async {
    try {
      final next = !_fullscreen;
      await ReceiverWindow.setFullscreen(next);
      if (mounted) setState(() => _fullscreen = next);
    } catch (error) {
      if (mounted) setState(() => _error = error.toString());
    }
  }

  @override
  void dispose() {
    _disposing = true;
    unawaited(_stop().whenComplete(_renderer.dispose));
    super.dispose();
  }
}
