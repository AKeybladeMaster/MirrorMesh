#include "receiver_host.h"
#include "device_context.h"
#include "native_casting.h"
#include <algorithm>
#include <array>
#include <charconv>
#include <cmath>
#include <cstdio>
#include <cwchar>
#include <map>
#include <optional>
#include <vector>

namespace {
using flutter::EncodableMap;
using flutter::EncodableValue;
std::optional<std::string> PublicField(const std::string &encoded) {
  if (encoded.size() > 640 || encoded.size() % 2)
    return std::nullopt;
  std::string decoded;
  for (size_t index = 0; index < encoded.size(); index += 2) {
    unsigned int byte = 0;
    const auto conversion = std::from_chars(
        encoded.data() + index, encoded.data() + index + 2, byte, 16);
    if (conversion.ec != std::errc{} ||
        conversion.ptr != encoded.data() + index + 2 || byte < 32 ||
        byte == 127)
      return std::nullopt;
    decoded.push_back(static_cast<char>(byte));
  }
  if (!decoded.empty() &&
      MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, decoded.data(),
                          static_cast<int>(decoded.size()), nullptr, 0) <= 0)
    return std::nullopt;
  return decoded;
}
void Close(HANDLE &handle) {
  if (handle) {
    CloseHandle(handle);
    handle = nullptr;
  }
}
std::wstring ModuleDirectory() {
  std::array<wchar_t, 32768> path{};
  DWORD count =
      GetModuleFileNameW(nullptr, path.data(), static_cast<DWORD>(path.size()));
  if (!count || count == path.size())
    return L"";
  std::wstring value(path.data(), count);
  return value.substr(0, value.find_last_of(L"\\/"));
}
std::string ExitError(DWORD code) {
  std::array<char, 11> hex{};
  std::snprintf(hex.data(), hex.size(), "0x%08lX",
                static_cast<unsigned long>(code));
  const std::string prefix =
      "AirPlay helper exited (" + std::string(hex.data()) + "). ";
  if (code == 0xC0000135 || code == ERROR_MOD_NOT_FOUND)
    return prefix + "A required DLL is missing. Extract the complete Windows "
                    "bundle into a new folder.";
  if (code == 0xC000007B)
    return prefix + "A runtime DLL has the wrong format. Use the complete "
                    "Windows x64 bundle.";
  if (code == 0xC000001D)
    return prefix + "The runtime used an unsupported CPU instruction.";
  return prefix + "Run the bundled diagnose-airplay.ps1 -CheckStartup for "
                  "local diagnostics.";
}
struct KeyLess {
  bool operator()(const std::wstring &a, const std::wstring &b) const {
    return _wcsicmp(a.c_str(), b.c_str()) < 0;
  }
};
std::vector<wchar_t> Environment(const std::wstring &runtime, HWND stage,
                                 HANDLE status) {
  std::map<std::wstring, std::wstring, KeyLess> values;
  LPWCH original = GetEnvironmentStringsW();
  if (original) {
    for (const wchar_t *item = original; *item; item += wcslen(item) + 1) {
      std::wstring entry(item);
      size_t split = entry.find(L'=', 1);
      if (split != std::wstring::npos)
        values[entry.substr(0, split)] = entry.substr(split + 1);
    }
    FreeEnvironmentStringsW(original);
  }
  values[L"PATH"] = runtime + L"\\bin;" + values[L"PATH"];
  values[L"GST_PLUGIN_PATH_1_0"] = runtime + L"\\lib\\gstreamer-1.0";
  values[L"GST_PLUGIN_SYSTEM_PATH_1_0"] = values[L"GST_PLUGIN_PATH_1_0"];
  values[L"GST_PLUGIN_SCANNER_1_0"] =
      runtime + L"\\libexec\\gstreamer-1.0\\gst-plugin-scanner.exe";
  const std::wstring cache = values[L"LOCALAPPDATA"] + L"\\MirrorMesh";
  CreateDirectoryW(cache.c_str(), nullptr);
  values[L"GST_REGISTRY_1_0"] = cache + L"\\airplay-gstreamer-registry.bin";
  values[L"MIRRORMESH_AIRPLAY_PARENT"] =
      std::to_wstring(reinterpret_cast<uintptr_t>(stage));
  values[L"MIRRORMESH_AIRPLAY_STATUS"] =
      std::to_wstring(reinterpret_cast<uintptr_t>(status));
  std::vector<wchar_t> result;
  for (const auto &item : values) {
    const std::wstring entry = item.first + L"=" + item.second;
    result.insert(result.end(), entry.begin(), entry.end());
    result.push_back(0);
  }
  result.push_back(0);
  return result;
}
std::optional<double> Number(const EncodableValue *value, const char *key) {
  if (!value)
    return std::nullopt;
  const auto *map = std::get_if<EncodableMap>(value);
  if (!map)
    return std::nullopt;
  auto found = map->find(EncodableValue(key));
  if (found == map->end())
    return std::nullopt;
  if (const auto *number = std::get_if<double>(&found->second)) {
    if (std::isfinite(*number) && *number >= 0 && *number <= 32768)
      return *number;
  }
  return std::nullopt;
}
} // namespace

ReceiverHost::ReceiverHost(HWND owner) : owner_(owner) {
  stage_ = CreateWindowExW(
      0, L"STATIC", L"", WS_CHILD | WS_CLIPSIBLINGS | SS_BLACKRECT, 0, 0, 1, 1,
      owner, nullptr, GetModuleHandleW(nullptr), nullptr);
}
ReceiverHost::~ReceiverHost() {
  Stop();
  if (stage_)
    DestroyWindow(stage_);
}

bool ReceiverHost::Start() {
  Stop();
  error_.clear();
  const std::wstring runtime = ModuleDirectory() + L"\\airplay";
  const std::wstring executable = runtime + L"\\bin\\uxplay.exe";
  if (!stage_ ||
      GetFileAttributesW(executable.c_str()) == INVALID_FILE_ATTRIBUTES) {
    error_ = "AirPlay components are missing. Reinstall the complete Windows "
             "bundle.";
    return false;
  }
  SECURITY_ATTRIBUTES security{sizeof(SECURITY_ATTRIBUTES), nullptr, TRUE};
  HANDLE child_output = nullptr, child_input = nullptr;
  HANDLE log =
      CreateFileW(L"NUL", GENERIC_WRITE, FILE_SHARE_READ | FILE_SHARE_WRITE,
                  &security, OPEN_EXISTING, 0, nullptr);
  if (log == INVALID_HANDLE_VALUE)
    log = nullptr;
  if (!log || !CreatePipe(&output_, &child_output, &security, 0) ||
      !SetHandleInformation(output_, HANDLE_FLAG_INHERIT, 0) ||
      !CreatePipe(&child_input, &input_, &security, 0) ||
      !SetHandleInformation(input_, HANDLE_FLAG_INHERIT, 0)) {
    Close(child_output);
    Close(child_input);
    Close(log);
    Stop();
    error_ = "Could not create private AirPlay control pipes.";
    return false;
  }
  job_ = CreateJobObjectW(nullptr, nullptr);
  JOBOBJECT_EXTENDED_LIMIT_INFORMATION limits{};
  limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
  if (!job_ || !SetInformationJobObject(job_, JobObjectExtendedLimitInformation,
                                        &limits, sizeof(limits))) {
    Close(child_output);
    Close(child_input);
    Close(log);
    Stop();
    error_ = "Could not contain the AirPlay process.";
    return false;
  }
  SIZE_T size = 0;
  InitializeProcThreadAttributeList(nullptr, 1, 0, &size);
  std::vector<unsigned char> storage(size);
  auto *attributes =
      reinterpret_cast<LPPROC_THREAD_ATTRIBUTE_LIST>(storage.data());
  bool initialized =
      InitializeProcThreadAttributeList(attributes, 1, 0, &size) != FALSE;
  HANDLE inherited[] = {child_output, child_input, log};
  bool configured =
      initialized && UpdateProcThreadAttribute(
                         attributes, 0, PROC_THREAD_ATTRIBUTE_HANDLE_LIST,
                         inherited, sizeof(inherited), nullptr, nullptr);
  STARTUPINFOEXW startup{};
  startup.StartupInfo.cb = sizeof(startup);
  startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES | STARTF_USESHOWWINDOW;
  startup.StartupInfo.wShowWindow = SW_HIDE;
  startup.StartupInfo.hStdInput = child_input;
  startup.StartupInfo.hStdOutput = startup.StartupInfo.hStdError = log;
  startup.lpAttributeList = attributes;
  // Fixed argv: fresh authentication, no persisted AirPlay keys/config,
  // recording or HLS.
  std::wstring command =
      L"\"" + executable +
      L"\" -n \"MirrorMesh PC\" -nh -pw -key -p 7100 -s 1920x1080@60 -vs "
      L"d3d11videosink -as wasapisink -vsync no";
  auto environment = Environment(runtime, stage_, child_output);
  PROCESS_INFORMATION info{};
  bool launched =
      configured &&
      CreateProcessW(
          executable.c_str(), command.data(), nullptr, nullptr, TRUE,
          CREATE_NO_WINDOW | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT |
              EXTENDED_STARTUPINFO_PRESENT,
          environment.data(), runtime.c_str(), &startup.StartupInfo, &info);
  if (initialized)
    DeleteProcThreadAttributeList(attributes);
  Close(child_output);
  Close(child_input);
  Close(log);
  if (!launched) {
    Stop();
    error_ =
        "AirPlay could not start. Check the packaged runtime dependencies.";
    return false;
  }
  process_ = info.hProcess;
  if (!AssignProcessToJobObject(job_, process_) ||
      ResumeThread(info.hThread) == static_cast<DWORD>(-1)) {
    TerminateProcess(process_, 1);
    CloseHandle(info.hThread);
    Stop();
    error_ = "AirPlay could not enter its contained process.";
    return false;
  }
  CloseHandle(info.hThread);
  start_time_ = GetTickCount64();
  return true;
}

void ReceiverHost::Stop() {
  if (stage_)
    ShowWindow(stage_, SW_HIDE);
  if (job_)
    TerminateJobObject(job_, 0);
  Close(process_);
  Close(job_);
  Close(output_);
  Close(input_);
  buffered_.clear();
  pin_.clear();
  pin_deadline_ = 0;
  request_ = 0;
  request_name_.clear();
  request_model_.clear();
  request_address_.clear();
  ready_ = video_ = overlay_ = false;
  Fullscreen(false);
}

void ReceiverHost::Poll() {
  if (!process_)
    return;
  DWORD available = 0, count = 0;
  std::array<char, 4096> buffer{};
  bool media_error = false;
  // Dedicated pipe excludes upstream logs. Request labels use bounded hex
  // fields.
  for (int batch = 0;
       batch < 16 &&
       PeekNamedPipe(output_, nullptr, 0, nullptr, &available, nullptr) &&
       available;
       ++batch) {
    if (!ReadFile(output_, buffer.data(),
                  (std::min)(available, static_cast<DWORD>(buffer.size())),
                  &count, nullptr))
      break;
    buffered_.append(buffer.data(), count);
    size_t end;
    while ((end = buffered_.find('\n')) != std::string::npos) {
      std::string line = buffered_.substr(0, end);
      buffered_.erase(0, end + 1);
      if (line == "MM_AIRPLAY_VIDEO_IDLE")
        video_ = false;
      else if (line == "MM_AIRPLAY_ERROR")
        media_error = true;
      else if (line == "MM_AIRPLAY_READY")
        ready_ = true;
      else if (line == "MM_AIRPLAY_VIDEO") {
        video_ = true;
        pin_.clear();
        pin_deadline_ = 0;
      } else if (line == "MM_AIRPLAY_IDLE") {
        video_ = false;
        // An iOS device can close its challenge socket while its PIN prompt is
        // open. A transport gap must not erase the active authentication code.
        request_ = 0;
      } else if (line.rfind("MM_AIRPLAY_PIN=", 0) == 0) {
        // A second device's unauthenticated challenge must not attach a new
        // PIN-expiry timer to an already approved stream or approval request.
        if (video_ || request_ > 0)
          continue;
        std::string code = line.substr(15);
        if (code.size() == 4 &&
            code.find_first_not_of("0123456789") == std::string::npos) {
          if (pin_ != code || !pin_deadline_)
            pin_deadline_ = GetTickCount64() + 120000;
          pin_ = code;
        }
      } else if (line.rfind("MM_AIRPLAY_REQUEST=", 0) == 0) {
        std::array<std::string, 4> fields;
        size_t start = 19;
        bool valid = true;
        for (size_t index = 0; index < fields.size(); ++index) {
          const size_t split = line.find('\t', start);
          if ((index < 3 && split == std::string::npos) ||
              (index == 3 && split != std::string::npos)) {
            valid = false;
            break;
          }
          fields[index] = line.substr(
              start, split == std::string::npos ? split : split - start);
          start = split == std::string::npos ? line.size() : split + 1;
        }
        if (!valid)
          continue;
        const std::string &id = fields[0];
        const auto name = PublicField(fields[1]),
                   model = PublicField(fields[2]),
                   address = PublicField(fields[3]);
        int64_t parsed = 0;
        const auto conversion =
            std::from_chars(id.data(), id.data() + id.size(), parsed);
        if (conversion.ec == std::errc{} &&
            conversion.ptr == id.data() + id.size() && parsed > 0 && name &&
            model && address) {
          request_ = parsed;
          request_name_ = *name;
          request_model_ = *model;
          request_address_ = *address;
          pin_.clear();
          pin_deadline_ = 0;
        }
      } else if (line.rfind("MM_AIRPLAY_DECIDED=", 0) == 0)
        request_ = 0;
    }
    if (buffered_.size() > 4096)
      buffered_.clear();
  }
  if (stage_)
    ShowWindow(stage_, video_ && !overlay_ ? SW_SHOWNA : SW_HIDE);
  if (pin_deadline_ && GetTickCount64() >= pin_deadline_) {
    // End the helper too, so hiding an expired code cannot leave it usable.
    Stop();
    error_ = "The AirPlay code expired. Start the receiver again and select "
             "MirrorMesh PC on your device.";
    return;
  }
  const bool exited = WaitForSingleObject(process_, 0) == WAIT_OBJECT_0;
  const bool timed_out = !ready_ && GetTickCount64() - start_time_ > 45000;
  if (media_error || exited || timed_out) {
    DWORD code = 0;
    if (exited)
      GetExitCodeProcess(process_, &code);
    std::string message =
        media_error ? "AirPlay media initialization failed. Check the bundled "
                      "decoder and rendering runtime."
        : exited    ? ExitError(code)
                    : "AirPlay did not become ready within 45 seconds. Check "
                      "local-network permissions and port availability.";
    Stop();
    error_ = std::move(message);
  }
}

std::string ReceiverHost::DiagnoseStartup(bool await_pin) {
  // Explicit CLI diagnostics use the same process, private pipes, PIN and
  // consent boundary as the UI. Never approve a client or start capture in this
  // mode.
  if (!Start())
    return error_;
  const ULONGLONG deadline = GetTickCount64() + 15000;
  ULONGLONG ready_since = 0;
  bool stable = false;
  while (process_ && GetTickCount64() < deadline) {
    MSG message;
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
      TranslateMessage(&message);
      DispatchMessageW(&message);
    }
    Poll();
    if (process_ && ready_ && (!await_pin || !pin_.empty())) {
      if (!ready_since)
        ready_since = GetTickCount64();
      // READY announces discovery registration. Keep polling afterward so an
      // initial asynchronous GStreamer bus error cannot pass this diagnostic.
      if (GetTickCount64() - ready_since >= 2000) {
        stable = true;
        break;
      }
    }
    if (!ready_ || (await_pin && pin_.empty()))
      ready_since = 0;
    if (process_)
      Sleep(25);
  }
  const std::string result =
      stable ? ""
      : error_.empty()
          ? await_pin
                ? "AirPlay authentication code did not remain available for "
                  "two seconds within the 15-second diagnostic window."
                : "AirPlay startup did not remain Ready for two seconds within "
                  "the 15-second diagnostic window."
          : error_;
  Stop();
  return result;
}

void ReceiverHost::Fullscreen(bool enabled) {
  if (enabled == fullscreen_)
    return;
  if (enabled) {
    placement_.length = sizeof(placement_);
    GetWindowPlacement(owner_, &placement_);
    style_ = GetWindowLongPtrW(owner_, GWL_STYLE);
    MONITORINFO monitor{};
    monitor.cbSize = sizeof(monitor);
    if (!GetMonitorInfoW(MonitorFromWindow(owner_, MONITOR_DEFAULTTONEAREST),
                         &monitor))
      return;
    SetWindowLongPtrW(owner_, GWL_STYLE,
                      style_ & ~static_cast<LONG_PTR>(WS_OVERLAPPEDWINDOW));
    SetWindowPos(owner_, nullptr, monitor.rcMonitor.left, monitor.rcMonitor.top,
                 monitor.rcMonitor.right - monitor.rcMonitor.left,
                 monitor.rcMonitor.bottom - monitor.rcMonitor.top,
                 SWP_NOZORDER | SWP_FRAMECHANGED);
  } else {
    SetWindowLongPtrW(owner_, GWL_STYLE, style_);
    SetWindowPlacement(owner_, &placement_);
    SetWindowPos(owner_, nullptr, 0, 0, 0, 0,
                 SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED);
  }
  fullscreen_ = enabled;
}
void ReceiverHost::Key(WPARAM key) {
  if (!process_)
    return;
  if (key == VK_F11)
    Fullscreen(!fullscreen_);
  else if (key == VK_ESCAPE)
    Fullscreen(false);
}

void ReceiverHost::Handle(
    const flutter::MethodCall<EncodableValue> &call,
    std::unique_ptr<flutter::MethodResult<EncodableValue>> result) {
  if (call.method_name() == "device_context") {
    result->Success(QueryDeviceContext());
    return;
  }
  if (call.method_name() == "open_wireless_display") {
    const std::string error = OpenWirelessDisplay();
    if (error.empty())
      result->Success();
    else
      result->Error("wireless_display_unavailable", error);
    return;
  }
  if (call.method_name() == "wireless_display_status") {
    if (!wireless_status_.valid())
      wireless_status_ =
          std::async(std::launch::async, QueryWirelessDisplayStatus);
    if (wireless_status_.wait_for(std::chrono::seconds(0)) !=
        std::future_status::ready) {
      result->Success(EncodableValue(
          EncodableMap{{EncodableValue("pending"), EncodableValue(true)}}));
    } else {
      const auto status = wireless_status_.get();
      result->Success(EncodableValue(EncodableMap{
          {EncodableValue("pending"), EncodableValue(false)},
          {EncodableValue("wifi"), EncodableValue(status.wifi)},
          {EncodableValue("listening"), EncodableValue(status.listening)},
          {EncodableValue("error"), EncodableValue(status.error)}}));
    }
    return;
  }
  if (call.method_name() == "wireless_display_settings") {
    const auto error = OpenWirelessDisplaySettings();
    if (error.empty())
      result->Success();
    else
      result->Error("settings_unavailable", error);
    return;
  }
  if (call.method_name() == "fullscreen") {
    const auto *enabled =
        call.arguments() ? std::get_if<bool>(call.arguments()) : nullptr;
    if (!enabled) {
      result->Error("invalid_argument", "Expected a full-screen setting.");
      return;
    }
    Fullscreen(*enabled);
    result->Success();
    return;
  }
  if (call.method_name() == "airplay_start") {
    if (!Start())
      result->Error("airplay_unavailable", error_);
    else
      result->Success();
    return;
  }
  if (call.method_name() == "airplay_stop") {
    Stop();
    result->Success();
    return;
  }
  if (call.method_name() == "airplay_poll") {
    Poll();
    result->Success(EncodableValue(EncodableMap{
        {EncodableValue("running"), EncodableValue(process_ != nullptr)},
        {EncodableValue("ready"), EncodableValue(ready_)},
        {EncodableValue("video"), EncodableValue(video_)},
        {EncodableValue("pin"), EncodableValue(pin_)},
        {EncodableValue("request"), EncodableValue(request_)},
        {EncodableValue("device"),
         EncodableValue(EncodableMap{
             {EncodableValue("display_name"), EncodableValue(request_name_)},
             {EncodableValue("source_ip"), EncodableValue(request_address_)},
             {EncodableValue("requested_grants"),
              EncodableValue(EncodableMap{
                  {EncodableValue("view"), EncodableValue(true)}})},
             {EncodableValue("device_details"),
              EncodableValue(EncodableMap{
                  {EncodableValue("model"), EncodableValue(request_model_)},
                  {EncodableValue("manufacturer"), EncodableValue("")},
                  {EncodableValue("platform"),
                   EncodableValue("AirPlay sender")}})}})},
        {EncodableValue("fullscreen"), EncodableValue(fullscreen_)},
        {EncodableValue("error"), EncodableValue(error_)}}));
    return;
  }
  if (call.method_name() == "airplay_overlay") {
    const auto *enabled =
        call.arguments() ? std::get_if<bool>(call.arguments()) : nullptr;
    if (!enabled) {
      result->Error("invalid_argument", "Expected an overlay setting.");
      return;
    }
    overlay_ = *enabled;
    if (stage_)
      ShowWindow(stage_, video_ && !overlay_ ? SW_SHOWNA : SW_HIDE);
    result->Success();
    return;
  }
  if (call.method_name() == "airplay_decide") {
    const auto *map = call.arguments()
                          ? std::get_if<EncodableMap>(call.arguments())
                          : nullptr;
    if (!map || !input_ || request_ <= 0) {
      result->Error("request_expired", "The AirPlay request has expired.");
      return;
    }
    auto id = map->find(EncodableValue("request")),
         decision = map->find(EncodableValue("allow"));
    int64_t actual = 0;
    if (id != map->end()) {
      if (auto *value = std::get_if<int64_t>(&id->second))
        actual = *value;
      else if (auto *value32 = std::get_if<int32_t>(&id->second))
        actual = *value32;
    }
    const auto *allow =
        decision != map->end() ? std::get_if<bool>(&decision->second) : nullptr;
    if (actual != request_ || !allow) {
      result->Error("request_expired", "The AirPlay request has changed.");
      return;
    }
    const std::string line = std::string(*allow ? "allow " : "reject ") +
                             std::to_string(request_) + "\n";
    DWORD written = 0;
    if (!WriteFile(input_, line.data(), static_cast<DWORD>(line.size()),
                   &written, nullptr) ||
        written != line.size()) {
      result->Error("control_failed",
                    "AirPlay consent could not be delivered.");
      return;
    }
    request_ = 0;
    result->Success();
    return;
  }
  if (call.method_name() == "airplay_bounds") {
    const auto x = Number(call.arguments(), "x"),
               y = Number(call.arguments(), "y"),
               width = Number(call.arguments(), "width"),
               height = Number(call.arguments(), "height");
    if (!x || !y || !width || !height || *width < 1 || *height < 1) {
      result->Error("invalid_bounds", "The video area is unavailable.");
      return;
    }
    if (stage_)
      SetWindowPos(stage_, HWND_TOP, static_cast<int>(*x), static_cast<int>(*y),
                   static_cast<int>(*width), static_cast<int>(*height),
                   SWP_NOACTIVATE);
    result->Success();
    return;
  }
  result->NotImplemented();
}
