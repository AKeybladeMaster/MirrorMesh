import 'package:flutter/widgets.dart';

enum QuarterTurn { none, clockwise90, clockwise180, clockwise270 }

/// Maps between widget-local positions and normalized, unrotated video points.
///
/// The video renderer and annotation overlay must use the same fit and quarter
/// turn. For `BoxFit.contain`, touches in the letterboxed area are rejected.
/// For `BoxFit.cover`, source points outside the visible crop map outside the
/// viewport so a clipped canvas can preserve continuous strokes at the edges.
class AnnotationCoordinateMapper {
  AnnotationCoordinateMapper({
    required this.viewportSize,
    required this.videoSize,
    this.fit = BoxFit.contain,
    this.rotation = QuarterTurn.none,
  }) : _orientedVideoSize = _rotateSize(videoSize, rotation) {
    if (!_hasArea(viewportSize) || !_hasArea(_orientedVideoSize)) {
      _sourceRect = null;
      _destinationRect = null;
      return;
    }

    final fitted = applyBoxFit(fit, _orientedVideoSize, viewportSize);
    if (!_hasArea(fitted.source) || !_hasArea(fitted.destination)) {
      _sourceRect = null;
      _destinationRect = null;
      return;
    }
    _sourceRect = Alignment.center.inscribe(
      fitted.source,
      Offset.zero & _orientedVideoSize,
    );
    _destinationRect = Alignment.center.inscribe(
      fitted.destination,
      Offset.zero & viewportSize,
    );
  }

  final Size viewportSize;
  final Size videoSize;
  final BoxFit fit;
  final QuarterTurn rotation;
  final Size _orientedVideoSize;
  late final Rect? _sourceRect;
  late final Rect? _destinationRect;

  /// Converts a local touch point to the original video's normalized 0..1 space.
  ///
  /// Returns null for invalid geometry, non-finite input, points outside the
  /// widget, or points in a letterbox created by `BoxFit.contain`.
  Offset? toVideoPoint(Offset localPosition) {
    final sourceRect = _sourceRect;
    final destinationRect = _destinationRect;
    if (sourceRect == null ||
        destinationRect == null ||
        !_isFinite(localPosition) ||
        !(Offset.zero & viewportSize).contains(localPosition) ||
        !destinationRect.contains(localPosition)) {
      return null;
    }

    final orientedPoint = Offset(
      sourceRect.left +
          ((localPosition.dx - destinationRect.left) / destinationRect.width) *
              sourceRect.width,
      sourceRect.top +
          ((localPosition.dy - destinationRect.top) / destinationRect.height) *
              sourceRect.height,
    );
    final normalizedOrientedPoint = Offset(
      orientedPoint.dx / _orientedVideoSize.width,
      orientedPoint.dy / _orientedVideoSize.height,
    );
    return _rotateToSource(normalizedOrientedPoint, rotation);
  }

  /// Converts an original-video normalized point into widget-local coordinates.
  ///
  /// A point hidden by `BoxFit.cover` can map beyond the viewport; paint the
  /// overlay inside a canvas clipped to its widget bounds.
  Offset? toViewportPoint(Offset normalizedVideoPoint) {
    final sourceRect = _sourceRect;
    final destinationRect = _destinationRect;
    if (sourceRect == null ||
        destinationRect == null ||
        !_isFinite(normalizedVideoPoint) ||
        normalizedVideoPoint.dx < 0 ||
        normalizedVideoPoint.dx > 1 ||
        normalizedVideoPoint.dy < 0 ||
        normalizedVideoPoint.dy > 1) {
      return null;
    }

    final orientedPoint = _rotateToDisplay(normalizedVideoPoint, rotation);
    final sourcePoint = Offset(
      orientedPoint.dx * _orientedVideoSize.width,
      orientedPoint.dy * _orientedVideoSize.height,
    );
    return Offset(
      destinationRect.left +
          ((sourcePoint.dx - sourceRect.left) / sourceRect.width) *
              destinationRect.width,
      destinationRect.top +
          ((sourcePoint.dy - sourceRect.top) / sourceRect.height) *
              destinationRect.height,
    );
  }

  static Size _rotateSize(Size size, QuarterTurn rotation) =>
      rotation == QuarterTurn.clockwise90 ||
          rotation == QuarterTurn.clockwise270
      ? Size(size.height, size.width)
      : size;

  static bool _hasArea(Size size) =>
      size.width.isFinite &&
      size.height.isFinite &&
      size.width > 0 &&
      size.height > 0;

  static bool _isFinite(Offset point) => point.dx.isFinite && point.dy.isFinite;

  static Offset _rotateToDisplay(Offset source, QuarterTurn rotation) =>
      switch (rotation) {
        QuarterTurn.none => source,
        QuarterTurn.clockwise90 => Offset(1 - source.dy, source.dx),
        QuarterTurn.clockwise180 => Offset(1 - source.dx, 1 - source.dy),
        QuarterTurn.clockwise270 => Offset(source.dy, 1 - source.dx),
      };

  static Offset _rotateToSource(Offset display, QuarterTurn rotation) =>
      switch (rotation) {
        QuarterTurn.none => display,
        QuarterTurn.clockwise90 => Offset(display.dy, 1 - display.dx),
        QuarterTurn.clockwise180 => Offset(1 - display.dx, 1 - display.dy),
        QuarterTurn.clockwise270 => Offset(1 - display.dy, display.dx),
      };
}
