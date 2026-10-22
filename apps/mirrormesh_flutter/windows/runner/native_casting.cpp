#include "native_casting.h"
// clang-format off
#include <windows.h>
#include <appmodel.h>
// clang-format on
#include <chrono>
#include <shellapi.h>
#include <shobjidl.h>
#include <vector>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Media.Miracast.h>
#include <wrl/client.h>

WirelessDisplayStatus QueryWirelessDisplayStatus() {
  using namespace winrt::Windows::Media::Miracast;
  WirelessDisplayStatus result;
  try {
    winrt::init_apartment(winrt::apartment_type::multi_threaded);
    struct Apartment {
      ~Apartment() { winrt::uninit_apartment(); }
    } apartment;
    auto operation = MiracastReceiver().GetStatusAsync();
    if (operation.wait_for(std::chrono::seconds(5)) !=
        winrt::Windows::Foundation::AsyncStatus::Completed) {
      operation.Cancel();
      result.error = "Windows did not return Miracast status in time.";
      return result;
    }
    const auto status = operation.GetResults();
    switch (status.WiFiStatus()) {
    case MiracastReceiverWiFiStatus::MiracastSupported:
      result.wifi = "supported";
      break;
    case MiracastReceiverWiFiStatus::MiracastSupportNotOptimized:
      result.wifi = "limited";
      break;
    case MiracastReceiverWiFiStatus::MiracastNotSupported:
      result.wifi = "unsupported";
      break;
    default:
      result.wifi = "unknown";
    }
    switch (status.ListeningStatus()) {
    case MiracastReceiverListeningStatus::Listening:
      result.listening = "listening";
      break;
    case MiracastReceiverListeningStatus::ConnectionPending:
      result.listening = "pending";
      break;
    case MiracastReceiverListeningStatus::Connected:
      result.listening = "connected";
      break;
    case MiracastReceiverListeningStatus::DisabledByPolicy:
      result.listening = "policy";
      break;
    case MiracastReceiverListeningStatus::TemporarilyDisabled:
      result.listening = "disabled";
      break;
    default:
      result.listening = "not_listening";
    }
  } catch (const winrt::hresult_error &) {
    result.error = "Windows could not query Miracast. Check Projecting to this "
                   "PC in Settings.";
  } catch (...) {
    result.error =
        "Miracast status is unavailable on this Windows installation.";
  }
  return result;
}

std::string OpenWirelessDisplaySettings() {
  const auto result = reinterpret_cast<INT_PTR>(
      ShellExecuteW(nullptr, L"open", L"ms-settings:project", nullptr, nullptr,
                    SW_SHOWNORMAL));
  return result > 32 ? "" : "Windows could not open projection settings.";
}

std::string OpenWirelessDisplay() {
  // Exact Microsoft package family; no executable path or app ID from a peer.
  constexpr wchar_t family[] = L"Microsoft.PPIProjection_cw5n1h2txyewy";
  UINT32 count = 0, length = 0;
  LONG status =
      GetPackagesByPackageFamily(family, &count, nullptr, &length, nullptr);
  if (status != ERROR_INSUFFICIENT_BUFFER || count == 0 || count > 64 ||
      length > 1048576)
    return "Install the Windows Wireless Display optional feature in Settings "
           "> System > Projecting to this PC, then retry. A compatible Wi-Fi "
           "adapter and Miracast sender are required.";
  std::vector<PWSTR> names(count);
  std::vector<wchar_t> buffer(length);
  if (GetPackagesByPackageFamily(family, &count, names.data(), &length,
                                 buffer.data()) != ERROR_SUCCESS)
    return "Windows could not find the Wireless Display app. Check its "
           "installation in Windows Settings.";
  Microsoft::WRL::ComPtr<IApplicationActivationManager> manager;
  if (FAILED(CoCreateInstance(CLSID_ApplicationActivationManager, nullptr,
                              CLSCTX_INPROC_SERVER,
                              IID_PPV_ARGS(manager.GetAddressOf()))))
    return "Windows could not open Wireless Display. Open it from the Start "
           "menu.";
  for (UINT32 index = 0; index < count; ++index) {
    PACKAGE_INFO_REFERENCE package = nullptr;
    if (OpenPackageInfoByFullName(names[index], 0, &package) != ERROR_SUCCESS)
      continue;
    UINT32 bytes = 0, applications = 0;
    status = GetPackageApplicationIds(package, &bytes, nullptr, &applications);
    if (status != ERROR_INSUFFICIENT_BUFFER || !applications ||
        applications > 64 || bytes > 1048576) {
      ClosePackageInfo(package);
      continue;
    }
    std::vector<BYTE> ids(bytes);
    status =
        GetPackageApplicationIds(package, &bytes, ids.data(), &applications);
    ClosePackageInfo(package);
    if (status != ERROR_SUCCESS)
      continue;
    const auto *application_ids = reinterpret_cast<PCWSTR const *>(ids.data());
    for (UINT32 app = 0; app < applications; ++app) {
      DWORD process = 0;
      if (SUCCEEDED(manager->ActivateApplication(application_ids[app], nullptr,
                                                 AO_NONE, &process)))
        return "";
    }
  }
  return "Windows could not launch Wireless Display. Open it from the Start "
         "menu and check the receiver requirements it displays.";
}
