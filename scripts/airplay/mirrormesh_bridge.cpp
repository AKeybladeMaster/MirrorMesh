// MirrorMesh UxPlay integration. SPDX-License-Identifier: GPL-3.0-or-later
// Private inherited pipes carry bounded status/consent only. Never send media over IPC.
#include "mirrormesh_bridge.h"
#ifdef _WIN32
#include <windows.h>
#include <atomic>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <mutex>
#include <string>
#include <gst/gst.h>
#include "renderers/video_renderer.h"

static HWND parent_window;
static std::mutex output_guard, approval_guard;
static std::atomic<unsigned long> next_request(1);
static HANDLE status_pipe;

bool mm_bridge_enabled() { return std::getenv("MIRRORMESH_AIRPLAY_PARENT") != nullptr; }

static void event(const std::string &value) {
    if (!mm_bridge_enabled()) return;
    std::lock_guard<std::mutex> lock(output_guard);
    const std::string line = "MM_AIRPLAY_" + value + "\n";
    DWORD written = 0;
    if (!status_pipe || !WriteFile(status_pipe, line.data(), static_cast<DWORD>(line.size()), &written, nullptr)
        || written != line.size()) std::exit(2);
}

static LRESULT CALLBACK video_window_proc(HWND window, UINT message, WPARAM wp, LPARAM lp) {
    if (message == WM_KEYDOWN && (wp == VK_F11 || wp == VK_ESCAPE)) {
        PostMessageW(GetAncestor(parent_window, GA_ROOT), WM_APP + 91, wp, 0);
        return 0;
    }
    if (message == WM_TIMER) {
        if (!IsWindow(parent_window)) { DestroyWindow(window); return 0; }
        RECT area, own;
        GetClientRect(parent_window, &area);
        GetClientRect(window, &own);
        if (area.right != own.right || area.bottom != own.bottom)
            MoveWindow(window, 0, 0, area.right, area.bottom, TRUE);
        return 0;
    }
    if (message == WM_DESTROY) { PostQuitMessage(0); return 0; }
    return DefWindowProcW(window, message, wp, lp);
}

struct WindowStart { HANDLE ready; HWND child; };
static DWORD WINAPI window_thread(void *argument) {
    WindowStart *start = static_cast<WindowStart *>(argument);
    typedef HANDLE (WINAPI *SetThreadDpi)(HANDLE);
    SetThreadDpi set_dpi = reinterpret_cast<SetThreadDpi>(GetProcAddress(GetModuleHandleW(L"user32.dll"), "SetThreadDpiAwarenessContext"));
    if (set_dpi) set_dpi(reinterpret_cast<HANDLE>(static_cast<intptr_t>(-4)));
    WNDCLASSW cls = {};
    cls.lpfnWndProc = video_window_proc;
    cls.hInstance = GetModuleHandleW(nullptr);
    cls.lpszClassName = L"MirrorMeshAirPlayVideo";
    cls.hbrBackground = static_cast<HBRUSH>(GetStockObject(BLACK_BRUSH));
    RegisterClassW(&cls);
    RECT area = {};
    GetClientRect(parent_window, &area);
    // The rendering HWND belongs to this process, so GStreamer can subclass it safely.
    // Only the native container parent belongs to the Flutter process.
    HWND child = CreateWindowExW(0, cls.lpszClassName, L"", WS_CHILD | WS_VISIBLE,
        0, 0, area.right, area.bottom, parent_window, nullptr, cls.hInstance, nullptr);
    start->child = child;
    if (child) SetTimer(child, 1, 40, nullptr);
    SetEvent(start->ready);
    if (!child) return 1;
    MSG message;
    while (GetMessageW(&message, nullptr, 0, 0) > 0) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
    }
    return 0;
}

void mm_bridge_init() {
    if (!mm_bridge_enabled()) return;
    char *end = nullptr;
    const char *text = std::getenv("MIRRORMESH_AIRPLAY_PARENT");
    parent_window = reinterpret_cast<HWND>(std::strtoull(text, &end, 10));
    if (!end || *end || !IsWindow(parent_window)) std::exit(2);
    text = std::getenv("MIRRORMESH_AIRPLAY_STATUS");
    if (!text) std::exit(2);
    status_pipe = reinterpret_cast<HANDLE>(std::strtoull(text, &end, 10));
    if (!end || *end || !status_pipe || status_pipe == INVALID_HANDLE_VALUE) std::exit(2);
    // Keep this allocation alive until the thread has acknowledged initialization.
    static WindowStart start = {};
    start.ready = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    HANDLE thread = start.ready ? CreateThread(nullptr, 0, window_thread, &start, 0, nullptr) : nullptr;
    if (!thread || WaitForSingleObject(start.ready, 10000) != WAIT_OBJECT_0 || !start.child) std::exit(2);
    CloseHandle(thread);
    CloseHandle(start.ready);
    video_renderer_set_window_handle(reinterpret_cast<uintptr_t>(start.child));
}

void mm_bridge_ready() { event("READY"); }
void mm_bridge_idle() { event("IDLE"); }
void mm_bridge_video_idle() { event("VIDEO_IDLE"); }
void mm_bridge_pin(const char *pin) {
    if (!pin || std::strlen(pin) != 4 || std::strspn(pin, "0123456789") != 4) return;
    event(std::string("PIN=") + pin);
}
extern "C" void mm_bridge_video() { event("VIDEO"); }
extern "C" void mm_bridge_error() { event("ERROR"); }

bool mm_bridge_approve() {
    if (!mm_bridge_enabled()) return true;
    std::unique_lock<std::mutex> lock(approval_guard, std::try_to_lock);
    if (!lock.owns_lock()) return false;
    const unsigned long id = next_request.fetch_add(1);
    event("REQUEST=" + std::to_string(id));
    HANDLE input = GetStdHandle(STD_INPUT_HANDLE);
    const ULONGLONG deadline = GetTickCount64() + 30000;
    std::string line;
    bool approved = false;
    while (GetTickCount64() < deadline) {
        DWORD available = 0, read = 0;
        if (!PeekNamedPipe(input, nullptr, 0, nullptr, &available, nullptr)) break;
        if (!available) { Sleep(25); continue; }
        char value;
        if (!ReadFile(input, &value, 1, &read, nullptr) || read != 1) break;
        if (value == '\n') {
            if (line == "allow " + std::to_string(id)) { approved = true; break; }
            if (line == "reject " + std::to_string(id)) break;
            line.clear();
        } else if (value != '\r') {
            line += value;
            if (line.size() > 64) break;
        }
    }
    event("DECIDED=" + std::to_string(id));
    return approved;
}
#endif
