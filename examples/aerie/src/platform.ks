module Aerie

// What differs between the desktop and the browser build. Everything else
// is shared: the game is driven one frame at a time either way.

import sdl.(NativeWindow)
import wgpu.(SurfaceSource)

/// The wgpu surface source for the game's window.
@platform(.Linux, .Darwin)
func surfaceSource(window: NativeWindow) -> SurfaceSource {
    match window {
        .Xlib(display, id) => .Xlib(display, id),
        .Wayland(display, surface) => .Wayland(display, surface),
        .MetalLayer(layer) => .MetalLayer(layer),
        .Windows(hinstance, hwnd) => .Windows(hinstance, hwnd),
        .Unsupported => fatalError("this platform's windows aren't supported")
    }
}

/// In a browser, SDL's window is the page's `<canvas id="canvas">`.
@platform(.Emscripten)
func surfaceSource(window: NativeWindow) -> SurfaceSource {
    .Canvas("#canvas")
}

/// Where saves go by default: the working folder on the desktop.
@platform(.Linux, .Darwin)
func defaultSavePath() -> String { "world.aerie" }

/// In a browser, `/saves` is IndexedDB-backed (mounted by `web/shell.html`).
@platform(.Emscripten)
func defaultSavePath() -> String { "/saves/world.aerie" }

/// Make a just-written save durable. Files are already on disk natively.
@platform(.Linux, .Darwin)
func persistSaves() {}

/// Flush the in-memory file system to IndexedDB.
@platform(.Emscripten)
func persistSaves() {
    let script = "FS.syncfs(false, (e) => { if (e) console.error('save failed', e); });".toCString();
    unsafe emscripten_run_script(script);
    script.free();
}

/// The end of a frame. The desktop loop runs on; a browser must get control
/// back to show the frame and deliver input (needs `-sASYNCIFY`).
@platform(.Linux, .Darwin)
func endFrame() {}

@platform(.Emscripten)
func endFrame() {
    unsafe emscripten_sleep(0);
}

@platform(.Emscripten)
@extern(.C, mangleName: "emscripten_sleep")
func emscripten_sleep(ms: UInt32)

@platform(.Emscripten)
@extern(.C, mangleName: "emscripten_run_script")
func emscripten_run_script(script: CString)
