# ADR-0045 — Save an explicitly requested Android TV frame screenshot

- **Status:** Proposed
- **Date:** 2026-10-19

## Context

The Android TV receiver renders decoded WebRTC frames into a native
`SurfaceViewRenderer`. Session diagnostics report dimensions and frame count,
but there is no way for a user to save a received frame.

## Decision

Enable a D-pad accessible **Save screenshot** action only after the first
frame is rendered. On activation, the receiver copies the next decoded frame
from I420 into bounded NV21 memory, releases the WebRTC frame buffer
immediately, and compresses the one-shot snapshot to JPEG on a worker thread.
The copy is limited to 3840 x 2160 pixels and is never retained as a stream
buffer. The JPEG is saved to `Pictures/MirrorMesh` through MediaStore on Android
10 and later; older versions use the app-specific Pictures directory. No broad
storage permission is added. If no new frame arrives within five seconds, the
request is cancelled and can be retried.

This storage choice follows the Android guidance that an app does not need a
storage permission to create and access its own MediaStore media on Android 10
and later: [Access media files from shared storage](https://developer.android.com/training/data-storage/shared/media).

## Limits

The conversion has JDK-only tests for plane stride, order, and bounds. It has
not been compiled against the Android SDK, run on an emulator or TV, or
validated with a hardware-decoded frame. The snapshot copy and JPEG conversion
occur only after a direct user action and are not evidence of a working
sender-to-TV media path.

## Verification gate

`SOURCE_IMPLEMENTED` requires one-shot capture, bounded conversion, explicit
save action, and cleanup. `BUILD_VERIFIED` requires a successful Android TV
APK build. `AUTOMATED_INTEGRATION_VERIFIED` requires an automated decoded frame
and screenshot output. `HARDWARE_VERIFIED` requires final physical-TV testing.
