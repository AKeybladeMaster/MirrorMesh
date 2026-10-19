import 'package:flutter/material.dart';

/// Animated synthetic desktop content for the native capture/WebRTC test path.
///
/// The pixels are painted into the Flutter window and captured by the selected
/// native desktop-capture source. This widget does not send frame bytes through
/// Dart or FFI and is not a direct native video-track source.
class SyntheticVideoTestPattern extends StatefulWidget {
  const SyntheticVideoTestPattern({super.key, this.child});

  final Widget? child;

  @override
  State<SyntheticVideoTestPattern> createState() =>
      _SyntheticVideoTestPatternState();
}

class _SyntheticVideoTestPatternState extends State<SyntheticVideoTestPattern>
    with SingleTickerProviderStateMixin {
  late final AnimationController _animation;

  @override
  void initState() {
    super.initState();
    _animation = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 2),
    )..repeat();
  }

  @override
  void dispose() {
    _animation.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _animation,
    child: widget.child,
    builder: (context, child) => CustomPaint(
      key: const Key('synthetic-video-test-pattern-canvas'),
      painter: _SyntheticVideoTestPatternPainter(progress: _animation.value),
      child: child,
    ),
  );
}

class _SyntheticVideoTestPatternPainter extends CustomPainter {
  const _SyntheticVideoTestPatternPainter({required this.progress});

  final double progress;

  static const _bars = <Color>[
    Color(0xFFFFFFFF),
    Color(0xFFFFFF00),
    Color(0xFF00FFFF),
    Color(0xFF00FF00),
    Color(0xFFFF00FF),
    Color(0xFFFF0000),
    Color(0xFF0000FF),
    Color(0xFF000000),
  ];

  @override
  void paint(Canvas canvas, Size size) {
    final barWidth = size.width / _bars.length;
    for (var index = 0; index < _bars.length; index++) {
      canvas.drawRect(
        Rect.fromLTWH(index * barWidth, 0, barWidth, size.height),
        Paint()..color = _bars[index],
      );
    }

    final center = Offset(size.width / 2, size.height / 2);
    canvas.drawCircle(center, size.height * 0.16, Paint()..color = Colors.grey);
    canvas.drawCircle(
      center,
      size.height * 0.13,
      Paint()..color = Colors.white,
    );

    final markerX = size.width * progress;
    canvas.drawLine(
      Offset(markerX, 0),
      Offset(markerX, size.height),
      Paint()
        ..color = Colors.black
        ..strokeWidth = 4,
    );
    canvas.drawCircle(
      Offset(markerX, size.height * 0.82),
      size.height * 0.055,
      Paint()..color = Colors.white,
    );
  }

  @override
  bool shouldRepaint(covariant _SyntheticVideoTestPatternPainter oldDelegate) =>
      progress != oldDelegate.progress;
}
