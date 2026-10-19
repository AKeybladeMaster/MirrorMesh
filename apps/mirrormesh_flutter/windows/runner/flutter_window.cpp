#include "flutter_window.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <variant>

#include <windows.h>

#include "flutter/generated_plugin_registrant.h"

namespace {

constexpr char kWindowsPairingChannel[] = "com.mirrormesh/windows_pairing";
constexpr char kPreparePairingRuntimeMethod[] = "preparePairingRuntime";
constexpr char kBeginSenderPairingMethod[] = "beginSenderPairing";
constexpr char kPollSenderPairingMethod[] = "pollSenderPairing";
constexpr char kCancelSenderPairingMethod[] = "cancelSenderPairing";
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
constexpr uint32_t kSenderPairingApprovedState = 3;

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
