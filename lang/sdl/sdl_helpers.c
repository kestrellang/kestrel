#include <SDL.h>
#include <stdint.h>
#include <string.h>
#include <stddef.h>
#include <time.h>

// Wraps SDL_GetTicks for use as an FPS clock. SDL_Init must have run.
uint32_t Kestrel_GetTicks(void) { return SDL_GetTicks(); }

// Monotonic millisecond clock that does NOT require SDL_Init — needed for
// headless runs that skip the video subsystem.
int64_t Kestrel_MonotonicMs(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (int64_t)ts.tv_sec * 1000 + (int64_t)(ts.tv_nsec / 1000000);
}

// Event helpers
uint32_t Kestrel_GetEventType(SDL_Event* event) { return event->type; }
int32_t Kestrel_GetKeyScancode(SDL_Event* event) {
    if (event->type == SDL_KEYDOWN || event->type == SDL_KEYUP) return event->key.keysym.scancode;
    return 0;
}
int32_t Kestrel_GetMouseX(SDL_Event* event) {
    if (event->type == SDL_MOUSEBUTTONDOWN || event->type == SDL_MOUSEBUTTONUP) return event->button.x;
    if (event->type == SDL_MOUSEMOTION) return event->motion.x;
    return 0;
}
int32_t Kestrel_GetMouseY(SDL_Event* event) {
    if (event->type == SDL_MOUSEBUTTONDOWN || event->type == SDL_MOUSEBUTTONUP) return event->button.y;
    if (event->type == SDL_MOUSEMOTION) return event->motion.y;
    return 0;
}

// Which button a mouse button event is for (SDL_BUTTON_LEFT is 1).
int32_t Kestrel_GetMouseButton(SDL_Event* event) {
    if (event->type == SDL_MOUSEBUTTONDOWN || event->type == SDL_MOUSEBUTTONUP) return event->button.button;
    return 0;
}
// How far the mouse moved, also while relative mouse mode hides it.
int32_t Kestrel_GetMouseRelX(SDL_Event* event) {
    return event->type == SDL_MOUSEMOTION ? event->motion.xrel : 0;
}
int32_t Kestrel_GetMouseRelY(SDL_Event* event) {
    return event->type == SDL_MOUSEMOTION ? event->motion.yrel : 0;
}
// Wheel clicks: positive away from the user, whatever the OS's direction.
int32_t Kestrel_GetWheelY(SDL_Event* event) {
    if (event->type != SDL_MOUSEWHEEL) return 0;
    return event->wheel.direction == SDL_MOUSEWHEEL_FLIPPED ? -event->wheel.y : event->wheel.y;
}

// Pointer helpers
int32_t Kestrel_IsNull(void* ptr) { return ptr == NULL; }

// Use int64_t for all coordinates to ensure stack alignment with Kestrel
void* Kestrel_CreateWindow(const char* title, int64_t w, int64_t h) {
    return SDL_CreateWindow(title, SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED, (int)w, (int)h, SDL_WINDOW_SHOWN);
}

void* Kestrel_CreateRenderer(void* window) {
    return SDL_CreateRenderer((SDL_Window*)window, -1, SDL_RENDERER_ACCELERATED | SDL_RENDERER_PRESENTVSYNC);
}

void Kestrel_FillRect(void* renderer, int64_t x, int64_t y, int64_t w, int64_t h) {
    SDL_Rect r = {(int)x, (int)y, (int)w, (int)h};
    SDL_RenderFillRect((SDL_Renderer*)renderer, &r);
}

int64_t Kestrel_SetRenderDrawColor(void* renderer, int64_t r, int64_t g, int64_t b, int64_t a) {
    return SDL_SetRenderDrawColor((SDL_Renderer*)renderer, (uint8_t)r, (uint8_t)g, (uint8_t)b, (uint8_t)a);
}

// Simple Bitmap Font (5x7)
// 0-9, A-Z, space, [], :, !
static uint8_t font_bits[][5] = {
    ['0'] = {0x3E, 0x51, 0x49, 0x45, 0x3E},
    ['1'] = {0x00, 0x42, 0x7F, 0x40, 0x00},
    ['2'] = {0x42, 0x61, 0x51, 0x49, 0x46},
    ['3'] = {0x21, 0x41, 0x45, 0x4B, 0x31},
    ['4'] = {0x18, 0x14, 0x12, 0x7F, 0x10},
    ['5'] = {0x27, 0x45, 0x45, 0x45, 0x39},
    ['6'] = {0x3C, 0x4A, 0x49, 0x49, 0x30},
    ['7'] = {0x01, 0x71, 0x09, 0x05, 0x03},
    ['8'] = {0x36, 0x49, 0x49, 0x49, 0x36},
    ['9'] = {0x06, 0x49, 0x49, 0x29, 0x1E},
    ['A'] = {0x7C, 0x12, 0x11, 0x12, 0x7C},
    ['B'] = {0x7F, 0x49, 0x49, 0x49, 0x36},
    ['C'] = {0x3E, 0x41, 0x41, 0x41, 0x22},
    ['D'] = {0x7F, 0x41, 0x41, 0x22, 0x1C},
    ['E'] = {0x7F, 0x49, 0x49, 0x49, 0x41},
    ['F'] = {0x7F, 0x09, 0x09, 0x09, 0x01},
    ['G'] = {0x3E, 0x41, 0x49, 0x49, 0x7A},
    ['H'] = {0x7F, 0x08, 0x08, 0x08, 0x7F},
    ['I'] = {0x00, 0x41, 0x7F, 0x41, 0x00},
    ['J'] = {0x20, 0x40, 0x41, 0x3F, 0x01},
    ['K'] = {0x7F, 0x08, 0x14, 0x22, 0x41},
    ['L'] = {0x7F, 0x40, 0x40, 0x40, 0x40},
    ['M'] = {0x7F, 0x02, 0x0C, 0x02, 0x7F},
    ['N'] = {0x7F, 0x04, 0x08, 0x10, 0x7F},
    ['O'] = {0x3E, 0x41, 0x41, 0x41, 0x3E},
    ['P'] = {0x7F, 0x09, 0x09, 0x09, 0x06},
    ['Q'] = {0x3E, 0x41, 0x51, 0x21, 0x5E},
    ['R'] = {0x7F, 0x09, 0x19, 0x29, 0x46},
    ['S'] = {0x46, 0x49, 0x49, 0x49, 0x31},
    ['T'] = {0x01, 0x01, 0x7F, 0x01, 0x01},
    ['U'] = {0x3F, 0x40, 0x40, 0x40, 0x3F},
    ['V'] = {0x1F, 0x20, 0x40, 0x20, 0x1F},
    ['W'] = {0x3F, 0x40, 0x38, 0x40, 0x3F},
    ['X'] = {0x63, 0x14, 0x08, 0x14, 0x63},
    ['Y'] = {0x07, 0x08, 0x70, 0x08, 0x07},
    ['Z'] = {0x61, 0x51, 0x49, 0x45, 0x43},
    [' '] = {0x00, 0x00, 0x00, 0x00, 0x00},
    ['['] = {0x00, 0x7F, 0x41, 0x41, 0x00},
    [']'] = {0x00, 0x41, 0x41, 0x7F, 0x00},
    [':'] = {0x00, 0x00, 0x24, 0x00, 0x00},
    ['-'] = {0x08, 0x08, 0x08, 0x08, 0x08},
};

void Kestrel_DrawText(void* renderer, const char* text, int64_t x, int64_t y, int64_t scale) {
    if (!text) return;
    int cur_x = (int)x;
    for (int i = 0; i < strlen(text); i++) {
        char c = text[i];
        if (c >= 'a' && c <= 'z') c -= 32; // Uppercase only
        if (c > 127 || font_bits[(uint8_t)c][0] == 0 && c != ' ' && c != '1') {
            c = '?'; // Unknown
        }

        for (int col = 0; col < 5; col++) {
            uint8_t bits = font_bits[(uint8_t)c][col];
            for (int row = 0; row < 7; row++) {
                if (bits & (1 << row)) {
                    SDL_Rect r = {cur_x + col * (int)scale, (int)y + row * (int)scale, (int)scale, (int)scale};
                    SDL_RenderFillRect((SDL_Renderer*)renderer, &r);
                }
            }
        }
        cur_x += 6 * (int)scale;
    }
}

// --- Windows for GPU APIs (wgpu) ---
#include <SDL_syswm.h>

// A window with no SDL_Renderer, for a GPU API to draw into. On Apple
// platforms it is Metal-backed so it can host a CAMetalLayer.
void* Kestrel_CreateGPUWindow(const char* title, int64_t w, int64_t h) {
    Uint32 flags = SDL_WINDOW_SHOWN | SDL_WINDOW_RESIZABLE | SDL_WINDOW_ALLOW_HIGHDPI;
#ifdef __APPLE__
    flags |= SDL_WINDOW_METAL;
#endif
    return SDL_CreateWindow(title, SDL_WINDOWPOS_UNDEFINED, SDL_WINDOWPOS_UNDEFINED, (int)w, (int)h, flags);
}

// The window's drawable size in pixels (larger than its size on HiDPI).
void Kestrel_GetDrawableSize(void* window, int64_t* w, int64_t* h) {
    int iw = 0, ih = 0;
#if SDL_VERSION_ATLEAST(2, 26, 0)
    SDL_GetWindowSizeInPixels((SDL_Window*)window, &iw, &ih);
#else
    SDL_GetWindowSize((SDL_Window*)window, &iw, &ih);
#endif
    *w = iw;
    *h = ih;
}

// The platform handles behind a window. Returns 1 for X11 (a = Display*,
// id = Window), 2 for Wayland (a = wl_display*, b = wl_surface*), 3 for a
// Metal layer (a = CAMetalLayer*, made on first call), 4 for Windows
// (a = HINSTANCE, b = HWND), 0 when unsupported.
int32_t Kestrel_GetNativeWindow(void* window, void** a, void** b, uint64_t* id) {
    SDL_Window* win = (SDL_Window*)window;
    *a = NULL;
    *b = NULL;
    *id = 0;
#ifdef __APPLE__
    SDL_MetalView view = SDL_Metal_CreateView(win);
    if (!view) return 0;
    *a = SDL_Metal_GetLayer(view);
    return 3;
#else
    SDL_SysWMinfo info;
    SDL_VERSION(&info.version);
    if (!SDL_GetWindowWMInfo(win, &info)) return 0;
    switch (info.subsystem) {
#if defined(SDL_VIDEO_DRIVER_X11)
    case SDL_SYSWM_X11:
        *a = info.info.x11.display;
        *id = (uint64_t)info.info.x11.window;
        return 1;
#endif
#if defined(SDL_VIDEO_DRIVER_WAYLAND)
    case SDL_SYSWM_WAYLAND:
        *a = info.info.wl.display;
        *b = info.info.wl.surface;
        return 2;
#endif
#if defined(SDL_VIDEO_DRIVER_WINDOWS)
    case SDL_SYSWM_WINDOWS:
        *a = info.info.win.hinstance;
        *b = info.info.win.window;
        return 4;
#endif
    default:
        return 0;
    }
#endif
}

// For a window event: the SDL_WindowEventID, with the new size for a
// resize. 0 for any other event.
int32_t Kestrel_GetWindowEvent(SDL_Event* event, int64_t* w, int64_t* h) {
    if (event->type != SDL_WINDOWEVENT) return 0;
    *w = event->window.data1;
    *h = event->window.data2;
    return event->window.event;
}
