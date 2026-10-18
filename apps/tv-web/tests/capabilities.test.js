import test from "node:test";
import assert from "node:assert/strict";
import {
  assessReceiverSupport,
  describeReceiverStatus,
  nextRemoteFocusIndex,
} from "../dist/capabilities.js";

const allMediaReady = {
  isSecureContext: true,
  protocol: "https:",
  hostname: "receiver.example",
  rtcAvailable: true,
  videoCodecs: ["video/VP8", "video/H264"],
};

test("allows secure localhost previews but requires HTTPS for remote hosts", () => {
  assert.equal(assessReceiverSupport({ ...allMediaReady, protocol: "http:", hostname: "localhost" }).secureOrigin, true);
  assert.equal(assessReceiverSupport({ ...allMediaReady, protocol: "http:", hostname: "tv.lan" }).secureOrigin, false);
  assert.equal(assessReceiverSupport({ ...allMediaReady, isSecureContext: false }).secureOrigin, false);
});

test("reports WebRTC and H.264 independently", () => {
  const unsupported = assessReceiverSupport({ ...allMediaReady, rtcAvailable: false, videoCodecs: ["video/VP8"] });
  assert.equal(unsupported.webrtc, false);
  assert.equal(unsupported.h264, false);
  assert.equal(assessReceiverSupport({ ...allMediaReady, videoCodecs: null }).h264, null);
});

test("never enables receive until authenticated signaling is configured", () => {
  assert.equal(assessReceiverSupport(allMediaReady).canReceive, false);
  assert.equal(assessReceiverSupport({ ...allMediaReady, signalingConfigured: true }).canReceive, true);
  assert.equal(assessReceiverSupport({ ...allMediaReady, signalingConfigured: true, rtcAvailable: false }).canReceive, false);
});

test("reports concrete secure-origin, WebRTC, and H.264 blockers", () => {
  const insecure = assessReceiverSupport({ ...allMediaReady, isSecureContext: false });
  assert.match(describeReceiverStatus(insecure), /HTTPS with a trusted certificate/i);
  assert.doesNotMatch(describeReceiverStatus(insecure), /accept.*certificate/i);

  const noWebRtc = assessReceiverSupport({ ...allMediaReady, rtcAvailable: false });
  assert.match(describeReceiverStatus(noWebRtc), /RTCPeerConnection/i);

  const noH264 = assessReceiverSupport({ ...allMediaReady, videoCodecs: ["video/VP8"] });
  assert.match(describeReceiverStatus(noH264), /H\.264/i);

  const unknownH264 = assessReceiverSupport({ ...allMediaReady, videoCodecs: null });
  assert.match(describeReceiverStatus(unknownH264), /could not verify H\.264/i);
});

test("explains that autoplay may need remote input and signaling is still required", () => {
  const blockedPlayback = assessReceiverSupport({ ...allMediaReady, autoplayStatus: "blocked" });
  assert.equal(blockedPlayback.autoplay, "blocked");
  assert.match(describeReceiverStatus(blockedPlayback), /press Play.*TV remote/i);

  const uncheckedPlayback = assessReceiverSupport(allMediaReady);
  assert.equal(uncheckedPlayback.autoplay, "unknown");
  assert.match(describeReceiverStatus(uncheckedPlayback), /autoplay has not been verified/i);
  assert.match(describeReceiverStatus(uncheckedPlayback), /authenticated signaling is not configured/i);
});

test("a user gesture can address autoplay policy but does not waive other gates", () => {
  const manuallyStarted = assessReceiverSupport({
    ...allMediaReady,
    signalingConfigured: true,
    autoplayStatus: "blocked",
  });
  assert.equal(manuallyStarted.canReceive, true);

  const noH264 = assessReceiverSupport({
    ...manuallyStarted,
    videoCodecs: ["video/VP8"],
  });
  assert.equal(noH264.canReceive, false);
});

test("remote arrows move focus through available controls and wrap at each end", () => {
  assert.equal(nextRemoteFocusIndex(0, 3, "ArrowLeft"), 2);
  assert.equal(nextRemoteFocusIndex(2, 3, "ArrowRight"), 0);
  assert.equal(nextRemoteFocusIndex(1, 3, "ArrowDown"), 2);
  assert.equal(nextRemoteFocusIndex(1, 3, "ArrowUp"), 0);
  assert.equal(nextRemoteFocusIndex(1, 3, "Enter"), 1);
  assert.equal(nextRemoteFocusIndex(-1, 3, "ArrowDown"), -1);
  assert.equal(nextRemoteFocusIndex(0, 1, "ArrowRight"), 0);
});
