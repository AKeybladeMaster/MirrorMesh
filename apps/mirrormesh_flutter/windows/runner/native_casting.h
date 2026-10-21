#pragma once
#include <string>

// Opens Microsoft's installed receiver. It owns consent, rendering and Stop.
// This does not create an embedded MirrorMesh Miracast session.
std::string OpenWirelessDisplay();
