import 'package:flutter/material.dart';

import 'annotation_geometry.dart';

/// An immutable annotation stroke in normalized, unrotated video coordinates.
class AnnotationStroke {
  AnnotationStroke({
    required List<Offset> points,
    required this.color,
    required this.strokeWidth,
  }) : points = List<Offset>.unmodifiable(points) {
    if (!strokeWidth.isFinite || strokeWidth <= 0) {
      throw ArgumentError.value(
        strokeWidth,
        'strokeWidth',
        'must be positive and finite',
      );
    }
  }

  final List<Offset> points;
  final Color color;
  final double strokeWidth;
}

/// Paints normalized, unrotated video strokes over a decoded frame surface.
///
/// The host should place this as a foreground painter and use the same mapper
/// for pointer input and video layout. Pixels are never copied into this class;
/// it receives only vector coordinates and layout metadata.
class AnnotationOverlayPainter extends CustomPainter {
  AnnotationOverlayPainter({required this.strokes, required this.mapper});

  final List<AnnotationStroke> strokes;
  final AnnotationCoordinateMapper mapper;

  @override
  void paint(Canvas canvas, Size size) {
    if (!size.width.isFinite ||
        !size.height.isFinite ||
        size.width <= 0 ||
        size.height <= 0) {
      return;
    }

    canvas.save();
    canvas.clipRect(Offset.zero & size);
    for (final stroke in strokes) {
      if (stroke.points.isEmpty) continue;
      final firstPoint = mapper.toViewportPoint(stroke.points.first);
      if (firstPoint == null) continue;
      final dotPaint = Paint()
        ..color = stroke.color
        ..style = PaintingStyle.fill;
      if (stroke.points.length == 1) {
        canvas.drawCircle(firstPoint, stroke.strokeWidth / 2, dotPaint);
        continue;
      }

      final path = Path()..moveTo(firstPoint.dx, firstPoint.dy);
      var hasInvalidPoint = false;
      for (final point in stroke.points.skip(1)) {
        final viewportPoint = mapper.toViewportPoint(point);
        if (viewportPoint == null) {
          hasInvalidPoint = true;
          break;
        }
        path.lineTo(viewportPoint.dx, viewportPoint.dy);
      }
      if (!hasInvalidPoint) {
        final strokePaint = Paint()
          ..color = stroke.color
          ..strokeWidth = stroke.strokeWidth
          ..strokeCap = StrokeCap.round
          ..strokeJoin = StrokeJoin.round
          ..style = PaintingStyle.stroke;
        canvas.drawPath(path, strokePaint);
      }
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant AnnotationOverlayPainter oldDelegate) =>
      !identical(oldDelegate.strokes, strokes) ||
      oldDelegate.mapper.viewportSize != mapper.viewportSize ||
      oldDelegate.mapper.videoSize != mapper.videoSize ||
      oldDelegate.mapper.fit != mapper.fit ||
      oldDelegate.mapper.rotation != mapper.rotation;
}
