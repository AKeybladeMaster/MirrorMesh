#pragma once
#include <string>

struct WirelessDisplayStatus {
  std::string wifi;
  std::string listening;
  std::string error;
};

// Read-only OS status. Call on a worker; the query has a five-second bound.
WirelessDisplayStatus QueryWirelessDisplayStatus();
std::string OpenWirelessDisplaySettings();

// Opens Microsoft's installed receiver. It owns consent, rendering and Stop.
// This does not create an embedded MirrorMesh Miracast session.
std::string OpenWirelessDisplay();
