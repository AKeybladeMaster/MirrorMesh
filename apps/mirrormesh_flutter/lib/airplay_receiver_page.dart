import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'receiver_window.dart';

/// A Windows-native receiver for the iOS system Screen Mirroring picker.
/// This channel carries status, consent and a window rectangle, never video/audio.
class AirPlayReceiverPage extends StatefulWidget {
  const AirPlayReceiverPage({super.key});
  @override
  State<AirPlayReceiverPage> createState() => _AirPlayReceiverPageState();
}

class _AirPlayReceiverPageState extends State<AirPlayReceiverPage> {
  final _stage = GlobalKey();
  final _pinPanel = GlobalKey(), _approvalPanel = GlobalKey();
  Timer? _timer;
  bool _running = false, _ready = false, _video = false, _busy = false;
  bool _polling = false, _fullscreen = false, _closing = false;
  int _request = 0;
  int _generation = 0;
  String _pin = '', _error = '';
  static const _channel = ReceiverWindow.channel;

  Future<void> _start() async {
    if (_busy || _running || _closing) return;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _error = '';
    });
    try {
      await _channel.invokeMethod<void>('airplay_start');
      if (_closing || !mounted || generation != _generation) {
        await _channel.invokeMethod<void>('airplay_stop');
        return;
      }
      setState(() => _running = true);
      _timer = Timer.periodic(
        const Duration(milliseconds: 250),
        (_) => unawaited(_poll()),
      );
      await _poll();
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _poll() async {
    if (_polling || _closing || !_running) return;
    _polling = true;
    final generation = _generation;
    try {
      final state = await _channel.invokeMapMethod<String, dynamic>(
        'airplay_poll',
      );
      if (!mounted || _closing || generation != _generation || state == null) {
        return;
      }
      final previousPin = _pin, previousRequest = _request;
      setState(() {
        _running = state['running'] == true;
        _ready = state['ready'] == true;
        _video = state['video'] == true;
        _fullscreen = state['fullscreen'] == true;
        _pin = state['pin'] as String? ?? '';
        _request = state['request'] as int? ?? 0;
        _error = state['error'] as String? ?? '';
      });
      final reveal = _pin.isNotEmpty && _pin != previousPin
          ? _pinPanel
          : _request > 0 && _request != previousRequest
          ? _approvalPanel
          : null;
      if (reveal != null) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted || _closing || generation != _generation) return;
          final panelContext = reveal.currentContext;
          if (panelContext != null) {
            unawaited(Scrollable.ensureVisible(panelContext, alignment: 0.2));
          }
        });
      }
      if (!_running) _timer?.cancel();
    } catch (error) {
      _timer?.cancel();
      if (generation == _generation) {
        try {
          await _channel.invokeMethod<void>('airplay_stop');
        } catch (_) {}
        if (mounted) {
          setState(() {
            _running = _video = false;
            _pin = '';
            _request = 0;
            _error = _describe(error);
          });
        }
      }
    } finally {
      _polling = false;
    }
  }

  Future<void> _decide(bool allow) async {
    final request = _request;
    if (request <= 0 || _busy) return;
    setState(() => _busy = true);
    try {
      await _channel.invokeMethod<void>('airplay_decide', {
        'request': request,
        'allow': allow,
      });
      if (mounted && request == _request) setState(() => _request = 0);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _stop() async {
    ++_generation;
    _timer?.cancel();
    try {
      await _channel.invokeMethod<void>('airplay_stop');
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    }
    if (mounted) {
      setState(() {
        _running = _ready = _video = _fullscreen = false;
        _pin = '';
        _request = 0;
      });
    }
  }

  Future<void> _toggleFullscreen() async {
    try {
      final next = !_fullscreen;
      await ReceiverWindow.setFullscreen(next);
      if (mounted) setState(() => _fullscreen = next);
    } catch (error) {
      if (mounted) setState(() => _error = _describe(error));
    }
  }

  void _layoutStage() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _closing || !_running) return;
      final box = _stage.currentContext?.findRenderObject() as RenderBox?;
      if (box == null || !box.hasSize || box.size.isEmpty) return;
      final offset = box.localToGlobal(Offset.zero);
      final scale = View.of(context).devicePixelRatio;
      unawaited(
        _channel
            .invokeMethod<void>('airplay_bounds', {
              'x': offset.dx * scale,
              'y': offset.dy * scale,
              'width': box.size.width * scale,
              'height': box.size.height * scale,
            })
            .catchError((Object error) {
              if (mounted && !_closing) {
                setState(() => _error = _describe(error));
              }
            }),
      );
    });
  }

  String _describe(Object error) => error is PlatformException
      ? error.message ?? 'AirPlay is unavailable.'
      : error.toString();

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: !_busy,
    child: CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.f11): _toggleFullscreen,
        const SingleActivator(LogicalKeyboardKey.escape): () {
          if (_fullscreen) _toggleFullscreen();
        },
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          appBar: AppBar(
            title: const Text('Receive iPhone or iPad · AirPlay preview'),
            actions: [
              IconButton(
                tooltip: _fullscreen
                    ? 'Exit full screen (Esc)'
                    : 'Full screen (F11)',
                onPressed: _running ? _toggleFullscreen : null,
                icon: Icon(
                  _fullscreen ? Icons.fullscreen_exit : Icons.fullscreen,
                ),
              ),
            ],
          ),
          body: SafeArea(
            child: Column(
              children: [
                Flexible(
                  flex: _video ? 1 : 3,
                  child: SingleChildScrollView(
                    child: Padding(
                      padding: const EdgeInsets.all(16),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Wrap(
                            spacing: 20,
                            runSpacing: 8,
                            crossAxisAlignment: WrapCrossAlignment.center,
                            children: [
                              Text(
                                _video
                                    ? 'Receiving your screen'
                                    : _pin.isNotEmpty
                                    ? 'Enter the AirPlay code on your device'
                                    : _ready
                                    ? 'Ready as “MirrorMesh PC”'
                                    : _running
                                    ? 'Starting AirPlay…'
                                    : 'Connect with iOS Screen Mirroring',
                                style: Theme.of(context).textTheme.titleLarge,
                              ),
                              if (_running)
                                FilledButton.tonalIcon(
                                  onPressed: _stop,
                                  icon: const Icon(Icons.stop),
                                  label: const Text('Stop receiving'),
                                )
                              else
                                FilledButton.icon(
                                  onPressed: _busy ? null : _start,
                                  icon: const Icon(Icons.airplay),
                                  label: const Text(
                                    'Start iPhone / iPad receiver',
                                  ),
                                ),
                            ],
                          ),
                          if (!_video)
                            const Padding(
                              padding: EdgeInsets.only(top: 12),
                              child: Text(
                                'Connect both devices to the same network. On iPhone or iPad, open Control Center → Screen Mirroring → MirrorMesh PC. Enter the code shown here, then allow the connection on this PC.',
                              ),
                            ),
                          if (_pin.isNotEmpty)
                            Padding(
                              key: _pinPanel,
                              padding: const EdgeInsets.only(top: 12),
                              child: Wrap(
                                spacing: 16,
                                runSpacing: 8,
                                crossAxisAlignment: WrapCrossAlignment.center,
                                children: [
                                  const Text('AirPlay code'),
                                  Semantics(
                                    liveRegion: true,
                                    child: Container(
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 20,
                                        vertical: 10,
                                      ),
                                      decoration: BoxDecoration(
                                        color: Theme.of(context)
                                            .colorScheme
                                            .primaryContainer,
                                        borderRadius: BorderRadius.circular(12),
                                      ),
                                      child: SelectableText(
                                        _pin,
                                        style: Theme.of(context)
                                            .textTheme
                                            .headlineLarge
                                            ?.copyWith(
                                              color: Theme.of(context)
                                                  .colorScheme
                                                  .onPrimaryContainer,
                                              fontWeight: FontWeight.w700,
                                              letterSpacing: 8,
                                            ),
                                      ),
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          if (_request > 0)
                            Padding(
                              key: _approvalPanel,
                              padding: const EdgeInsets.only(top: 12),
                              child: Wrap(
                                spacing: 12,
                                runSpacing: 8,
                                crossAxisAlignment: WrapCrossAlignment.center,
                                children: [
                                  const Text(
                                    'Allow the device you just selected to share its screen?',
                                  ),
                                  FilledButton(
                                    onPressed: _busy
                                        ? null
                                        : () => _decide(true),
                                    child: const Text('Allow'),
                                  ),
                                  OutlinedButton(
                                    onPressed: _busy
                                        ? null
                                        : () => _decide(false),
                                    child: const Text('Reject'),
                                  ),
                                ],
                              ),
                            ),
                          if (_error.isNotEmpty)
                            Padding(
                              padding: const EdgeInsets.only(top: 8),
                              child: SelectableText(
                                _error,
                                style: TextStyle(
                                  color: Theme.of(context).colorScheme.error,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
                Expanded(
                  flex: _video ? 4 : 1,
                  child: LayoutBuilder(
                    builder: (context, constraints) {
                      _layoutStage();
                      return ColoredBox(
                        key: _stage,
                        color: Colors.black,
                        child: const SizedBox.expand(),
                      );
                    },
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    ),
  );

  @override
  void dispose() {
    ++_generation;
    _closing = true;
    _timer?.cancel();
    unawaited(
      _channel.invokeMethod<void>('airplay_stop').catchError((Object _) {}),
    );
    super.dispose();
  }
}
