import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/synthetic_video_test_pattern.dart';

void main() {
  testWidgets('synthetic pattern produces changing repaint frames', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: SyntheticVideoTestPattern())),
    );

    final canvas = find.byKey(const Key('synthetic-video-test-pattern-canvas'));
    final initialPainter = tester.widget<CustomPaint>(canvas).painter!;

    await tester.pump(const Duration(milliseconds: 250));

    final nextPainter = tester.widget<CustomPaint>(canvas).painter!;
    expect(nextPainter.shouldRepaint(initialPainter), isTrue);
  });
}
