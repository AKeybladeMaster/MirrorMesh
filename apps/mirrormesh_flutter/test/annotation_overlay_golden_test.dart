import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/annotation_geometry.dart';
import 'package:mirrormesh_flutter/annotation_overlay.dart';

void main() {
  testWidgets('draws normalized annotations over a known-color frame', (
    tester,
  ) async {
    const viewportSize = Size(128, 96);
    final mapper = AnnotationCoordinateMapper(
      viewportSize: viewportSize,
      videoSize: const Size(4, 3),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: RepaintBoundary(
              key: const Key('annotation-golden-boundary'),
              child: SizedBox.fromSize(
                size: viewportSize,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    const ColoredBox(color: Color(0xFF102030)),
                    CustomPaint(
                      painter: AnnotationOverlayPainter(
                        strokes: [
                          AnnotationStroke(
                            points: const [
                              Offset(0.1, 0.3),
                              Offset(0.5, 0.5),
                              Offset(0.9, 0.3),
                            ],
                            color: const Color(0xFFFFC857),
                            strokeWidth: 7,
                          ),
                          AnnotationStroke(
                            points: const [Offset(0.5, 0.8)],
                            color: const Color(0xFFFFC857),
                            strokeWidth: 7,
                          ),
                        ],
                        mapper: mapper,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );

    await expectLater(
      find.byKey(const Key('annotation-golden-boundary')),
      matchesGoldenFile('goldens/annotation_overlay.png'),
    );
  });
}
