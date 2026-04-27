#include <SDL.h>
#include <stdint.h>
#include <string.h>

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
