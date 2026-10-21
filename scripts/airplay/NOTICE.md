# AirPlay interoperability preview

UxPlay revision 15a6b87172eca38b0df4e615fbf1bba108734dde:
https://github.com/FDH2/UxPlay

UxPlay and MirrorMesh's modifications to it are GPL-3.0-or-later. The license and complete
modified UxPlay source accompany this locally built preview under licenses/ and source/.
Build it with the supplied script and the recorded MSYS2 UCRT64 packages. No Apple SDK,
signing credentials or AirPlay certification is supplied by MirrorMesh.

Native shared libraries and selected GStreamer plugins come from official signed MSYS2
packages. Their licenses are in licenses/; exact package versions and upstream build recipe
links are in source/runtime-packages.json. These source recipes identify their upstream
source archives and checksums. Consult https://www.msys2.org/dev/package-sources/ for the
package source process. This preview is built locally for the project owner; do not publish
its binaries as a public release until complete dependency source/notice obligations have
been reviewed and satisfied. Source-only MirrorMesh changes remain independently reviewable.

This receiver is experimental, is not Apple-certified, and does not bypass protected-content
restrictions. DRM streaming, peer-to-peer AWDL and AirPlay multi-room audio are not promised.
No raw media enters the Flutter/Dart or Rust metadata bridges. Existing MirrorMesh native
WebRTC connections retain their own TLS-pinned pairing protocol.
