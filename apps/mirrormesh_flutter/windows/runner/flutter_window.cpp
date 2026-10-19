#include "flutter_window.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <variant>
#include <vector>

#include <windows.h>

#include "flutter/generated_plugin_registrant.h"

namespace {

constexpr char kWindowsPairingChannel[] = "com.mirrormesh/windows_pairing";
constexpr char kPreparePairingRuntimeMethod[] = "preparePairingRuntime";
constexpr char kBeginSenderPairingMethod[] = "beginSenderPairing";
constexpr char kPollSenderPairingMethod[] = "pollSenderPairing";
constexpr char kCancelSenderPairingMethod[] = "cancelSenderPairing";
constexpr char kBeginSenderSessionMethod[] = "beginSenderSession";
constexpr char kStartSenderSessionMethod[] = "startSenderSession";
constexpr char kStopSenderSessionMethod[] = "stopSenderSession";
constexpr char kSendSenderSessionIceMethod[] = "sendSenderSessionIce";
constexpr char kPollSenderSessionEventMethod[] = "pollSenderSessionEvent";
constexpr char kPreparePairingRuntimeSymbol[] =
    "mirrormesh_windows_prepare_pairing_runtime";
constexpr char kBeginSenderPairingSymbol[] =
    "mirrormesh_windows_begin_sender_pairing";
constexpr char kPollSenderPairingSymbol[] =
    "mirrormesh_windows_poll_sender_pairing";
constexpr char kCancelSenderPairingSymbol[] =
    "mirrormesh_windows_cancel_sender_pairing";
constexpr char kShutdownSenderPairingSymbol[] =
    "mirrormesh_windows_shutdown_sender_pairing";
constexpr char kBeginSenderSessionSymbol[] =
    "mirrormesh_windows_begin_sender_session";
constexpr char kStartSenderSessionSymbol[] =
    "mirrormesh_windows_start_sender_session";
constexpr char kStopSenderSessionSymbol[] =
    "mirrormesh_windows_stop_sender_session";
constexpr char kSendSenderSessionIceSymbol[] =
    "mirrormesh_windows_send_sender_session_ice";
constexpr char kPollSenderSessionEventSymbol[] =
    "mirrormesh_windows_poll_sender_session_event";
constexpr uint32_t kSenderPairingApprovedState = 3;
constexpr size_t kMaxSessionEventJsonBytes = 256 * 1024;

struct SenderPairingSnapshot {
  uint32_t state;
  uint32_t error_code;
  std::array<uint8_t, 32> receiver_fingerprint;
};

std::string FormatDeviceId(const std::array<uint8_t, 16>& bytes) {
  constexpr char kHex[] = "0123456789abcdef";
  std::string formatted;
  formatted.reserve(36);
  for (size_t index = 0; index < bytes.size(); ++index) {
    if (index == 4 || index == 6 || index == 8 || index == 10) {
      formatted.push_back('-');
    }
    formatted.push_back(kHex[bytes[index] >> 4]);
    formatted.push_back(kHex[bytes[index] & 0x0f]);
  }
  return formatted;
}

std::string FormatFingerprint(const std::array<uint8_t, 32>& bytes) {
  constexpr char kHex[] = "0123456789abcdef";
  std::string formatted;
  formatted.reserve(bytes.size() * 2);
  for (const uint8_t byte : bytes) {
    formatted.push_back(kHex[byte >> 4]);
    formatted.push_back(kHex[byte & 0x0f]);
  }
  return formatted;
}

bool PreparePairingRuntime(std::array<uint8_t, 16>* device_id) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return false;
  }
  using PrepareRuntime = int32_t (*)(uint8_t*, size_t);
  auto prepare_runtime = reinterpret_cast<PrepareRuntime>(
      ::GetProcAddress(rust_bridge, kPreparePairingRuntimeSymbol));
  return prepare_runtime != nullptr &&
         prepare_runtime(device_id->data(), device_id->size()) == 0;
}

int32_t BeginSenderPairing(const std::string& invitation_uri,
                           uint64_t* request_id) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using BeginPairing = int32_t (*)(const uint8_t*, size_t, uint64_t*);
  auto begin_pairing = reinterpret_cast<BeginPairing>(
      ::GetProcAddress(rust_bridge, kBeginSenderPairingSymbol));
  if (begin_pairing == nullptr) {
    return 2;
  }
  return begin_pairing(
      reinterpret_cast<const uint8_t*>(invitation_uri.data()),
      invitation_uri.size(), request_id);
}

bool PollSenderPairing(uint64_t request_id,
                       SenderPairingSnapshot* snapshot) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return false;
  }
  using PollPairing = int32_t (*)(uint64_t, SenderPairingSnapshot*);
  auto poll_pairing = reinterpret_cast<PollPairing>(
      ::GetProcAddress(rust_bridge, kPollSenderPairingSymbol));
  return poll_pairing != nullptr && poll_pairing(request_id, snapshot) == 0;
}

int32_t CancelSenderPairing(uint64_t request_id) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using CancelPairing = int32_t (*)(uint64_t);
  auto cancel_pairing = reinterpret_cast<CancelPairing>(
      ::GetProcAddress(rust_bridge, kCancelSenderPairingSymbol));
  return cancel_pairing == nullptr ? 2 : cancel_pairing(request_id);
}

int32_t BeginSenderSession(uint64_t request_id, const std::string& sdp,
                           std::string* session_id) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using BeginSession = int32_t (*)(uint64_t, const uint8_t*, size_t, uint8_t*,
                                   size_t, size_t*);
  auto begin_session = reinterpret_cast<BeginSession>(
      ::GetProcAddress(rust_bridge, kBeginSenderSessionSymbol));
  if (begin_session == nullptr) {
    return 2;
  }
  std::array<uint8_t, 36> output{};
  size_t output_length = 0;
  const int32_t status = begin_session(
      request_id, reinterpret_cast<const uint8_t*>(sdp.data()), sdp.size(),
      output.data(), output.size(), &output_length);
  if (status == 0 && output_length == output.size()) {
    session_id->assign(reinterpret_cast<const char*>(output.data()),
                       output_length);
  }
  return status;
}

int32_t SendSenderSessionIce(uint64_t request_id,
                             const std::string& media_id,
                             uint32_t media_line_index,
                             const std::string& candidate) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using SendIce = int32_t (*)(uint64_t, const uint8_t*, size_t, uint32_t,
                              const uint8_t*, size_t);
  auto send_ice = reinterpret_cast<SendIce>(
      ::GetProcAddress(rust_bridge, kSendSenderSessionIceSymbol));
  if (send_ice == nullptr) {
    return 2;
  }
  return send_ice(
      request_id, reinterpret_cast<const uint8_t*>(media_id.data()),
      media_id.size(), media_line_index,
      reinterpret_cast<const uint8_t*>(candidate.data()), candidate.size());
}

int32_t StartSenderSession(uint64_t request_id, const std::string& source_id) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using StartSession = int32_t (*)(uint64_t, const uint8_t*, size_t);
  auto start_session = reinterpret_cast<StartSession>(
      ::GetProcAddress(rust_bridge, kStartSenderSessionSymbol));
  if (start_session == nullptr) {
    return 2;
  }
  return start_session(
      request_id, reinterpret_cast<const uint8_t*>(source_id.data()),
      source_id.size());
}

int32_t StopSenderSession(uint64_t request_id, const std::string& reason) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using StopSession = int32_t (*)(uint64_t, const uint8_t*, size_t);
  auto stop_session = reinterpret_cast<StopSession>(
      ::GetProcAddress(rust_bridge, kStopSenderSessionSymbol));
  if (stop_session == nullptr) {
    return 2;
  }
  return stop_session(
      request_id, reinterpret_cast<const uint8_t*>(reason.data()),
      reason.size());
}

int32_t PollSenderSessionEvent(uint64_t request_id, std::string* event_json) {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return 2;
  }
  using PollSessionEvent = int32_t (*)(uint64_t, uint8_t*, size_t, size_t*);
  auto poll_event = reinterpret_cast<PollSessionEvent>(
      ::GetProcAddress(rust_bridge, kPollSenderSessionEventSymbol));
  if (poll_event == nullptr) {
    return 2;
  }
  std::vector<uint8_t> output(kMaxSessionEventJsonBytes);
  size_t output_length = 0;
  const int32_t status = poll_event(request_id, output.data(), output.size(),
                                    &output_length);
  if (status == 0 && output_length <= output.size()) {
    event_json->assign(reinterpret_cast<const char*>(output.data()),
                       output_length);
  }
  return status;
}

void ShutdownSenderPairing() {
  HMODULE rust_bridge = ::GetModuleHandleW(L"mm_ffi.dll");
  if (rust_bridge == nullptr) {
    return;
  }
  using ShutdownPairing = void (*)();
  auto shutdown_pairing = reinterpret_cast<ShutdownPairing>(
      ::GetProcAddress(rust_bridge, kShutdownSenderPairingSymbol));
  if (shutdown_pairing != nullptr) {
    shutdown_pairing();
  }
}

flutter::EncodableValue PairingStatusValue(
    const SenderPairingSnapshot& snapshot) {
  flutter::EncodableMap status;
  status[flutter::EncodableValue("state")] =
      flutter::EncodableValue(static_cast<int32_t>(snapshot.state));
  status[flutter::EncodableValue("errorCode")] =
      flutter::EncodableValue(static_cast<int32_t>(snapshot.error_code));
  status[flutter::EncodableValue("receiverFingerprint")] =
      flutter::EncodableValue(
          snapshot.state == kSenderPairingApprovedState
              ? FormatFingerprint(snapshot.receiver_fingerprint)
              : std::string());
  return flutter::EncodableValue(status);
}

std::optional<uint64_t> ReadPairingRequestId(
    const flutter::EncodableValue* argument) {
  if (argument == nullptr) {
    return std::nullopt;
  }
  if (const auto* request_id = std::get_if<int32_t>(argument)) {
    return *request_id > 0 ? std::optional<uint64_t>(*request_id)
                           : std::nullopt;
  }
  if (const auto* request_id = std::get_if<int64_t>(argument)) {
    return *request_id > 0 ? std::optional<uint64_t>(*request_id)
                           : std::nullopt;
  }
  return std::nullopt;
}

std::optional<int64_t> ReadIntegerMapValue(
    const flutter::EncodableValue* argument, const std::string& key) {
  if (argument == nullptr) {
    return std::nullopt;
  }
  const auto* values = std::get_if<flutter::EncodableMap>(argument);
  if (values == nullptr) {
    return std::nullopt;
  }
  const auto found = values->find(flutter::EncodableValue(key));
  if (found == values->end()) {
    return std::nullopt;
  }
  if (const auto* value = std::get_if<int32_t>(&found->second)) {
    return *value;
  }
  if (const auto* value = std::get_if<int64_t>(&found->second)) {
    return *value;
  }
  return std::nullopt;
}

std::optional<std::string> ReadStringMapValue(
    const flutter::EncodableValue* argument, const std::string& key) {
  if (argument == nullptr) {
    return std::nullopt;
  }
  const auto* values = std::get_if<flutter::EncodableMap>(argument);
  if (values == nullptr) {
    return std::nullopt;
  }
  const auto found = values->find(flutter::EncodableValue(key));
  if (found == values->end()) {
    return std::nullopt;
  }
  const auto* value = std::get_if<std::string>(&found->second);
  return value == nullptr ? std::nullopt : std::optional<std::string>(*value);
}

}  // namespace

FlutterWindow::FlutterWindow(const flutter::DartProject& project)
    : project_(project) {}

FlutterWindow::~FlutterWindow() {}

bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  RECT frame = GetClientArea();

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  // Ensure that basic setup of the controller was successful.
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  RegisterPlugins(flutter_controller_->engine());
  windows_pairing_channel_ =
      std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
          flutter_controller_->engine()->messenger(), kWindowsPairingChannel,
          &flutter::StandardMethodCodec::GetInstance());
  windows_pairing_channel_->SetMethodCallHandler(
      [](const flutter::MethodCall<flutter::EncodableValue>& call,
         std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
             result) {
        if (call.method_name() == kPreparePairingRuntimeMethod) {
          std::array<uint8_t, 16> device_id{};
          if (!PreparePairingRuntime(&device_id)) {
            result->Error("pairing_runtime_unavailable",
                          "Windows could not prepare local pairing support.");
            return;
          }
          result->Success(flutter::EncodableValue(FormatDeviceId(device_id)));
          return;
        }

        if (call.method_name() == kBeginSenderPairingMethod) {
          const auto* invitation_uri =
              std::get_if<std::string>(call.arguments());
          if (invitation_uri == nullptr) {
            result->Error("invalid_argument", "A pairing invitation is required.");
            return;
          }
          uint64_t request_id = 0;
          const int32_t status = BeginSenderPairing(*invitation_uri, &request_id);
          if (status != 0) {
            result->Error(status == 3 ? "pairing_busy" : "pairing_unavailable",
                          status == 3
                              ? "A Windows pairing attempt is already active."
                              : "Windows could not start sender pairing.");
            return;
          }
          result->Success(flutter::EncodableValue(
              static_cast<int64_t>(request_id)));
          return;
        }

        if (call.method_name() == kPollSenderPairingMethod) {
          const auto request_id = ReadPairingRequestId(call.arguments());
          if (!request_id.has_value()) {
            result->Error("invalid_argument", "A valid pairing request ID is required.");
            return;
          }
          SenderPairingSnapshot snapshot{};
          if (!PollSenderPairing(*request_id, &snapshot)) {
            result->Error("pairing_unavailable",
                          "Windows could not read sender pairing status.");
            return;
          }
          result->Success(PairingStatusValue(snapshot));
          return;
        }

        if (call.method_name() == kCancelSenderPairingMethod) {
          const auto request_id = ReadPairingRequestId(call.arguments());
          if (!request_id.has_value()) {
            result->Error("invalid_argument", "A valid pairing request ID is required.");
            return;
          }
          const int32_t status = CancelSenderPairing(*request_id);
          if (status != 0) {
            result->Error("pairing_unavailable",
                          "Windows could not cancel sender pairing.");
            return;
          }
          result->Success(flutter::EncodableValue(true));
          return;
        }

        if (call.method_name() == kBeginSenderSessionMethod) {
          const auto request_id = ReadIntegerMapValue(call.arguments(), "requestId");
          const auto sdp = ReadStringMapValue(call.arguments(), "sdp");
          if (!request_id.has_value() || *request_id <= 0 || !sdp.has_value()) {
            result->Error("invalid_argument",
                          "A pairing request ID and session offer are required.");
            return;
          }
          std::string session_id;
          const int32_t status = BeginSenderSession(
              static_cast<uint64_t>(*request_id), *sdp, &session_id);
          if (status != 0) {
            result->Error(
                status == 3 ? "session_busy"
                            : (status == 5 ? "session_not_ready"
                                           : "session_unavailable"),
                status == 3
                    ? "A sender session is already active."
                    : (status == 5 ? "Pairing must be approved before streaming."
                                   : "Windows could not start the sender session."));
            return;
          }
          result->Success(flutter::EncodableValue(session_id));
          return;
        }

        if (call.method_name() == kSendSenderSessionIceMethod) {
          const auto request_id = ReadIntegerMapValue(call.arguments(), "requestId");
          const auto candidate =
              ReadStringMapValue(call.arguments(), "candidate");
          const auto media_id = ReadStringMapValue(call.arguments(), "mediaId");
          const auto media_line_index =
              ReadIntegerMapValue(call.arguments(), "mediaLineIndex");
          if (!request_id.has_value() || *request_id <= 0 ||
              !candidate.has_value() || !media_id.has_value() ||
              !media_line_index.has_value() || *media_line_index < 0 ||
              *media_line_index > 1024) {
            result->Error("invalid_argument",
                          "A valid session request and ICE candidate are required.");
            return;
          }
          const int32_t status = SendSenderSessionIce(
              static_cast<uint64_t>(*request_id), *media_id,
              static_cast<uint32_t>(*media_line_index), *candidate);
          if (status != 0) {
            result->Error(status == 5 ? "session_not_ready"
                                     : "session_unavailable",
                          status == 5
                              ? "Start a sender session before sending ICE."
                              : "Windows could not send the ICE candidate.");
            return;
          }
          result->Success(flutter::EncodableValue(true));
          return;
        }

        if (call.method_name() == kStartSenderSessionMethod) {
          const auto request_id = ReadIntegerMapValue(call.arguments(), "requestId");
          const auto source_id = ReadStringMapValue(call.arguments(), "sourceId");
          if (!request_id.has_value() || *request_id <= 0 || !source_id.has_value()) {
            result->Error("invalid_argument",
                          "A pairing request ID and source ID are required.");
            return;
          }
          const int32_t status = StartSenderSession(
              static_cast<uint64_t>(*request_id), *source_id);
          if (status != 0) {
            result->Error(status == 5 ? "session_not_ready"
                                     : (status == 3 ? "session_busy"
                                                    : "session_unavailable"),
                          status == 5
                              ? "The peer connection must be ready before capture starts."
                              : (status == 3 ? "The session start was already requested."
                                             : "Windows could not report the active source."));
            return;
          }
          result->Success(flutter::EncodableValue(true));
          return;
        }

        if (call.method_name() == kStopSenderSessionMethod) {
          const auto request_id = ReadIntegerMapValue(call.arguments(), "requestId");
          const auto reason = ReadStringMapValue(call.arguments(), "reason");
          if (!request_id.has_value() || *request_id <= 0 || !reason.has_value()) {
            result->Error("invalid_argument",
                          "A pairing request ID and stop reason are required.");
            return;
          }
          const int32_t status = StopSenderSession(
              static_cast<uint64_t>(*request_id), *reason);
          if (status != 0) {
            result->Error(status == 5 ? "session_not_ready"
                                     : "session_stop_unavailable",
                          status == 5
                              ? "There is no active sender session to stop."
                              : "Windows could not send the session stop message.");
            return;
          }
          result->Success(flutter::EncodableValue(true));
          return;
        }

        if (call.method_name() == kPollSenderSessionEventMethod) {
          const auto request_id = ReadPairingRequestId(call.arguments());
          if (!request_id.has_value()) {
            result->Error("invalid_argument", "A valid pairing request ID is required.");
            return;
          }
          std::string event_json;
          const int32_t status =
              PollSenderSessionEvent(*request_id, &event_json);
          if (status != 0) {
            result->Error(status == 5 ? "session_not_ready"
                                     : "session_unavailable",
                          status == 5
                              ? "Start a sender session before polling events."
                              : "Windows could not read the sender session channel.");
            return;
          }
          result->Success(flutter::EncodableValue(event_json));
          return;
        }

        result->NotImplemented();
      });
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    this->Show();
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::OnDestroy() {
  ShutdownSenderPairing();
  if (windows_pairing_channel_) {
    windows_pairing_channel_->SetMethodCallHandler(nullptr);
    windows_pairing_channel_.reset();
  }
  if (flutter_controller_) {
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  // Give Flutter, including plugins, an opportunity to handle window messages.
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }
  }

  switch (message) {
    case WM_FONTCHANGE:
      flutter_controller_->engine()->ReloadSystemFonts();
      break;
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}
