#include "device_context.h"
#include <algorithm>
#include <arpa/inet.h>
#include <array>
#include <fstream>
#include <ifaddrs.h>
#include <limits>
#include <map>
#include <net/if.h>
#include <sstream>
#include <string>
#include <sys/utsname.h>
#include <unistd.h>

namespace {
std::string clean(const std::string &input) {
  if (!g_utf8_validate(input.c_str(), static_cast<gssize>(input.size()),
                       nullptr))
    return "";
  std::string result;
  unsigned count = 0;
  for (const char *cursor = input.c_str(); *cursor && count < 80;
       cursor = g_utf8_next_char(cursor)) {
    const gunichar ch = g_utf8_get_char(cursor);
    if (g_unichar_iscntrl(ch) || ch == 0x061c || ch == 0x200e || ch == 0x200f ||
        (ch >= 0x202a && ch <= 0x202e) || (ch >= 0x2066 && ch <= 0x2069))
      continue;
    result.append(cursor, g_utf8_next_char(cursor) - cursor);
    ++count;
  }
  return result;
}
std::string public_file(const char *path) {
  std::ifstream input(path);
  std::string line;
  std::getline(input, line);
  return clean(line.substr(0, 1024));
}
std::string preferred_address() {
  std::map<std::string, unsigned long> routes;
  std::ifstream input("/proc/net/route");
  std::string line;
  std::getline(input, line);
  while (std::getline(input, line)) {
    std::istringstream fields(line);
    std::string name, destination, gateway, flags, refs, uses;
    unsigned long metric = 0;
    if (fields >> name >> destination >> gateway >> flags >> refs >> uses >>
        metric) {
      if (destination == "00000000" && gateway != "00000000") {
        auto found = routes.find(name);
        if (found == routes.end() || metric < found->second)
          routes[name] = metric;
      }
    }
  }
  ifaddrs *addresses = nullptr;
  if (getifaddrs(&addresses) != 0)
    return "";
  std::string selected, selected_name;
  unsigned long best = std::numeric_limits<unsigned long>::max();
  bool best_routed = false;
  for (auto *item = addresses; item; item = item->ifa_next) {
    if (!item->ifa_addr || item->ifa_addr->sa_family != AF_INET ||
        !(item->ifa_flags & IFF_UP) || !(item->ifa_flags & IFF_RUNNING) ||
        (item->ifa_flags & IFF_LOOPBACK))
      continue;
    const std::string name(item->ifa_name);
    const std::string device = "/sys/class/net/" + name + "/device";
    if (access(device.c_str(), F_OK) != 0)
      continue; // Exclude bridges, tunnels and virtual NICs.
    const auto found = routes.find(name);
    const bool routed = found != routes.end();
    const unsigned long metric = routed ? found->second : 0;
    if (!selected.empty() &&
        ((best_routed && !routed) ||
         (best_routed == routed &&
          (metric > best || (metric == best && name >= selected_name)))))
      continue;
    const auto *socket = reinterpret_cast<sockaddr_in *>(item->ifa_addr);
    const auto host = ntohl(socket->sin_addr.s_addr);
    if (!host || (host >> 24) == 127 || (host >> 16) == 0xa9fe ||
        (host >> 28) >= 14)
      continue;
    std::array<char, INET_ADDRSTRLEN> value{};
    if (inet_ntop(AF_INET, &socket->sin_addr, value.data(), value.size())) {
      selected = value.data();
      selected_name = name;
      best = metric;
      best_routed = routed;
    }
  }
  freeifaddrs(addresses);
  return selected;
}
} // namespace

FlValue *query_device_context() {
  FlValue *result = fl_value_new_map();
  std::array<char, 256> name{};
  const std::string hostname = gethostname(name.data(), name.size() - 1) == 0
                                   ? clean(name.data())
                                   : "Linux PC";
  utsname system{};
  uname(&system);
  const auto put = [result](const char *key, const std::string &value) {
    fl_value_set_string_take(result, key, fl_value_new_string(value.c_str()));
  };
  put("name", hostname.empty() ? "Linux PC" : hostname);
  put("platform", "Linux");
  put("manufacturer", public_file("/sys/devices/virtual/dmi/id/sys_vendor"));
  put("model", public_file("/sys/devices/virtual/dmi/id/product_name"));
  put("os_version", clean(std::string(system.sysname) + " " + system.release));
  put("lan_address", preferred_address());
  return result;
}
