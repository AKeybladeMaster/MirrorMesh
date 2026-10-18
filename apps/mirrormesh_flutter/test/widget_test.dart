import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/app.dart';
import 'package:mirrormesh_flutter/annotation_overlay.dart';

void main() {
  testWidgets('shell navigation uses logical width at 2x pixel density', (
    tester,
  ) async {
    tester.view.devicePixelRatio = 2;
    tester.view.physicalSize = const Size(800, 1600);
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(const MirrorMeshApp());
    expect(find.byType(NavigationBar), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);

    tester.view.physicalSize = const Size(1600, 1200);
    await tester.pumpAndSettle();
    expect(find.byType(NavigationRail), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
  });

  testWidgets('connection waits for the signaling service', (tester) async {
    await tester.pumpWidget(const MirrorMeshApp());

    expect(find.text('Pairing service is not connected'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.byKey(const Key('connect-button')))
          .onPressed,
      isNull,
    );
    final connectScroll = find
        .descendant(
          of: find.byType(ListView).first,
          matching: find.byType(Scrollable),
        )
        .first;
    await tester.scrollUntilVisible(
      find.textContaining('Screen capture will only be requested'),
      180,
      scrollable: connectScroll,
    );
    expect(
      find.textContaining('Screen capture will only be requested'),
      findsOneWidget,
    );
  });

  testWidgets('session controls remain inactive with no session', (
    tester,
  ) async {
    await tester.pumpWidget(const MirrorMeshApp());
    await tester.tap(find.text('Session').first);
    await tester.pumpAndSettle();

    expect(find.text('No active session'), findsOneWidget);
    expect(
      tester
          .widget<OutlinedButton>(find.byKey(const Key('stop-session-button')))
          .onPressed,
      isNull,
    );
    expect(
      find.textContaining('No capture permission has been requested.'),
      findsOneWidget,
    );
  });

  testWidgets('demo receiver can be discovered, paired, and ended', (
    tester,
  ) async {
    await tester.pumpWidget(const MirrorMeshApp());
    await tester.tap(find.byKey(const Key('scan-demo-button')));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Devices').first);
    await tester.pumpAndSettle();
    expect(find.text('Demo receiver'), findsOneWidget);
    await tester.tap(find.byKey(const Key('connect-demo-button')));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Session').first);
    await tester.pumpAndSettle();
    final sessionScroll = find
        .descendant(
          of: find.byType(ListView).first,
          matching: find.byType(Scrollable),
        )
        .first;

    expect(find.text('Demo pairing preview'), findsOneWidget);
    expect(find.text('Demo pairing active'), findsOneWidget);
    expect(
      find.textContaining('Local simulation only; no peer is verified'),
      findsOneWidget,
    );
    await tester.scrollUntilVisible(
      find.byKey(const Key('demo-preview')),
      240,
      scrollable: sessionScroll,
    );
    expect(find.byKey(const Key('demo-preview')), findsOneWidget);
    await tester.scrollUntilVisible(
      find.byKey(const Key('screenshot-button')),
      180,
      scrollable: sessionScroll,
    );
    await tester.scrollUntilVisible(
      find.byKey(const Key('stop-session-button')),
      180,
      scrollable: sessionScroll,
    );
    expect(
      tester
          .widget<OutlinedButton>(find.byKey(const Key('screenshot-button')))
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<OutlinedButton>(find.byKey(const Key('stop-session-button')))
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byKey(const Key('stop-session-button')));
    await tester.pumpAndSettle();

    expect(find.text('No active session'), findsOneWidget);
    await tester.tap(find.text('Devices').first);
    await tester.pumpAndSettle();
    expect(find.text('No discovered devices'), findsOneWidget);
    await tester.tap(find.text('Connect').first);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('scan-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Devices').first);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('connect-demo-button')), findsOneWidget);
  });

  testWidgets('demo annotations are local and can be cleared', (tester) async {
    await tester.pumpWidget(const MirrorMeshApp());
    await tester.tap(find.byKey(const Key('scan-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Devices').first);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('connect-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Session').first);
    await tester.pumpAndSettle();
    final sessionScroll = find
        .descendant(
          of: find.byType(ListView).first,
          matching: find.byType(Scrollable),
        )
        .first;

    await tester.scrollUntilVisible(
      find.byKey(const Key('toggle-annotation-button')),
      220,
      scrollable: sessionScroll,
    );
    await tester.tap(find.byKey(const Key('toggle-annotation-button')));
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('demo-preview')),
      -220,
      scrollable: sessionScroll,
    );
    final preview = tester.getRect(find.byKey(const Key('demo-preview')));
    await tester.dragFrom(
      Offset(preview.center.dx, preview.bottom - 30),
      const Offset(60, 40),
    );
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-count')),
      220,
      scrollable: sessionScroll,
    );
    expect(find.text('1 local annotation stroke'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.byKey(const Key('undo-annotation-button')),
      220,
      scrollable: sessionScroll,
    );
    expect(
      tester
          .widget<OutlinedButton>(
            find.byKey(const Key('undo-annotation-button')),
          )
          .onPressed,
      isNotNull,
    );
    await tester.scrollUntilVisible(
      find.byKey(const Key('clear-annotations-button')),
      220,
      scrollable: sessionScroll,
    );
    await tester.ensureVisible(
      find.byKey(const Key('clear-annotations-button')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('clear-annotations-button')));
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-count')),
      220,
      scrollable: sessionScroll,
    );
    expect(find.text('0 local annotation strokes'), findsOneWidget);
  });

  testWidgets('demo strokes keep the color and width selected at draw time', (
    tester,
  ) async {
    await tester.pumpWidget(const MirrorMeshApp());
    await tester.tap(find.byKey(const Key('scan-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Devices').first);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('connect-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Session').first);
    await tester.pumpAndSettle();
    expect(find.byType(ListView), findsOneWidget);
    expect(find.byType(Scrollable), findsOneWidget);
    final sessionScroll = find.byType(Scrollable).first;
    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-color-coral')),
      220,
      scrollable: sessionScroll,
    );
    await tester.tap(find.byKey(const Key('annotation-color-coral')));
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-width-bold')),
      220,
      scrollable: sessionScroll,
    );
    await tester.tap(find.byKey(const Key('annotation-width-bold')));
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('toggle-annotation-button')),
      220,
      scrollable: sessionScroll,
    );
    await tester.tap(find.byKey(const Key('toggle-annotation-button')));
    await tester.pumpAndSettle();
    await tester.scrollUntilVisible(
      find.byKey(const Key('demo-preview')),
      -220,
      scrollable: sessionScroll,
    );
    final preview = tester.getRect(find.byKey(const Key('demo-preview')));
    await tester.dragFrom(
      Offset(preview.center.dx, preview.center.dy),
      const Offset(50, 20),
    );
    await tester.pumpAndSettle();

    AnnotationOverlayPainter overlayPainter() =>
        tester
                .widget<CustomPaint>(
                  find.byKey(const Key('annotation-overlay')),
                )
                .foregroundPainter!
            as AnnotationOverlayPainter;

    expect(overlayPainter().strokes.single.color, const Color(0xFFFF7A90));
    expect(overlayPainter().strokes.single.strokeWidth, 8);

    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-color-teal')),
      220,
      scrollable: sessionScroll,
    );
    await tester.tap(find.byKey(const Key('annotation-color-teal')));
    expect(overlayPainter().strokes.single.color, const Color(0xFFFF7A90));
  });

  testWidgets('session keyboard shortcuts toggle drawing and undo strokes', (
    tester,
  ) async {
    final semantics = tester.ensureSemantics();

    await tester.pumpWidget(const MirrorMeshApp());
    await tester.sendKeyEvent(LogicalKeyboardKey.keyP);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('scan-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Devices').first);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('connect-demo-button')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Session').first);
    await tester.pumpAndSettle();

    final sessionScroll = find.byType(Scrollable).first;
    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-color-teal')),
      220,
      scrollable: sessionScroll,
    );
    expect(find.bySemanticsLabel(RegExp('Teal')), findsOneWidget);
    await tester.scrollUntilVisible(
      find.byKey(const Key('annotation-width-bold')),
      220,
      scrollable: sessionScroll,
    );
    expect(find.bySemanticsLabel(RegExp('Bold')), findsOneWidget);
    await tester.scrollUntilVisible(
      find.byKey(const Key('toggle-annotation-button')),
      220,
      scrollable: sessionScroll,
    );
    expect(find.bySemanticsLabel('Draw locally (P)'), findsOneWidget);
    expect(find.bySemanticsLabel('Undo (Ctrl/⌘+Z)'), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.keyP);
    await tester.pumpAndSettle();
    expect(find.text('Turn pen off (P)'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.byKey(const Key('demo-preview')),
      -220,
      scrollable: sessionScroll,
    );
    final preview = tester.getRect(find.byKey(const Key('demo-preview')));
    await tester.dragFrom(
      Offset(preview.center.dx, preview.center.dy),
      const Offset(50, 20),
    );
    await tester.pumpAndSettle();
    expect(find.text('1 local annotation stroke'), findsOneWidget);

    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();

    expect(find.text('0 local annotation strokes'), findsOneWidget);

    await tester.scrollUntilVisible(
      find.byKey(const Key('demo-preview')),
      -220,
      scrollable: sessionScroll,
    );
    final secondPreview = tester.getRect(find.byKey(const Key('demo-preview')));
    await tester.dragFrom(
      Offset(secondPreview.center.dx, secondPreview.center.dy),
      const Offset(50, 20),
    );
    await tester.pumpAndSettle();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
    await tester.pumpAndSettle();

    expect(find.text('0 local annotation strokes'), findsOneWidget);
    semantics.dispose();
  });
}
