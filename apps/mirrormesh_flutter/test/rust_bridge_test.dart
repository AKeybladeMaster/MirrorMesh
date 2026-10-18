import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirrormesh_flutter/app.dart';
import 'package:mirrormesh_flutter/rust_session_status.dart';

void main() {
  final libraryPath = Platform.environment['MIRRORMESH_FFI_LIBRARY'];

  testWidgets(
    'Rust bridge initializes the state view and keeps idle stop idempotent',
    (tester) async {
      final status = await RustSessionStatus.initialize(
        libraryPath: libraryPath,
      );

      expect(status.isAvailable, isTrue);
      expect(status.initialState, 'Idle');

      await tester.pumpWidget(MirrorMeshApp(rustSessionStatus: status));
      await tester.tap(find.text('Settings').first);
      await tester.pumpAndSettle();
      final settingsScroll = find
          .descendant(
            of: find.byType(ListView).first,
            matching: find.byType(Scrollable),
          )
          .first;
      await tester.scrollUntilVisible(
        find.text('Session state: Idle'),
        180,
        scrollable: settingsScroll,
      );
      expect(find.text('Rust core state bridge'), findsOneWidget);
      expect(status.stopSession(), 'Idle');
      expect(
        tester
            .widget<OutlinedButton>(
              find.byKey(const Key('stop-core-session-button')),
            )
            .onPressed,
        isNull,
      );

      await tester.tap(find.text('Connect').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('scan-demo-button')));
      await tester.pumpAndSettle();
      expect(status.currentState(), 'Discovered');

      await tester.tap(find.text('Devices').first);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('connect-demo-button')));
      await tester.pumpAndSettle();
      expect(status.currentState(), 'Pairing');

      await tester.tap(find.text('Session').first);
      await tester.pumpAndSettle();
      final sessionScroll = find
          .descendant(
            of: find.byType(ListView).first,
            matching: find.byType(Scrollable),
          )
          .first;
      await tester.scrollUntilVisible(
        find.byKey(const Key('stop-session-button')),
        180,
        scrollable: sessionScroll,
      );
      await tester.tap(find.byKey(const Key('stop-session-button')));
      await tester.pumpAndSettle();
      expect(status.currentState(), 'Stopped');

      await tester.tap(find.text('Settings').first);
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        find.text('Session state: Stopped'),
        180,
        scrollable: settingsScroll,
      );
      expect(find.text('Rust core state bridge'), findsOneWidget);
    },
    skip: (!Platform.isWindows && !Platform.isLinux) || libraryPath == null,
  );
}
