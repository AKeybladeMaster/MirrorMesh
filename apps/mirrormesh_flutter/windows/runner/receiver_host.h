#ifndef MIRRORMESH_RECEIVER_HOST_H_
#define MIRRORMESH_RECEIVER_HOST_H_
#include "native_casting.h"
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
#include <future>
#include <memory>
#include <string>
#include <windows.h>

// Owns the isolated AirPlay process tree and native window. No media over IPC.
class ReceiverHost {
public:
  explicit ReceiverHost(HWND owner);
  ~ReceiverHost();
  void Handle(
      const flutter::MethodCall<flutter::EncodableValue> &call,
      std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>> result);
  void Key(WPARAM key);
  std::string DiagnoseStartup(bool await_pin = false);

private:
  bool Start();
  void Stop();
  void Poll();
  void Fullscreen(bool enabled);
  HWND owner_, stage_ = nullptr;
  HANDLE job_ = nullptr, process_ = nullptr, output_ = nullptr,
         input_ = nullptr;
  std::string buffered_, pin_, error_;
  std::string request_name_, request_model_, request_address_;
  int64_t request_ = 0;
  bool ready_ = false, video_ = false, fullscreen_ = false;
  bool overlay_ = false;
  ULONGLONG start_time_ = 0;
  ULONGLONG pin_deadline_ = 0;
  WINDOWPLACEMENT placement_{};
  LONG_PTR style_ = 0;
  std::future<WirelessDisplayStatus> wireless_status_;
};
#endif
