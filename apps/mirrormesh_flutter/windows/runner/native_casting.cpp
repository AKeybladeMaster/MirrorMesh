#include "native_casting.h"
// clang-format off
#include <windows.h>
#include <appmodel.h>
// clang-format on
#include <shobjidl.h>
#include <vector>
#include <wrl/client.h>

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
