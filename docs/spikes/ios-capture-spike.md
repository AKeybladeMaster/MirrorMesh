# iOS screen capture feasibility spike

- **Date:** 2026-10-18
- **Scope:** iOS/iPadOS display capture sender only
- **Status:** API path identified; macOS SDK compilation and physical-device validation remain open
- **Requirements:** P2-IOS, C-01

## Finding

Apple's current [Capturing screen content on iOS sample](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-on-ios) demonstrates a system picker, `SCStream`, and screen output, and says the sample requires iOS 27 or later. Apple labels the sample documentation preliminary and subject to change. Treat iOS 27+ as the first candidate capability gate for this implementation; the sample's deployment requirement does not establish the minimum availability of every ScreenCaptureKit API.

The sample returns the selected `SCContentFilter` from the system-provided sharing picker and starts `SCStream` from that user-selected filter. It also declares a `screen-capture` background mode for full-display capture that continues while the app is not frontmost. MirrorMesh should not add this background mode or camera, microphone, or photo-library permissions until its own tested flow requires them.

## Current repository evidence

- The Flutter app deployment target is iOS 15.0; no ScreenCaptureKit capture code is present.
- The iOS Rust state bridge has an Xcode device build phase and a Dart FFI symbol export list. This host is Windows and has no Xcode, so the device link and API availability were not compiled here.
- The iOS Flutter CI job requests `aarch64-apple-ios` and builds an unsigned iOS app on `macos-latest`, but no hosted workflow run is available in this workspace.
- No iOS 27+ device is available. No picker, capture, background, orientation, audio, thermal, or App Review scenario was run.
- ADR-0002 has not selected the production transport. No captured video may cross Dart FFI.

## Capability and validation matrix

| Area | Current official evidence | MirrorMesh result |
|---|---|---|
| Compile/runtime | Apple's current sample says iOS 27+; no local Xcode or iOS SDK is installed. | Not compiled or run. Keep sender unavailable until a macOS SDK probe and device test pass. |
| User picker | The sample uses Apple's shared content picker and starts capture from its returned content filter. | Required design for a sender; no app flow implemented or exercised. |
| Background | The sample declares `screen-capture` so full-display capture can continue when the app is not frontmost. | MirrorMesh has no background capture mode. Need product-specific policy and battery/thermal tests before adding one. |
| Orientation | No orientation behavior has been tested on MirrorMesh hardware. | Open; verify dimensions, rotation metadata, and receiver behavior after transport and render contracts exist. |
| Audio | Apple's sample includes microphone controls and an `audio` background mode for its microphone path. | MirrorMesh has no audio capture; do not request microphone permission for the initial screen-only slice. |
| App Store review | No app submission or capture-policy review has been performed. | Open; review exact user disclosure, privacy strings, background behavior, and audio use before a release claim. |

## Decision and next gates

Keep iOS screen sending unavailable in the product. Do not add a ReplayKit fallback or claim support for earlier OS versions based on assumptions. Continue the iOS receiver and control-plane work only against reviewed shared contracts; real media remains gated on ADR-0002.

On a macOS runner with the final iOS SDK, record `xcodebuild -version` and `xcrun --sdk iphoneos --show-sdk-version`, compile a minimal availability probe against the chosen deployment target, and verify the current sample's documented iOS 27+ path. Then test a physical supported device for the system picker, explicit start/stop, background policy, and thermal behavior before advertising a sender capability. Check privacy strings and App Review requirements for the exact capture and audio behavior before requesting those permissions.

## Sources

- [Apple: Capturing screen content on iOS](https://developer.apple.com/documentation/screencapturekit/capturing-screen-content-on-ios)
- [Apple: `SCStream`](https://developer.apple.com/documentation/screencapturekit/scstream)
- [Apple: `SCContentSharingPicker`](https://developer.apple.com/documentation/screencapturekit/sccontentsharingpicker)
