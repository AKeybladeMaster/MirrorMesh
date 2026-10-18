export type AutoplayStatus = "available" | "blocked" | "unknown";

export interface ReceiverSupportInput {
  isSecureContext?: boolean;
  protocol?: string;
  hostname?: string;
  rtcAvailable?: boolean;
  videoCodecs?: readonly string[] | null;
  signalingConfigured?: boolean;
  autoplayStatus?: AutoplayStatus;
}

export interface ReceiverCapabilities {
  secureOrigin: boolean;
  webrtc: boolean;
  h264: boolean | null;
  signaling: boolean;
  autoplay: AutoplayStatus;
  canReceive: boolean;
}

const LOOPBACK_HOSTS: ReadonlySet<string> = new Set(["localhost", "127.0.0.1", "::1"]);

export function assessReceiverSupport({
  isSecureContext = false,
  protocol = "",
  hostname = "",
  rtcAvailable = false,
  videoCodecs = null,
  signalingConfigured = false,
  autoplayStatus = "unknown",
}: ReceiverSupportInput = {}): ReceiverCapabilities {
  const localPreview = LOOPBACK_HOSTS.has(hostname);
  const secureOrigin = isSecureContext && (protocol === "https:" || localPreview);
  const webrtc = Boolean(rtcAvailable);
  const h264 = Array.isArray(videoCodecs)
    ? videoCodecs.some((codec) => codec.toLowerCase().startsWith("video/h264"))
    : null;
  const signaling = Boolean(signalingConfigured);
  const autoplay: AutoplayStatus = autoplayStatus === "available" || autoplayStatus === "blocked"
    ? autoplayStatus
    : "unknown";

  return {
    secureOrigin,
    webrtc,
    h264,
    signaling,
    autoplay,
    canReceive: secureOrigin && webrtc && h264 === true && signaling,
  };
}

export function describeReceiverStatus(capabilities: ReceiverCapabilities): string {
  const messages: string[] = [];
  if (!capabilities.secureOrigin) {
    messages.push("Remote access requires HTTPS with a trusted certificate. Do not bypass certificate warnings.");
  }
  if (!capabilities.webrtc) {
    messages.push("This browser does not expose RTCPeerConnection and cannot receive WebRTC media.");
  }
  if (capabilities.h264 === false) {
    messages.push("This browser does not advertise H.264 WebRTC receive support.");
  } else if (capabilities.h264 === null) {
    messages.push("Could not verify H.264 receive support in this browser.");
  }
  if (!capabilities.signaling) {
    messages.push("Authenticated signaling is not configured, so Connect remains unavailable.");
  }
  if (capabilities.autoplay === "blocked") {
    messages.push("Autoplay is blocked; after an approved session, press Play on the TV remote to start video.");
  } else if (capabilities.autoplay === "unknown") {
    messages.push("Autoplay has not been verified without an incoming media track; the TV remote may need a Play action.");
  }

  return messages.length > 0
    ? messages.join(" ")
    : "Browser checks pass. Pairing and playback still require explicit user action.";
}

export function nextRemoteFocusIndex(currentIndex: number, controlCount: number, key: string): number {
  if (!Number.isInteger(currentIndex) || !Number.isInteger(controlCount)
    || currentIndex < 0 || currentIndex >= controlCount || controlCount < 2) {
    return currentIndex;
  }

  const direction = key === "ArrowRight" || key === "ArrowDown"
    ? 1
    : key === "ArrowLeft" || key === "ArrowUp"
      ? -1
      : 0;
  if (direction === 0) return currentIndex;

  return (currentIndex + direction + controlCount) % controlCount;
}
