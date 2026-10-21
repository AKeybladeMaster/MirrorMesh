#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>

#include "flutter_window.h"
#include "utils.h"

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  // Initialize COM, so that it is available for use in the library and/or
  // plugins.
  ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

  std::vector<std::string> command_line_arguments = GetCommandLineArguments();

  if (command_line_arguments.size() == 1 &&
      command_line_arguments[0] == "--diagnose-airplay") {
    // Explicit local startup diagnostics: briefly advertise the receiver,
    // reject every unapproved request, then close its owned process tree. No
    // Flutter UI, screen capture, client identity, PIN or media is logged.
    HWND owner = CreateWindowExW(
        0, L"STATIC", L"MirrorMesh startup diagnostics", WS_OVERLAPPEDWINDOW, 0,
        0, 1280, 720, nullptr, nullptr, instance, nullptr);
    std::string error = "Could not create the native diagnostic window.";
    if (owner) {
      {
        ReceiverHost receiver(owner);
        error = receiver.DiagnoseStartup();
      }
      DestroyWindow(owner);
    }
    const std::string output =
        error.empty() ? "AirPlay startup: READY. Receiver stopped; no client "
                        "approved.\r\n"
                      : "AirPlay startup: FAILED. " + error + "\r\n";
    DWORD written = 0;
    WriteFile(GetStdHandle(STD_OUTPUT_HANDLE), output.data(),
              static_cast<DWORD>(output.size()), &written, nullptr);
    ::CoUninitialize();
    return error.empty() ? EXIT_SUCCESS : EXIT_FAILURE;
  }

  flutter::DartProject project(L"data");
  project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

  FlutterWindow window(project);
  Win32Window::Point origin(10, 10);
  Win32Window::Size size(1280, 720);
  if (!window.Create(L"MirrorMesh", origin, size)) {
    return EXIT_FAILURE;
  }
  window.SetQuitOnClose(true);

  ::MSG msg;
  while (::GetMessage(&msg, nullptr, 0, 0)) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }

  ::CoUninitialize();
  return EXIT_SUCCESS;
}
