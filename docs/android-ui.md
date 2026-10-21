# Android receiver UI - 0.1.2

Design guidance: the globally installed `frontend-design` skill, applied to the existing
native Android Views. Sleek and UI Taste were inspected; their external service/image
workflows were not used. No new dependencies or account connections were needed.

## Direction

Keep the established receiver identity: ink `#071017`, primary text `#F2F7F8`, secondary
text `#A6B6BD`, action teal `#39D7D0`, surface `#12232C`, focus `#E6FFFD`.
Use Android's system sans font, 16sp body/actions, 22sp section headings and 30sp title.
Content is left aligned; QR geometry stays square and unmodified. Teal identifies the
next primary action; all state and action meaning also appears in text.

Setup: title/status -> address selector -> Start receiver.
Listening: title/status -> Share/Copy invitation -> QR/link -> new invitation -> Stop.
Approval: title/status -> PC identity -> unchecked permissions -> Approve/Reject -> Stop.
Viewing: native video beside a scrollable control panel; a labelled toggle expands video.

## Accessibility changes

- Buttons and permission rows have a 56dp minimum height with wrap-content sizing.
  Body and action text use sp and do not have fixed-height clipping boxes.
- Pairing columns stack on narrow screens and with increased font scale.
- Focus/press states pair bright backgrounds with dark text and a visible focus border.
- Native button, checkbox and heading semantics remain available to TalkBack. Status
  announcements are separate from frequently updated frame counters.
- The address selector has an associated label and readable dialog rows.
- Video controls occupy separate layout space, avoiding overlapping buttons on phones.
  Large text can scroll; no timed auto-hide or gesture-only control is required.
- Revocation is separated under Connection details and requires explicit confirmation.
  The existing permissions, pairing approval and native media pipeline remain unchanged.

Android's [Views accessibility guidance](https://developer.android.com/guide/topics/ui/accessibility/views/apps-views)
recommends at least 48dp interactive targets; these action controls use 56dp minimums.

## Validation boundaries

Builds and static analysis do not demonstrate TalkBack traversal, TV remote focus,
cutout behavior or layout at large font settings. No emulator is installed in the local
Android SDK and no native render was inspected. User device review remains necessary.
The user reported successful baseline 0.1.1 PC-to-phone video; this does not establish
0.1.2 UI or TV verification. No unit/instrumentation suites are added or run.
