import {
  assessReceiverSupport,
  describeReceiverStatus,
  nextRemoteFocusIndex,
  type ReceiverCapabilities,
} from "./capabilities.js";

const checks = document.querySelectorAll<HTMLElement>("[data-check]");

function requiredElement<T extends HTMLElement>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`Required page element not found: ${selector}`);
  return element;
}

const connectButton = requiredElement<HTMLButtonElement>("#connect-button");
const connectionNote = requiredElement<HTMLElement>("#connection-note");
const fullscreenButton = requiredElement<HTMLButtonElement>("#fullscreen-button");

function detectCapabilities(): ReceiverCapabilities {
  let videoCodecs: string[] | null = null;
  try {
    const codecs = globalThis.RTCRtpReceiver?.getCapabilities?.("video")?.codecs;
    if (codecs) videoCodecs = codecs.map((codec) => codec.mimeType);
  } catch {
    videoCodecs = null;
  }

  return assessReceiverSupport({
    isSecureContext: globalThis.isSecureContext,
    protocol: location.protocol,
    hostname: location.hostname,
    rtcAvailable: typeof globalThis.RTCPeerConnection === "function",
    videoCodecs,
    signalingConfigured: false,
    autoplayStatus: "unknown",
  });
}

function renderChecks(capabilities: ReceiverCapabilities): void {
  const labels: Record<string, string> = {
    secureOrigin: capabilities.secureOrigin
      ? "Ready"
      : "Open this receiver over HTTPS",
    webrtc: capabilities.webrtc ? "Available" : "Not supported",
    h264: capabilities.h264 === true
      ? "Available"
      : capabilities.h264 === false
        ? "Not supported"
        : "Could not verify",
    signaling: capabilities.signaling ? "Available" : "Unavailable · not configured",
    autoplay: capabilities.autoplay === "available"
      ? "Available"
      : capabilities.autoplay === "blocked"
        ? "Use Play on the TV remote"
        : "Not tested until media arrives",
  };

  for (const row of checks) {
    const key = row.dataset.check;
    const value = row.querySelector<HTMLElement>(".check-value");
    if (!key || !value) continue;

    const passed = key === "secureOrigin" ? capabilities.secureOrigin
      : key === "webrtc" ? capabilities.webrtc
        : key === "h264" ? capabilities.h264 === true
          : key === "signaling" ? capabilities.signaling
            : key === "autoplay" ? capabilities.autoplay === "available"
              : false;
    const unverified = (key === "h264" && capabilities.h264 === null)
      || (key === "autoplay" && capabilities.autoplay === "unknown");
    row.dataset.state = passed ? "ready" : unverified ? "unknown" : "unavailable";
    value.textContent = labels[key] ?? "Could not verify";
  }

  connectButton.disabled = !capabilities.canReceive;
  connectionNote.textContent = describeReceiverStatus(capabilities);
}

function runChecks(): void {
  renderChecks(detectCapabilities());
}

function renderFullscreenState(): void {
  const isFullscreen = Boolean(document.fullscreenElement);
  fullscreenButton.textContent = isFullscreen ? "Exit full screen" : "Full screen";
  fullscreenButton.setAttribute("aria-pressed", String(isFullscreen));
}

requiredElement<HTMLButtonElement>("#refresh-button").addEventListener("click", runChecks);
fullscreenButton.addEventListener("click", async () => {
  try {
    if (document.fullscreenElement) {
      await document.exitFullscreen();
    } else if (document.documentElement.requestFullscreen) {
      await document.documentElement.requestFullscreen();
    } else {
      connectionNote.textContent = "This browser does not provide full-screen mode for this page.";
    }
  } catch {
    connectionNote.textContent = "Full-screen mode was not allowed by this browser.";
  }
});

document.addEventListener("fullscreenchange", renderFullscreenState);
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && document.fullscreenElement) {
    void document.exitFullscreen().catch(() => {
      connectionNote.textContent = "Full-screen mode could not be closed by this browser.";
    });
    return;
  }

  if (!event.key.startsWith("Arrow")) return;
  const controls = [...document.querySelectorAll<HTMLElement>("button:not(:disabled), a[href]")];
  const activeElement = document.activeElement;
  if (!(activeElement instanceof HTMLElement)) return;
  const currentIndex = controls.indexOf(activeElement);
  if (currentIndex < 0 || controls.length < 2) return;

  const nextIndex = nextRemoteFocusIndex(currentIndex, controls.length, event.key);
  if (nextIndex === currentIndex) return;
  event.preventDefault();
  controls[nextIndex]?.focus();
});

renderFullscreenState();
runChecks();
