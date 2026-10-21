#pragma once
#include <flutter/standard_method_codec.h>

// Public OS facts and one routed LAN address. Never returns serials or private
// keys.
flutter::EncodableValue QueryDeviceContext();
