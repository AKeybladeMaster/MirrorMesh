#include "flutter_window.h"

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>

#include <windows.h>

#include "flutter/generated_plugin_registrant.h"

namespace {

constexpr char kWindowsPairingChannel[] = "com.mirrormesh/windows_pairing";
constexpr char kPreparePairingRuntimeMethod[] = "preparePairingRuntime";
constexpr char kPreparePairingRuntimeSymbol[] =
    "mirrormesh_windows_prepare_pairing_runtime";

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
        if (call.method_name() != kPreparePairingRuntimeMethod) {
          result->NotImplemented();
          return;
        }
        std::array<uint8_t, 16> device_id{};
        if (!PreparePairingRuntime(&device_id)) {
          result->Error("pairing_runtime_unavailable",
                        "Windows could not prepare local pairing support.");
          return;
        }
        result->Success(flutter::EncodableValue(FormatDeviceId(device_id)));
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
