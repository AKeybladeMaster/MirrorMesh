#ifndef MIRRORMESH_RECEIVER_HOST_H_
#define MIRRORMESH_RECEIVER_HOST_H_
#include <flutter/method_channel.h>
#include <flutter/standard_method_codec.h>
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

private:
  bool Start();
  void Stop();
  void Poll();
  void Fullscreen(bool enabled);
  HWND owner_, stage_ = nullptr;
  HANDLE job_ = nullptr, process_ = nullptr, output_ = nullptr,
         input_ = nullptr;
  std::string buffered_, pin_, error_;
  int64_t request_ = 0;
  bool ready_ = false, video_ = false, fullscreen_ = false;
  ULONGLONG start_time_ = 0;
  WINDOWPLACEMENT placement_{};
  LONG_PTR style_ = 0;
};
#endif
