import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/annotation_geometry.dart';

void expectOffsetClose(Offset? actual, Offset expected) {
  expect(actual, isNotNull);
  expect(actual!.dx, closeTo(expected.dx, 1e-9));
  expect(actual.dy, closeTo(expected.dy, 1e-9));
}

void main() {
  group('AnnotationCoordinateMapper', () {
    test('maps normalized source coordinates for each quarter turn', () {
      final cases = <({QuarterTurn rotation, Offset expected})>[
        (rotation: QuarterTurn.none, expected: const Offset(40, 30)),
        (rotation: QuarterTurn.clockwise90, expected: const Offset(140, 20)),
        (rotation: QuarterTurn.clockwise180, expected: const Offset(160, 70)),
        (rotation: QuarterTurn.clockwise270, expected: const Offset(60, 80)),
      ];

      for (final testCase in cases) {
        final mapper = AnnotationCoordinateMapper(
          viewportSize: const Size(200, 100),
          videoSize:
              testCase.rotation == QuarterTurn.none ||
                  testCase.rotation == QuarterTurn.clockwise180
              ? const Size(200, 100)
              : const Size(100, 200),
          rotation: testCase.rotation,
        );

        expectOffsetClose(
          mapper.toViewportPoint(const Offset(0.2, 0.3)),
          testCase.expected,
        );
        expectOffsetClose(
          mapper.toVideoPoint(testCase.expected),
          const Offset(0.2, 0.3),
        );
      }
    });

    test('ignores touches in contain letterboxing', () {
      final mapper = AnnotationCoordinateMapper(
        viewportSize: const Size(100, 100),
        videoSize: const Size(160, 90),
      );

      expect(mapper.toVideoPoint(const Offset(50, 0)), isNull);
      expectOffsetClose(
        mapper.toVideoPoint(const Offset(50, 50)),
        const Offset(0.5, 0.5),
      );
    });

    test('maps cover cropping to the visible part of the source', () {
      final mapper = AnnotationCoordinateMapper(
        viewportSize: const Size(100, 100),
        videoSize: const Size(200, 100),
        fit: BoxFit.cover,
      );

      expectOffsetClose(
        mapper.toVideoPoint(const Offset(0, 50)),
        const Offset(0.25, 0.5),
      );
      expectOffsetClose(
        mapper.toVideoPoint(const Offset(50, 50)),
        const Offset(0.5, 0.5),
      );
      expectOffsetClose(
        mapper.toViewportPoint(const Offset(0.25, 0.5)),
        const Offset(0, 50),
      );
    });

    test(
      'preserves source points when the viewport is resized proportionally',
      () {
        final original = AnnotationCoordinateMapper(
          viewportSize: const Size(200, 100),
          videoSize: const Size(160, 90),
        );
        final resized = AnnotationCoordinateMapper(
          viewportSize: const Size(400, 200),
          videoSize: const Size(160, 90),
        );
        const localPoint = Offset(80, 50);

        expectOffsetClose(
          resized.toVideoPoint(const Offset(160, 100)),
          original.toVideoPoint(localPoint)!,
        );
      },
    );

    test(
      'rejects invalid geometry and pointer positions outside the viewport',
      () {
        final invalid = AnnotationCoordinateMapper(
          viewportSize: const Size(0, 100),
          videoSize: const Size(160, 90),
        );
        final valid = AnnotationCoordinateMapper(
          viewportSize: const Size(100, 100),
          videoSize: const Size(160, 90),
        );

        expect(invalid.toVideoPoint(Offset.zero), isNull);
        expect(invalid.toViewportPoint(Offset.zero), isNull);
        expect(valid.toVideoPoint(const Offset(-1, 50)), isNull);
        expect(valid.toVideoPoint(const Offset(double.nan, 50)), isNull);
        expect(valid.toViewportPoint(const Offset(1.1, 0.5)), isNull);
      },
    );
  });
}
