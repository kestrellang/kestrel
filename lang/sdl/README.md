# SDL

SDL2 bindings for Kestrel. A thin binding layer over SDL2 plus small high-level abstractions (window/renderer lifecycle, event polling, keyboard mapping) for writing games. Used by the `sdl_pong` and `life` examples.

## Installation

```toml
[dependencies]
sdl = { path = "../../lang/sdl" }
```

**Requires system SDL2.** The build shells out to `sdl2-config` for compile and link flags (see `[build] c-flags-cmd` / `link-cmd` in `flock.toml`), so `sdl2-config` must be on your `PATH`. On macOS: `brew install sdl2`. A bundled C helper (`sdl_helpers.c`) is compiled automatically.

## Key Types

- **SDLApp** (`not Copyable`) - owns the window and renderer; `deinit` tears down SDL
  - `init(title: String, width: Int64, height: Int64)`
  - `pollEvent() -> Event?` (mutating)
  - `render(body: (Renderer) -> ())` - draw inside the closure, then presents the frame
  - `delay(ms: Milliseconds)`
- **Renderer** - drawing surface passed to the `render` closure
  - `clear(color: Color)`, `setColor(color: Color)`
  - `fill(rect: Rectangle, color: Color)`, `fillRect(rect: Rectangle)` (uses current color)
  - `drawText(text: String, x: Int64, y: Int64, scale: Int64)` (built-in bitmap font)
- **Event** - `.Quit`, `.KeyDown(Key)`, `.KeyUp(Key)`, `.MouseDown(x, y)`, `.MouseMove(x, y)`
- **Key** - letters, digits, F1-F12, arrows, modifiers, navigation, punctuation, and `.Other(Int32)` for unmapped scancodes
- **Color** - `r/g/b/a` fields plus presets: `Color.black()`, `.white()`, `.red()`, `.green()`, `.blue()`, `.yellow()`, `.cyan()`, `.magenta()`
- **Rectangle** - `x`, `y`, `width`, `height`
- **Milliseconds** - `Milliseconds(16)` wrapper for `delay`
- Free functions: `getTicks() -> UInt32`, `monotonicMs() -> Int64`

## Usage

A minimal game loop (lifted from `examples/sdl_pong`):

```kestrel
import sdl.(Color, Rectangle, Milliseconds, Key, Event, Renderer, SDLApp)

@main
func main() -> lang.i32 {
    var app = SDLApp(title: "Pong", width: 800, height: 600);
    var running = true;

    while running {
        // Handle events
        while let .Some(event) = app.pollEvent() {
            match event {
                .Quit => { running = false },
                .KeyDown(key) => {
                    match key {
                        .Escape => { running = false },
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        // Draw a frame
        app.render { (renderer) in
            renderer.clear(Color.black());
            renderer.fill(Rectangle(x: 10, y: 250, width: 20, height: 100), Color.cyan());
            renderer.drawText("PRESS [SPACE] TO START", 180, 200, 3);
        };

        app.delay(Milliseconds(16));
    }

    0
}
```

Drawing many same-colored rects (e.g. a tile grid): call `setColor` once, then `fillRect` per tile to avoid a per-rect color syscall.
