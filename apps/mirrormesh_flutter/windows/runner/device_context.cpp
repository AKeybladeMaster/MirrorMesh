// Windows SDK networking headers require this include order.
// clang-format off
#include <winsock2.h>
#include <windows.h>
#include <ws2ipdef.h>
#include <ws2tcpip.h>
#include <iphlpapi.h>
#include <netioapi.h>
// clang-format on

#include "device_context.h"
#include <algorithm>
#include <array>
#include <cstdint>
#include <cwctype>
#include <limits>
#include <string>
#include <vector>

namespace {
std::string Text(const std::wstring &raw) {
  std::wstring value;
  for (wchar_t ch : raw) {
    if (iswcntrl(ch) || (ch >= 127 && ch <= 159) || ch == 0x061c ||
        ch == 0x200e || ch == 0x200f || (ch >= 0x202a && ch <= 0x202e) ||
        (ch >= 0x2066 && ch <= 0x2069))
      continue;
    if (value.size() >= 80)
      break;
    value.push_back(ch);
  }
  if (value.empty())
    return "";
  const int count = WideCharToMultiByte(
      CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
      static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
  if (count <= 0)
    return "";
  std::string output(count, '\0');
  WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, value.data(),
                      static_cast<int>(value.size()), output.data(), count,
                      nullptr, nullptr);
  return output;
}
std::string RegistryText(const wchar_t *key, const wchar_t *name) {
  std::array<wchar_t, 1024> value{};
  DWORD size = static_cast<DWORD>(value.size() * sizeof(wchar_t));
  if (RegGetValueW(HKEY_LOCAL_MACHINE, key, name,
                   RRF_RT_REG_SZ | RRF_ZEROONFAILURE, nullptr, value.data(),
                   &size) != ERROR_SUCCESS)
    return "";
  return Text(value.data());
}
std::string PreferredLanAddress() {
  ULONG size = 16 * 1024;
  std::vector<unsigned char> storage(size);
  ULONG status = ERROR_BUFFER_OVERFLOW;
  for (int retry = 0; retry < 3 && status == ERROR_BUFFER_OVERFLOW; ++retry) {
    if (size > 1024 * 1024)
      return "";
    storage.resize(size);
    status = GetAdaptersAddresses(
        AF_INET,
        GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_DNS_SERVER |
            GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST,
        nullptr, reinterpret_cast<IP_ADAPTER_ADDRESSES *>(storage.data()),
        &size);
  }
  if (status != NO_ERROR)
    return "";
  MIB_IPFORWARD_TABLE2 *routes = nullptr;
  if (GetIpForwardTable2(AF_INET, &routes) != NO_ERROR)
    return "";
  std::string selected;
  uint64_t best = std::numeric_limits<uint64_t>::max();
  bool best_routed = false;
  for (auto *adapter = reinterpret_cast<IP_ADAPTER_ADDRESSES *>(storage.data());
       adapter; adapter = adapter->Next) {
    if (adapter->OperStatus != IfOperStatusUp ||
        (adapter->IfType != IF_TYPE_ETHERNET_CSMACD &&
         adapter->IfType != IF_TYPE_IEEE80211))
      continue;
    MIB_IF_ROW2 interface_row{};
    interface_row.InterfaceLuid = adapter->Luid;
    if (GetIfEntry2(&interface_row) != NO_ERROR ||
        !interface_row.InterfaceAndOperStatusFlags.HardwareInterface ||
        interface_row.InterfaceAndOperStatusFlags.FilterInterface)
      continue;
    bool routed = false;
    uint64_t route_metric = std::numeric_limits<uint64_t>::max();
    for (ULONG index = 0; index < routes->NumEntries; ++index) {
      const auto &route = routes->Table[index];
      if (route.InterfaceIndex == adapter->IfIndex &&
          route.DestinationPrefix.PrefixLength == 0 &&
          route.NextHop.si_family == AF_INET &&
          route.NextHop.Ipv4.sin_addr.S_un.S_addr) {
        routed = true;
        route_metric =
            (std::min)(route_metric, static_cast<uint64_t>(route.Metric));
      }
    }
    const uint64_t metric = static_cast<uint64_t>(adapter->Ipv4Metric) +
                            (routed ? route_metric : 0);
    if (!selected.empty() &&
        ((best_routed && !routed) || (best_routed == routed && metric >= best)))
      continue;
    for (auto *address = adapter->FirstUnicastAddress; address;
         address = address->Next) {
      if (!address->Address.lpSockaddr ||
          address->Address.lpSockaddr->sa_family != AF_INET ||
          address->DadState != IpDadStatePreferred)
        continue;
      auto *socket =
          reinterpret_cast<sockaddr_in *>(address->Address.lpSockaddr);
      const ULONG host = ntohl(socket->sin_addr.S_un.S_addr);
      if (!host || (host >> 24) == 127 || (host >> 16) == 0xa9fe ||
          (host >> 28) >= 14)
        continue;
      std::array<char, INET_ADDRSTRLEN> text{};
      if (InetNtopA(AF_INET, &socket->sin_addr, text.data(),
                    static_cast<DWORD>(text.size()))) {
        selected = text.data();
        best = metric;
        best_routed = routed;
        break;
      }
    }
  }
  FreeMibTable(routes);
  return selected;
}
} // namespace

flutter::EncodableValue QueryDeviceContext() {
  using flutter::EncodableMap;
  using flutter::EncodableValue;
  std::array<wchar_t, 256> computer{};
  DWORD length = static_cast<DWORD>(computer.size());
  std::string name =
      GetComputerNameExW(ComputerNameDnsHostname, computer.data(), &length)
          ? Text(computer.data())
          : "Windows PC";
  if (name.empty())
    name = "Windows PC";
  const wchar_t *system = L"HARDWARE\\DESCRIPTION\\System\\BIOS";
  const wchar_t *version = L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
  std::string os = "Windows " + RegistryText(version, L"DisplayVersion") +
                   " (build " + RegistryText(version, L"CurrentBuildNumber") +
                   ")";
  return EncodableValue(EncodableMap{
      {EncodableValue("name"), EncodableValue(name)},
      {EncodableValue("platform"), EncodableValue("Windows")},
      {EncodableValue("manufacturer"),
       EncodableValue(RegistryText(system, L"SystemManufacturer"))},
      {EncodableValue("model"),
       EncodableValue(RegistryText(system, L"SystemProductName"))},
      {EncodableValue("os_version"), EncodableValue(os)},
      {EncodableValue("lan_address"), EncodableValue(PreferredLanAddress())}});
}
