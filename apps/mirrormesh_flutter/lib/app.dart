import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'rust_session_status.dart';
import 'webrtc_engine.dart';
import 'windows_pairing_bootstrap.dart';
import 'windows_sender_session.dart';
import 'network_devices.dart';
import 'receive_screen_page.dart';
import 'mobile_connection_home.dart';
import 'native_connection.dart';
import 'airplay_receiver_page.dart';

class MirrorMeshApp extends StatelessWidget {
  const MirrorMeshApp({super.key, this.rustSessionStatus});
  final RustSessionStatus? rustSessionStatus;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'MirrorMesh',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(
      useMaterial3: true,
      colorScheme: ColorScheme.fromSeed(
        seedColor: const Color(0xFF61D7C5),
        brightness: Brightness.dark,
        surface: const Color(0xFF111923),
      ),
      scaffoldBackgroundColor: const Color(0xFF0A1017),
      inputDecorationTheme: const InputDecorationTheme(
        border: OutlineInputBorder(),
      ),
    ),
    home: Platform.isIOS
        ? const MobileConnectionHome()
        : const MirrorMeshHome(),
  );
}

/// Release UI connected to the native pairing and media adapters.
class MirrorMeshHome extends StatefulWidget {
  const MirrorMeshHome({super.key});
  @override
  State<MirrorMeshHome> createState() => _MirrorMeshHomeState();
}

class _MirrorMeshHomeState extends State<MirrorMeshHome> {
  static const _pairing = WindowsPairingBootstrap();
  final _invitation = TextEditingController();
  String? _deviceId, _error, _sourceName;
  int? _requestId;
  bool _busy = false, _polling = false;
  bool _searching = false;
  List<Map<String, dynamic>> _nearby = [];
  String _deviceFilter = '';
  String? _directCode;
  bool _directRequest = false, _directConfirmed = false;
  Timer? _pairingTimer;
  WindowsSenderPairingStatus? _pairingStatus;
  WindowsSenderSessionController? _session;
  WindowsSenderSessionState? _sessionState;
  WebRtcOutboundVideoStats? _stats;
  StreamSubscription<WindowsSenderSessionState>? _stateSubscription;
  StreamSubscription<WebRtcOutboundVideoStats>? _statsSubscription;
  bool get _approved =>
      _pairingStatus?.state == WindowsSenderPairingState.approved;

  String get _status {
    if (_sessionState case final state?) {
      return switch (state) {
        WindowsSenderSessionState.idle => 'Ready to share',
        WindowsSenderSessionState.negotiating => 'Connecting video…',
        WindowsSenderSessionState.transportConnected =>
          'Connected · waiting for the first encoded frame',
        WindowsSenderSessionState.streaming => 'Sharing your screen',
        WindowsSenderSessionState.reconnecting =>
          'Connection interrupted · reconnecting…',
        WindowsSenderSessionState.stopped => 'Screen share ended',
        WindowsSenderSessionState.revoked => 'The receiver revoked access',
        WindowsSenderSessionState.failed => 'Screen share failed',
      };
    }
    return switch (_pairingStatus?.state) {
      WindowsSenderPairingState.connecting => 'Checking the receiver identity…',
      WindowsSenderPairingState.awaitingApproval =>
        'Approve this PC on the receiver',
      WindowsSenderPairingState.approved =>
        'Paired · choose a screen or window',
      WindowsSenderPairingState.rejected =>
        'The receiver rejected this request',
      WindowsSenderPairingState.failed =>
        'Pairing failed · refresh the receiver code and retry',
      WindowsSenderPairingState.cancelling => 'Disconnecting…',
      WindowsSenderPairingState.cancelled => 'Disconnected',
      null => 'Connect your PC to an Android receiver',
    };
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(title: const Text('MirrorMesh 0.3.0')),
    body: SafeArea(
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 900),
          child: ListView(
            padding: const EdgeInsets.all(24),
            children: [
              Text(
                'Your screen. Your devices.',
                style: Theme.of(context).textTheme.headlineMedium,
              ),
              const SizedBox(height: 8),
              const Text(
                'Windows sender → Android phone, tablet or TV receiver',
              ),
              const SizedBox(height: 24),
              OutlinedButton.icon(
                onPressed:
                    _requestId == null &&
                        !_busy &&
                        (Platform.isWindows || Platform.isIOS)
                    ? () => Navigator.of(context).push<void>(
                        MaterialPageRoute(
                          builder: (_) => const ReceiveScreenPage(),
                        ),
                      )
                    : null,
                icon: const Icon(Icons.qr_code_2),
                label: const Text(
                  'Receive a device’s screen / show this PC’s QR',
                ),
              ),
              OutlinedButton.icon(
                onPressed: Platform.isWindows && !_searching
                    ? _searchDevices
                    : null,
                icon: Icon(_searching ? Icons.hourglass_top : Icons.search),
                label: Text(
                  _searching
                      ? 'Searching the local network…'
                      : 'Find devices on this network',
                ),
              ),
              if (Platform.isWindows)
                OutlinedButton.icon(
                  onPressed: !_busy && _requestId == null
                      ? () => Navigator.of(context).push<void>(
                          MaterialPageRoute(
                            builder: (_) => const AirPlayReceiverPage(),
                          ),
                        )
                      : null,
                  icon: const Icon(Icons.airplay),
                  label: const Text(
                    'Receive iPhone / iPad with AirPlay (preview)',
                  ),
                ),
              if (_nearby.isNotEmpty) ...[
                const SizedBox(height: 12),
                TextField(
                  decoration: const InputDecoration(
                    labelText: 'Search by device name',
                    prefixIcon: Icon(Icons.search),
                  ),
                  onChanged: (value) =>
                      setState(() => _deviceFilter = value.toLowerCase()),
                ),
                const Text(
                  'Select a receiver to request a connection. Compare the connection codes on both screens before approving.',
                ),
                for (final device in _nearby.where(
                  (item) => (item['name'] as String).toLowerCase().contains(
                    _deviceFilter,
                  ),
                ))
                  ListTile(
                    leading: Icon(
                      device['receive'] == true ? Icons.cast : Icons.computer,
                    ),
                    title: Text(device['name'] as String),
                    subtitle: Text(
                      '${device['receive'] == true ? 'Can receive screens' : 'Sender'} · ${(device['endpoints'] as List).join(', ')}',
                    ),
                    trailing: device['receive'] == true
                        ? const Icon(Icons.arrow_forward)
                        : null,
                    onTap:
                        device['receive'] == true &&
                            !_busy &&
                            _requestId == null
                        ? () => _selectNearbyDevice(device)
                        : null,
                  ),
              ],
              const SizedBox(height: 16),
              Card(
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          const Icon(Icons.cast),
                          const SizedBox(width: 12),
                          Expanded(
                            child: Text(
                              _status,
                              style: Theme.of(context).textTheme.titleLarge,
                            ),
                          ),
                        ],
                      ),
                      if (_busy) ...[
                        const SizedBox(height: 16),
                        const LinearProgressIndicator(),
                      ],
                      if (_directCode case final code?) ...[
                        const SizedBox(height: 16),
                        const Text(
                          'Compare all four groups with the receiver. Reject if they differ.',
                        ),
                        SelectableText(
                          code,
                          style: Theme.of(context).textTheme.headlineMedium,
                        ),
                        FilledButton.icon(
                          onPressed: _directConfirmed || _busy
                              ? null
                              : _confirmDirectCode,
                          icon: Icon(
                            _directConfirmed
                                ? Icons.check
                                : Icons.verified_user_outlined,
                          ),
                          label: Text(
                            _directConfirmed
                                ? 'Code confirmed · awaiting receiver approval'
                                : 'The codes match',
                          ),
                        ),
                      ],
                      const SizedBox(height: 24),
                      TextField(
                        controller: _invitation,
                        enabled: !_busy && _requestId == null,
                        maxLines: 3,
                        maxLength: 4096,
                        autocorrect: false,
                        decoration: const InputDecoration(
                          labelText: 'Receiver invitation',
                          hintText:
                              'Paste the complete mirrormesh:// invitation',
                          helperText: 'Start the Android receiver, then copy or share its invitation.',
                        ),
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 12,
                        runSpacing: 12,
                        children: [
                          FilledButton.icon(
                            onPressed:
                                Platform.isWindows &&
                                    !_busy &&
                                    _requestId == null
                                ? _connect
                                : null,
                            icon: const Icon(Icons.link),
                            label: const Text('Pair with receiver'),
                          ),
                          OutlinedButton.icon(
                            onPressed: !_busy && _requestId == null
                                ? _pasteInvitation
                                : null,
                            icon: const Icon(Icons.content_paste),
                            label: const Text('Paste'),
                          ),
                          if (_requestId != null)
                            OutlinedButton.icon(
                              onPressed: _busy ? null : _disconnect,
                              icon: const Icon(Icons.link_off),
                              label: const Text('Disconnect'),
                            ),
                        ],
                      ),
                      if (_error != null) ...[
                        const SizedBox(height: 16),
                        SelectableText(
                          _error!,
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ],
                      if (_approved || _session != null) ...[
                        const Divider(height: 40),
                        Text(
                          'Share a screen or window',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        const SizedBox(height: 8),
                        const Text(
                          'Capture begins only after you select a source. Everything visible in that source may be shared.',
                        ),
                        const SizedBox(height: 16),
                        Wrap(
                          spacing: 12,
                          runSpacing: 12,
                          children: [
                            FilledButton.icon(
                              onPressed: _busy || _session != null || !_approved
                                  ? null
                                  : _startSharing,
                              icon: const Icon(Icons.screen_share),
                              label: const Text('Choose what to share'),
                            ),
                            if (_session != null)
                              FilledButton.tonalIcon(
                                onPressed: _busy ? null : _disconnect,
                                icon: const Icon(Icons.stop_screen_share),
                                label: const Text('Stop sharing'),
                              ),
                          ],
                        ),
                      ],
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 20),
              ExpansionTile(
                title: const Text('Identity and session diagnostics'),
                childrenPadding: const EdgeInsets.all(16),
                expandedCrossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  SelectableText(
                    'PC identity: ${_deviceId ?? 'Created when you pair'}',
                  ),
                  const SizedBox(height: 8),
                  SelectableText(
                    'Receiver fingerprint: ${_pairingStatus?.receiverFingerprint.isNotEmpty == true ? _pairingStatus!.receiverFingerprint : 'Not verified yet'}',
                  ),
                  const SizedBox(height: 8),
                  Text('Source: ${_sourceName ?? 'None'}'),
                  Text(
                    'Encoded frames: ${_stats?.framesEncoded ?? 0} · bytes sent: ${_stats?.bytesSent ?? 0}',
                  ),
                  Text(
                    'Codec: ${_stats?.codecMimeType ?? 'Waiting'} · encoder: ${_stats?.encoderImplementation ?? 'Not reported'}',
                  ),
                  const SizedBox(height: 8),
                  const Text(
                    'Sender statistics confirm encoding and transmission only. Check the receiver for rendered video.',
                  ),
                  TextButton.icon(
                    onPressed: _copyDiagnostics,
                    icon: const Icon(Icons.copy),
                    label: const Text('Copy diagnostics'),
                  ),
                ],
              ),
              const ExpansionTile(
                title: Text('How to connect'),
                childrenPadding: EdgeInsets.all(16),
                children: [
                  Text(
                    '1. Connect both devices to the same trusted LAN.\n\n2. Start the Android receiver on its Wi-Fi or Ethernet address. Copy or share its invitation, or scan the TV QR with your phone.\n\n3. Paste the full invitation here. On the receiver, review this PC, select Allow view and approve.\n\n4. Choose a source to begin sharing. Stop sharing on the PC or Disconnect on the receiver ends capture.\n\nInvitations expire and are single use. Refresh the receiver code before pairing again. Guest Wi-Fi or client isolation may prevent connections. Audio and Internet relays are not configured in this release candidate.',
                  ),
                ],
              ),
              const SizedBox(height: 20),
              Text(
                Platform.isWindows
                    ? 'Local encrypted screen sharing · receiver approval · no account required'
                    : 'This Flutter release sends screens on Windows. Use the native Android receiver APK on phones and TVs.',
              ),
            ],
          ),
        ),
      ),
    ),
  );

  Future<void> _pasteInvitation() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    if (mounted && data?.text != null) _invitation.text = data!.text!.trim();
  }

  Future<void> _searchDevices() async {
    setState(() {
      _searching = true;
      _error = null;
    });
    try {
      final devices = await discoverNetworkDevices();
      if (!mounted) return;
      setState(
        () => _nearby = devices
            .where((device) => device['id'] != _deviceId)
            .toList(),
      );
      if (_nearby.isEmpty) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text(
              'No MirrorMesh receivers found. Start the Android/TV receiver, check the same LAN, then search again. Other TVs need the MirrorMesh receiver app.',
            ),
          ),
        );
      }
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) setState(() => _searching = false);
    }
  }

  Future<void> _selectNearbyDevice(Map<String, dynamic> device) async {
    if (_busy || _requestId != null) return;
    if (device['request_fingerprint'] is String) {
      final endpoints = (device['endpoints'] as List).cast<String>().toList()
        ..sort(
          (a, b) => a.startsWith('[') == b.startsWith('[')
              ? a.compareTo(b)
              : (a.startsWith('[') ? 1 : -1),
        );
      if (endpoints.isEmpty) return;
      String? endpoint = endpoints.first;
      if (endpoints.length > 1) {
        endpoint = await showDialog<String>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text('Connect to ${device['name']}'),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(
                  'Choose the receiver address on your local network:',
                ),
                for (final address in endpoints)
                  ListTile(
                    title: Text(address),
                    trailing: const Icon(Icons.send),
                    onTap: () => Navigator.pop(context, address),
                  ),
              ],
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: const Text('Cancel'),
              ),
            ],
          ),
        );
      }
      if (endpoint != null && mounted) {
        await _connect(device: device, endpoint: endpoint);
      }
      return;
    }
    if (!mounted) return;
    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text('Connect to ${device['name']}'),
        content: const Text(
          'This receiver uses an older MirrorMesh version. Update it to enable direct requests, or use its invitation for this connection.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Enter invitation'),
          ),
        ],
      ),
    );
  }

  Future<void> _connect({
    Map<String, dynamic>? device,
    String? endpoint,
  }) async {
    if (_busy || _requestId != null) return;
    setState(() {
      _busy = true;
      _error = null;
      _sessionState = null;
      _stats = null;
      _directRequest = device != null;
      _directCode = null;
      _directConfirmed = false;
    });
    try {
      final text = _invitation.text.trim();
      if (device == null && (text.isEmpty || text.length > 4096)) {
        throw StateError('Paste a complete receiver invitation first.');
      }
      _deviceId ??= await _pairing.prepare();
      final id = device == null
          ? await _pairing.beginSenderPairing(text)
          : (await nativeConnection({
                  'op': 'send_direct',
                  'endpoint': endpoint,
                  'receiver_id': device['id'],
                  'fingerprint': device['request_fingerprint'],
                }))['id']
                as int;
      if (!mounted) {
        await _pairing.cancelSenderPairing(id);
        return;
      }
      _requestId = id;
      _pairingStatus = const WindowsSenderPairingStatus(
        state: WindowsSenderPairingState.connecting,
        errorCode: 0,
        receiverFingerprint: '',
      );
      _pairingTimer = Timer.periodic(
        const Duration(milliseconds: 300),
        (_) => unawaited(_pollPairing(id)),
      );
      await _pollPairing(id);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _pollPairing(int id) async {
    if (_polling || !mounted || id != _requestId) return;
    _polling = true;
    try {
      final status = await _pairing.pollSenderPairing(id);
      if (!mounted || id != _requestId) return;
      if (_directRequest && status.state.isActive && _directCode == null) {
        final verification = await nativeConnection({
          'op': 'send_verification',
          'id': id,
        });
        if (!mounted || id != _requestId) return;
        _directCode = verification['code'] as String?;
      }
      setState(() {
        _pairingStatus = status;
        if (status.failureDescription case final description?) {
          _error = '$description (MM-PAIR-${status.errorCode})';
        }
      });
      if (!status.state.isActive) {
        _pairingTimer?.cancel();
        _directCode = null;
        if (_session == null) setState(() => _requestId = null);
      } else if (status.state == WindowsSenderPairingState.approved) {
        _pairingTimer?.cancel();
        if (mounted) setState(() => _directCode = null);
      }
    } catch (error) {
      _pairingTimer?.cancel();
      if (mounted && id == _requestId) {
        setState(() => _error = _describe(error));
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _confirmDirectCode() async {
    final id = _requestId, code = _directCode;
    if (id == null || code == null || _busy) return;
    setState(() => _busy = true);
    try {
      await nativeConnection({'op': 'send_confirm', 'id': id, 'code': code});
      if (mounted && id == _requestId) setState(() => _directConfirmed = true);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _startSharing() async {
    final id = _requestId;
    if (_busy || !_approved || id == null || _session != null) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final sources = await const FlutterWebRtcPeerConnectionFactory()
          .listDesktopCaptureSources();
      if (!mounted) return;
      if (sources.isEmpty) {
        throw StateError('No screens or windows are available for capture.');
      }
      final source = await showDialog<WebRtcDesktopCaptureSource>(
        context: context,
        builder: (context) => AlertDialog(
          title: const Text('Choose what to share'),
          content: SizedBox(
            width: 480,
            child: ListView(
              shrinkWrap: true,
              children: [
                const Text(
                  'Only the source you select will be sent to the approved receiver.',
                ),
                const SizedBox(height: 12),
                for (final source in sources)
                  ListTile(
                    leading: Icon(
                      source.type == WebRtcDesktopCaptureSourceType.screen
                          ? Icons.desktop_windows
                          : Icons.web_asset,
                    ),
                    title: Text(source.name),
                    subtitle: Text(source.type.name),
                    onTap: () => Navigator.pop(context, source),
                  ),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
          ],
        ),
      );
      if (!mounted || source == null || id != _requestId) return;
      final session = WindowsSenderSessionController(pairing: _pairing);
      _session = session;
      _sourceName = source.name;
      _stateSubscription = session.states.listen((state) {
        if (!mounted || !identical(_session, session)) return;
        setState(() {
          _sessionState = state;
          if (state == WindowsSenderSessionState.failed) {
            _error = _describe(
              session.lastError ?? StateError('The media connection failed.'),
            );
          }
        });
        if (state == WindowsSenderSessionState.failed ||
            state == WindowsSenderSessionState.stopped ||
            state == WindowsSenderSessionState.revoked) {
          unawaited(_releaseSession(session));
        }
      });
      _statsSubscription = session.videoStats.listen((stats) {
        if (mounted && identical(_session, session)) {
          setState(() => _stats = stats);
        }
      });
      await session.start(requestId: id, sourceId: source.id);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _releaseSession(WindowsSenderSessionController session) async {
    if (!identical(_session, session)) return;
    _session = null;
    await _stateSubscription?.cancel();
    await _statsSubscription?.cancel();
    _stateSubscription = null;
    _statsSubscription = null;
    try {
      await session.dispose();
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    }
    if (mounted) {
      setState(() {
        _requestId = null;
        _pairingStatus = null;
      });
    }
  }

  Future<void> _disconnect() async {
    if (_busy) return;
    setState(() => _busy = true);
    _pairingTimer?.cancel();
    final id = _requestId;
    final session = _session;
    try {
      if (session != null) await _releaseSession(session);
      if (id != null) await _pairing.cancelSenderPairing(id);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _requestId = null;
          _pairingStatus = null;
          _sessionState = WindowsSenderSessionState.stopped;
          _directCode = null;
          _directConfirmed = false;
        });
      }
    }
  }

  String _describe(Object error) => error is PlatformException
      ? '${error.message ?? 'The native connection failed.'} (${error.code})'
      : error.toString();

  Future<void> _copyDiagnostics() => Clipboard.setData(
    ClipboardData(
      text:
          'MirrorMesh Windows 0.3.0\nStatus: $_status\nPairing code: ${_pairingStatus?.errorCode ?? 0}\nFrames encoded: ${_stats?.framesEncoded ?? 0}\nBytes sent: ${_stats?.bytesSent ?? 0}\nCodec: ${_stats?.codecMimeType ?? 'unknown'}\nEncoder: ${_stats?.encoderImplementation ?? 'unknown'}\nError: ${_error ?? 'none'}\n',
    ),
  );

  @override
  void dispose() {
    _pairingTimer?.cancel();
    _invitation.dispose();
    unawaited(_stateSubscription?.cancel());
    unawaited(_statsSubscription?.cancel());
    final session = _session;
    if (session != null) {
      unawaited(session.dispose());
    } else if (_requestId case final id?) {
      unawaited(_pairing.cancelSenderPairing(id));
    }
    super.dispose();
  }
}
