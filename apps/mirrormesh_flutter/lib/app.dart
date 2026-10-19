import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'android_pairing_bootstrap.dart';
import 'annotation_geometry.dart';
import 'annotation_overlay.dart';
import 'ios_pairing_bootstrap.dart';
import 'rust_session_status.dart';

enum _DemoReceiverState { undiscovered, discovered, pairing }

const _demoAnnotationColors = <({String id, String label, Color color})>[
  (id: 'teal', label: 'Teal', color: Color(0xFF61D7C5)),
  (id: 'amber', label: 'Amber', color: Color(0xFFFFC857)),
  (id: 'coral', label: 'Coral', color: Color(0xFFFF7A90)),
  (id: 'white', label: 'White', color: Color(0xFFF5F7FA)),
];

const _demoAnnotationWidths = <({String id, String label, double width})>[
  (id: 'fine', label: 'Fine', width: 2),
  (id: 'medium', label: 'Medium', width: 4),
  (id: 'bold', label: 'Bold', width: 8),
];

class MirrorMeshApp extends StatelessWidget {
  const MirrorMeshApp({super.key, this.rustSessionStatus});

  final RustSessionStatus? rustSessionStatus;

  @override
  Widget build(BuildContext context) {
    final colorScheme = ColorScheme.fromSeed(
      seedColor: const Color(0xFF61D7C5),
      brightness: Brightness.dark,
      surface: const Color(0xFF111923),
    );

    return MaterialApp(
      title: 'MirrorMesh',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: colorScheme,
        scaffoldBackgroundColor: const Color(0xFF0A1017),
        cardTheme: const CardThemeData(
          color: Color(0xFF111923),
          margin: EdgeInsets.zero,
        ),
        inputDecorationTheme: const InputDecorationTheme(
          border: OutlineInputBorder(),
        ),
      ),
      home: MirrorMeshHome(rustSessionStatus: rustSessionStatus),
    );
  }
}

class MirrorMeshHome extends StatefulWidget {
  const MirrorMeshHome({super.key, this.rustSessionStatus});

  final RustSessionStatus? rustSessionStatus;

  @override
  State<MirrorMeshHome> createState() => _MirrorMeshHomeState();
}

class _MirrorMeshHomeState extends State<MirrorMeshHome> {
  int _selectedIndex = 0;
  _DemoReceiverState _demoReceiverState = _DemoReceiverState.undiscovered;
  bool _annotationEnabled = false;
  bool _pairingRuntimePreparing = false;
  String? _pairingRuntimeDeviceId;
  String? _pairingRuntimeError;
  List<AnnotationStroke> _demoStrokes = [];
  Color _demoStrokeColor = const Color(0xFF61D7C5);
  double _demoStrokeWidth = 4;

  static const _destinations = [
    NavigationDestination(
      icon: Icon(Icons.cast_connected_outlined),
      selectedIcon: Icon(Icons.cast_connected),
      label: 'Connect',
    ),
    NavigationDestination(
      icon: Icon(Icons.monitor_outlined),
      selectedIcon: Icon(Icons.monitor),
      label: 'Session',
    ),
    NavigationDestination(
      icon: Icon(Icons.devices_outlined),
      selectedIcon: Icon(Icons.devices),
      label: 'Devices',
    ),
    NavigationDestination(
      icon: Icon(Icons.tune_outlined),
      selectedIcon: Icon(Icons.tune),
      label: 'Settings',
    ),
  ];

  bool get _demoPairing => _demoReceiverState == _DemoReceiverState.pairing;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final useRail = constraints.maxWidth >= 760;
        final currentPage = switch (_selectedIndex) {
          0 => _ConnectPage(
            demoReceiverState: _demoReceiverState,
            onScanDemo: _scanDemoReceiver,
          ),
          1 => _SessionPage(
            demoPairing: _demoPairing,
            annotationEnabled: _annotationEnabled,
            strokes: _demoStrokes,
            annotationColor: _demoStrokeColor,
            annotationStrokeWidth: _demoStrokeWidth,
            onStopDemo: _disconnectDemoReceiver,
            onToggleAnnotation: _toggleAnnotation,
            onStrokePoint: _addDemoStrokePoint,
            onUndo: _undoDemoStroke,
            onClear: _clearDemoStrokes,
            onSelectAnnotationColor: _selectDemoStrokeColor,
            onSelectAnnotationWidth: _selectDemoStrokeWidth,
          ),
          2 => _DevicesPage(
            demoReceiverState: _demoReceiverState,
            onConnectDemo: _connectDemoReceiver,
            pairingRuntimePreparing: _pairingRuntimePreparing,
            localDeviceId: _pairingRuntimeDeviceId,
            pairingRuntimeError: _pairingRuntimeError,
            onPreparePairing: _preparePairingRuntime,
          ),
          _ => _SettingsPage(rustSessionStatus: widget.rustSessionStatus),
        };
        final page = SafeArea(
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 1000),
              child: currentPage,
            ),
          ),
        );
        final pageWithSessionShortcuts = _selectedIndex == 1
            ? Focus(
                autofocus: true,
                onKeyEvent: _handleSessionKeyEvent,
                child: page,
              )
            : page;

        return Scaffold(
          body: useRail
              ? Row(
                  children: [
                    NavigationRail(
                      selectedIndex: _selectedIndex,
                      onDestinationSelected: _selectPage,
                      labelType: NavigationRailLabelType.all,
                      destinations: [
                        for (final destination in _destinations)
                          NavigationRailDestination(
                            icon: destination.icon,
                            selectedIcon: destination.selectedIcon,
                            label: Text(destination.label),
                          ),
                      ],
                    ),
                    const VerticalDivider(width: 1),
                    Expanded(child: pageWithSessionShortcuts),
                  ],
                )
              : pageWithSessionShortcuts,
          bottomNavigationBar: useRail
              ? null
              : NavigationBar(
                  selectedIndex: _selectedIndex,
                  onDestinationSelected: _selectPage,
                  destinations: _destinations,
                ),
        );
      },
    );
  }

  void _selectPage(int index) {
    setState(() => _selectedIndex = index);
  }

  KeyEventResult _handleSessionKeyEvent(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent || !_demoPairing) {
      return KeyEventResult.ignored;
    }

    final keyboard = HardwareKeyboard.instance;
    if (event.logicalKey == LogicalKeyboardKey.keyP &&
        !keyboard.isControlPressed &&
        !keyboard.isMetaPressed &&
        !keyboard.isAltPressed) {
      _toggleAnnotation();
      return KeyEventResult.handled;
    }

    if (event.logicalKey == LogicalKeyboardKey.keyZ &&
        (keyboard.isControlPressed || keyboard.isMetaPressed)) {
      _undoDemoStroke();
      return KeyEventResult.handled;
    }

    return KeyEventResult.ignored;
  }

  void _scanDemoReceiver() {
    final status = widget.rustSessionStatus;
    final nextState = status?.isAvailable == true
        ? status!.discoverDemoReceiver()
        : 'Discovered';
    if (nextState != 'Discovered') {
      _showDemoTransitionError();
      return;
    }
    setState(() => _demoReceiverState = _DemoReceiverState.discovered);
  }

  void _connectDemoReceiver() {
    if (_demoReceiverState != _DemoReceiverState.discovered) return;
    final status = widget.rustSessionStatus;
    final nextState = status?.isAvailable == true
        ? status!.beginDemoPairing()
        : 'Pairing';
    if (nextState != 'Pairing') {
      _showDemoTransitionError();
      return;
    }
    setState(() => _demoReceiverState = _DemoReceiverState.pairing);
  }

  Future<void> _preparePairingRuntime() async {
    if (_pairingRuntimePreparing || _pairingRuntimeDeviceId != null) return;
    setState(() {
      _pairingRuntimePreparing = true;
      _pairingRuntimeError = null;
    });
    try {
      final deviceId = switch (Platform.operatingSystem) {
        'android' => await const AndroidPairingBootstrap().prepare(),
        'ios' => await const IosPairingBootstrap().prepare(),
        _ => throw PlatformException(
          code: 'pairing_runtime_unsupported',
          message: 'This platform cannot prepare local pairing support.',
        ),
      };
      if (!mounted) return;
      setState(() => _pairingRuntimeDeviceId = deviceId);
    } on PlatformException {
      if (!mounted) return;
      setState(
        () => _pairingRuntimeError =
            '${Platform.operatingSystem} could not prepare local pairing support. Try again later.',
      );
    } on MissingPluginException {
      if (!mounted) return;
      setState(
        () => _pairingRuntimeError =
            'The ${Platform.operatingSystem} pairing service is unavailable in this build.',
      );
    } finally {
      if (mounted) setState(() => _pairingRuntimePreparing = false);
    }
  }

  void _disconnectDemoReceiver() {
    if (!_demoPairing) return;
    final status = widget.rustSessionStatus;
    final nextState = status?.isAvailable == true
        ? status!.disconnectDemoReceiver()
        : 'Stopped';
    if (nextState != 'Stopped') {
      _showDemoTransitionError();
      return;
    }
    setState(() {
      _demoReceiverState = _DemoReceiverState.undiscovered;
      _annotationEnabled = false;
      _demoStrokes = [];
    });
  }

  void _showDemoTransitionError() {
    ScaffoldMessenger.maybeOf(context)?.showSnackBar(
      const SnackBar(
        content: Text('The local demo session could not change state.'),
      ),
    );
  }

  void _toggleAnnotation() {
    if (!_demoPairing) return;
    setState(() => _annotationEnabled = !_annotationEnabled);
  }

  void _selectDemoStrokeColor(Color color) {
    setState(() => _demoStrokeColor = color);
  }

  void _selectDemoStrokeWidth(double width) {
    setState(() => _demoStrokeWidth = width);
  }

  void _addDemoStrokePoint(
    Offset localPosition,
    AnnotationCoordinateMapper mapper, {
    required bool startsStroke,
  }) {
    if (!_demoPairing || !_annotationEnabled) return;
    final point = mapper.toVideoPoint(localPosition);
    if (point == null) return;
    setState(() {
      if (startsStroke || _demoStrokes.isEmpty) {
        _demoStrokes = [
          ..._demoStrokes,
          AnnotationStroke(
            points: [point],
            color: _demoStrokeColor,
            strokeWidth: _demoStrokeWidth,
          ),
        ];
      } else {
        final currentStroke = _demoStrokes.last;
        _demoStrokes = [
          ..._demoStrokes.take(_demoStrokes.length - 1),
          AnnotationStroke(
            points: [...currentStroke.points, point],
            color: currentStroke.color,
            strokeWidth: currentStroke.strokeWidth,
          ),
        ];
      }
    });
  }

  void _undoDemoStroke() {
    if (_demoStrokes.isEmpty) return;
    setState(() => _demoStrokes = [..._demoStrokes]..removeLast());
  }

  void _clearDemoStrokes() {
    if (_demoStrokes.isEmpty) return;
    setState(() => _demoStrokes = []);
  }
}

class _ConnectPage extends StatelessWidget {
  const _ConnectPage({
    required this.demoReceiverState,
    required this.onScanDemo,
  });

  final _DemoReceiverState demoReceiverState;
  final VoidCallback onScanDemo;

  @override
  Widget build(BuildContext context) {
    final demoAvailable = demoReceiverState != _DemoReceiverState.undiscovered;
    final demoPairing = demoReceiverState == _DemoReceiverState.pairing;

    return ListView(
      padding: const EdgeInsets.all(28),
      children: [
        const _PageHeading(
          eyebrow: 'MIRRORMESH',
          title: 'Connect devices',
          subtitle: 'Private screen sharing starts with an explicit pairing.',
        ),
        const SizedBox(height: 24),
        const _StatusCard(
          icon: Icons.hourglass_top_rounded,
          title: 'Pairing service is not connected',
          message: 'This build has no signaling transport yet. A connection cannot be started until the service and identity checks are available.',
        ),
        const SizedBox(height: 20),
        const TextField(
          enabled: false,
          decoration: InputDecoration(
            labelText: 'Pairing code',
            hintText: 'Available when pairing service is ready',
            prefixIcon: Icon(Icons.key_outlined),
          ),
        ),
        const SizedBox(height: 14),
        FilledButton.icon(
          key: const Key('connect-button'),
          onPressed: null,
          icon: const Icon(Icons.link),
          label: const Text('Connect'),
        ),
        const SizedBox(height: 12),
        OutlinedButton.icon(
          key: const Key('scan-demo-button'),
          onPressed: demoPairing ? null : onScanDemo,
          icon: const Icon(Icons.science_outlined),
          label: Text(
            demoPairing
                ? 'Demo pairing in progress'
                : demoAvailable
                ? 'Demo receiver available'
                : 'Scan local demo devices',
          ),
        ),
        const SizedBox(height: 10),
        const Text(
          'The demo tracks discovery and pairing state locally. It does not verify a peer, use the network, or start capture or video.',
        ),
        const SizedBox(height: 18),
        const _PrivacyNote(
          text: 'Screen capture will only be requested through the operating system after pairing and your confirmation.',
        ),
      ],
    );
  }
}

class _SessionPage extends StatelessWidget {
  const _SessionPage({
    required this.demoPairing,
    required this.annotationEnabled,
    required this.strokes,
    required this.annotationColor,
    required this.annotationStrokeWidth,
    required this.onStopDemo,
    required this.onToggleAnnotation,
    required this.onStrokePoint,
    required this.onUndo,
    required this.onClear,
    required this.onSelectAnnotationColor,
    required this.onSelectAnnotationWidth,
  });

  final bool demoPairing;
  final bool annotationEnabled;
  final List<AnnotationStroke> strokes;
  final Color annotationColor;
  final double annotationStrokeWidth;
  final VoidCallback onStopDemo;
  final VoidCallback onToggleAnnotation;
  final void Function(
    Offset position,
    AnnotationCoordinateMapper mapper, {
    required bool startsStroke,
  })
  onStrokePoint;
  final VoidCallback onUndo;
  final VoidCallback onClear;
  final ValueChanged<Color> onSelectAnnotationColor;
  final ValueChanged<double> onSelectAnnotationWidth;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(28),
      children: [
        _PageHeading(
          eyebrow: 'CURRENT SESSION',
          title: demoPairing ? 'Demo pairing preview' : 'No active session',
          subtitle: demoPairing
              ? 'Pairing state only; no peer is verified and no media stream is active.'
              : 'Session controls appear here after the local demo pairing starts.',
        ),
        const SizedBox(height: 24),
        if (demoPairing) ...[
          const _StatusCard(
            icon: Icons.science_outlined,
            title: 'Demo pairing active',
            message: 'Local simulation only; no peer is verified, no network or media stream is active, and no capture permission was requested.',
          ),
          const SizedBox(height: 20),
          _DemoPreview(
            strokes: strokes,
            annotationEnabled: annotationEnabled,
            onStrokePoint: onStrokePoint,
          ),
          const SizedBox(height: 16),
          Text('Pen color', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              for (final option in _demoAnnotationColors)
                ChoiceChip(
                  key: Key('annotation-color-${option.id}'),
                  label: Text(option.label),
                  avatar: CircleAvatar(
                    radius: 8,
                    backgroundColor: option.color,
                  ),
                  selected: annotationColor == option.color,
                  onSelected: (selected) {
                    if (selected) onSelectAnnotationColor(option.color);
                  },
                ),
            ],
          ),
          const SizedBox(height: 12),
          Text('Pen width', style: Theme.of(context).textTheme.titleSmall),
          const SizedBox(height: 8),
          SegmentedButton<double>(
            segments: [
              for (final option in _demoAnnotationWidths)
                ButtonSegment<double>(
                  value: option.width,
                  label: KeyedSubtree(
                    key: Key('annotation-width-${option.id}'),
                    child: Text(option.label),
                  ),
                  enabled: demoPairing,
                ),
            ],
            selected: {annotationStrokeWidth},
            showSelectedIcon: false,
            onSelectionChanged: demoPairing
                ? (selection) => onSelectAnnotationWidth(selection.single)
                : null,
          ),
          const SizedBox(height: 12),
          Text(
            '${strokes.length} local annotation ${strokes.length == 1 ? 'stroke' : 'strokes'}',
            key: const Key('annotation-count'),
          ),
          const SizedBox(height: 10),
          Wrap(
            spacing: 10,
            runSpacing: 10,
            children: [
              OutlinedButton.icon(
                key: const Key('toggle-annotation-button'),
                onPressed: onToggleAnnotation,
                icon: Icon(
                  annotationEnabled
                      ? Icons.edit_off_outlined
                      : Icons.edit_outlined,
                ),
                label: Text(
                  annotationEnabled ? 'Turn pen off (P)' : 'Draw locally (P)',
                ),
              ),
              OutlinedButton.icon(
                key: const Key('undo-annotation-button'),
                onPressed: strokes.isEmpty ? null : onUndo,
                icon: const Icon(Icons.undo),
                label: const Text('Undo (Ctrl/⌘+Z)'),
              ),
              OutlinedButton.icon(
                key: const Key('clear-annotations-button'),
                onPressed: strokes.isEmpty ? null : onClear,
                icon: const Icon(Icons.delete_sweep_outlined),
                label: const Text('Clear'),
              ),
              OutlinedButton.icon(
                key: const Key('screenshot-button'),
                onPressed: null,
                icon: const Icon(Icons.screenshot_outlined),
                label: const Text('Screenshot'),
              ),
            ],
          ),
          const SizedBox(height: 8),
          const Text(
            'Screenshot export is unavailable because this preview has no received video frame. Nothing is saved.',
          ),
          const SizedBox(height: 16),
          const _StatusCard(
            icon: Icons.query_stats_outlined,
            title: 'Live diagnostics unavailable',
            message: 'FPS, bitrate, and latency require an active media transport; this demo reports no measured values.',
          ),
        ] else ...[
          const _StatusCard(
            icon: Icons.monitor_heart_outlined,
            title: 'Ready when you are',
            message: 'No capture permission has been requested. Nothing is being recorded or streamed.',
          ),
        ],
        const SizedBox(height: 20),
        OutlinedButton.icon(
          key: const Key('stop-session-button'),
          onPressed: demoPairing ? onStopDemo : null,
          icon: const Icon(Icons.stop_circle_outlined),
          label: Text(demoPairing ? 'End demo pairing' : 'Stop session'),
        ),
      ],
    );
  }
}

class _DemoPreview extends StatelessWidget {
  const _DemoPreview({
    required this.strokes,
    required this.annotationEnabled,
    required this.onStrokePoint,
  });

  final List<AnnotationStroke> strokes;
  final bool annotationEnabled;
  final void Function(
    Offset position,
    AnnotationCoordinateMapper mapper, {
    required bool startsStroke,
  })
  onStrokePoint;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Preview placeholder',
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        AspectRatio(
          aspectRatio: 16 / 9,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final size = Size(constraints.maxWidth, constraints.maxHeight);
              final mapper = AnnotationCoordinateMapper(
                viewportSize: size,
                videoSize: const Size(16, 9),
              );
              return GestureDetector(
                key: const Key('demo-preview'),
                behavior: HitTestBehavior.opaque,
                onPanStart: annotationEnabled
                    ? (details) => onStrokePoint(
                        details.localPosition,
                        mapper,
                        startsStroke: true,
                      )
                    : null,
                onPanUpdate: annotationEnabled
                    ? (details) => onStrokePoint(
                        details.localPosition,
                        mapper,
                        startsStroke: false,
                      )
                    : null,
                child: CustomPaint(
                  key: const Key('annotation-overlay'),
                  painter: const _DemoPreviewBackgroundPainter(),
                  foregroundPainter: AnnotationOverlayPainter(
                    strokes: strokes,
                    mapper: mapper,
                  ),
                  child: Center(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Icon(
                          Icons.smart_display_outlined,
                          size: 42,
                          color: Theme.of(context).colorScheme.primary,
                        ),
                        const SizedBox(height: 8),
                        const Text('No live frame received'),
                        Text(
                          annotationEnabled
                              ? 'Draw on this local placeholder'
                              : 'Enable Draw locally to try annotations',
                          style: Theme.of(context).textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}

class _DemoPreviewBackgroundPainter extends CustomPainter {
  const _DemoPreviewBackgroundPainter();

  @override
  void paint(Canvas canvas, Size size) {
    canvas.save();
    canvas.clipRect(Offset.zero & size);
    canvas.drawRect(
      Offset.zero & size,
      Paint()..color = const Color(0xFF172531),
    );
    final gridPaint = Paint()
      ..color = const Color(0xFF243846)
      ..strokeWidth = 1;
    for (var fraction = 0.2; fraction < 1; fraction += 0.2) {
      canvas.drawLine(
        Offset(size.width * fraction, 0),
        Offset(size.width * fraction, size.height),
        gridPaint,
      );
      canvas.drawLine(
        Offset(0, size.height * fraction),
        Offset(size.width, size.height * fraction),
        gridPaint,
      );
    }

    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _DemoPreviewBackgroundPainter oldDelegate) =>
      false;
}

class _DevicesPage extends StatelessWidget {
  const _DevicesPage({
    required this.demoReceiverState,
    required this.onConnectDemo,
    required this.pairingRuntimePreparing,
    required this.localDeviceId,
    required this.pairingRuntimeError,
    required this.onPreparePairing,
  });

  final _DemoReceiverState demoReceiverState;
  final VoidCallback onConnectDemo;
  final bool pairingRuntimePreparing;
  final String? localDeviceId;
  final String? pairingRuntimeError;
  final VoidCallback onPreparePairing;

  @override
  Widget build(BuildContext context) {
    final demoDiscovered = demoReceiverState != _DemoReceiverState.undiscovered;
    final demoPairing = demoReceiverState == _DemoReceiverState.pairing;

    return ListView(
      padding: const EdgeInsets.all(28),
      children: [
        _PageHeading(
          eyebrow: 'YOUR DEVICES',
          title: demoDiscovered ? 'Devices' : 'No discovered devices',
          subtitle: demoDiscovered
              ? 'The listed receiver is a local UI simulation.'
              : 'Run the local demo scan to preview the device flow.',
        ),
        const SizedBox(height: 24),
        if (demoDiscovered) ...[
          Card(
            child: Padding(
              padding: const EdgeInsets.all(8),
              child: Column(
                children: [
                  ListTile(
                    leading: const Icon(Icons.desktop_windows_outlined),
                    title: const Text('Demo receiver'),
                    subtitle: Text(
                      demoPairing
                          ? 'Pairing state only; not verified or connected on a network.'
                          : 'Local simulation only; not discovered on a network.',
                    ),
                    trailing: Icon(
                      demoPairing
                          ? Icons.check_circle_outline
                          : Icons.science_outlined,
                    ),
                  ),
                  Align(
                    alignment: Alignment.centerRight,
                    child: FilledButton.icon(
                      key: const Key('connect-demo-button'),
                      onPressed: demoPairing ? null : onConnectDemo,
                      icon: Icon(
                        demoPairing ? Icons.hourglass_top : Icons.link_outlined,
                      ),
                      label: Text(
                        demoPairing
                            ? 'Pairing in progress'
                            : 'Start demo pairing',
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
        ],
        const _StatusCard(
          icon: Icons.devices_other_outlined,
          title: 'Live pairing is not available yet',
          message: 'Network discovery, identity verification, trusted-device storage, and revocation are not implemented in this shell.',
        ),
        if (Platform.isAndroid || Platform.isIOS) ...[
          const SizedBox(height: 16),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text(
                    'Prepare this device for pairing',
                    style: TextStyle(fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'This opens the local ${Platform.isIOS ? 'iOS' : 'Android'} identity and trusted-peer database. It does not contact another device, grant trust, or start screen capture.',
                  ),
                  const SizedBox(height: 12),
                  FilledButton.icon(
                    key: Key(
                      Platform.isIOS
                          ? 'prepare-ios-pairing-button'
                          : 'prepare-android-pairing-button',
                    ),
                    onPressed: pairingRuntimePreparing || localDeviceId != null
                        ? null
                        : onPreparePairing,
                    icon: pairingRuntimePreparing
                        ? const SizedBox.square(
                            dimension: 18,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Icon(Icons.shield_outlined),
                    label: Text(
                      localDeviceId != null
                          ? 'Pairing support ready'
                          : pairingRuntimePreparing
                          ? 'Preparing…'
                          : 'Prepare pairing support',
                    ),
                  ),
                  if (localDeviceId != null) ...[
                    const SizedBox(height: 8),
                    SelectableText('Local device ID: $localDeviceId'),
                  ],
                  if (pairingRuntimeError != null) ...[
                    const SizedBox(height: 8),
                    Text(
                      pairingRuntimeError!,
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.error,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        ],
      ],
    );
  }
}

class _SettingsPage extends StatelessWidget {
  const _SettingsPage({required this.rustSessionStatus});

  final RustSessionStatus? rustSessionStatus;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(28),
      children: [
        const _PageHeading(
          eyebrow: 'PREFERENCES',
          title: 'Settings',
          subtitle: 'Privacy and platform status.',
        ),
        const SizedBox(height: 24),
        const _StatusCard(
          icon: Icons.shield_outlined,
          title: 'Privacy by default',
          message: 'Pairing requires approval. Capture permissions are requested through native operating system prompts.',
        ),
        const SizedBox(height: 16),
        const _StatusCard(
          icon: Icons.build_outlined,
          title: 'Platform adapters are in progress',
          message: 'This shell does not capture, encode, transmit, or render video. Those capabilities will be supplied by reviewed native adapters.',
        ),
        const SizedBox(height: 16),
        const _StatusCard(
          icon: Icons.query_stats_outlined,
          title: 'Diagnostics are not connected',
          message: 'This build has no transport or live diagnostic source. It does not display estimated performance as measured data.',
        ),
        const SizedBox(height: 16),
        _RustSessionCard(status: rustSessionStatus),
      ],
    );
  }
}

class _RustSessionCard extends StatelessWidget {
  const _RustSessionCard({required this.status});

  final RustSessionStatus? status;

  @override
  Widget build(BuildContext context) {
    final session = status;
    if (session == null || !session.isAvailable) {
      return const _StatusCard(
        icon: Icons.memory_outlined,
        title: 'Rust state bridge unavailable',
        message: 'The native library did not load. The local demo remains separate; pairing and capture are unavailable.',
      );
    }

    return StreamBuilder<String>(
      stream: session.events,
      initialData: session.currentState(),
      builder: (context, snapshot) {
        final state = snapshot.data ?? session.initialState;
        return Card(
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Rust core state bridge',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 6),
                const Text(
                  'Local domain state only. This does not indicate pairing, capture, or video.',
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    Expanded(child: Text('Session state: $state')),
                    OutlinedButton(
                      key: const Key('stop-core-session-button'),
                      onPressed: state == 'Idle' || state == 'Stopped'
                          ? null
                          : session.stopSession,
                      child: const Text('Stop core session'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _PageHeading extends StatelessWidget {
  const _PageHeading({
    required this.eyebrow,
    required this.title,
    required this.subtitle,
  });

  final String eyebrow;
  final String title;
  final String subtitle;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          eyebrow,
          style: theme.textTheme.labelLarge?.copyWith(
            color: theme.colorScheme.primary,
            letterSpacing: 1.4,
          ),
        ),
        const SizedBox(height: 8),
        Text(title, style: theme.textTheme.headlineMedium),
        const SizedBox(height: 8),
        Text(subtitle, style: theme.textTheme.bodyLarge),
      ],
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({
    required this.icon,
    required this.title,
    required this.message,
  });

  final IconData icon;
  final String title;
  final String message;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, color: Theme.of(context).colorScheme.primary),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(title, style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 6),
                  Text(message),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _PrivacyNote extends StatelessWidget {
  const _PrivacyNote({required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Icon(Icons.lock_outline, size: 18),
        const SizedBox(width: 10),
        Expanded(child: Text(text)),
      ],
    );
  }
}
