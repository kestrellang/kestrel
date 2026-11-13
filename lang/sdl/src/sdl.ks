module sdl

// --- SDL2 External Bindings ---

@extern(.C, mangleName: "SDL_Init")
func sdlInit(flags: UInt32) -> Int32

@extern(.C, mangleName: "SDL_Quit")
func sdlQuit()

@extern(.C, mangleName: "Kestrel_CreateWindow")
func sdlCreateWindow(title: CString, w: Int64, h: Int64) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "SDL_DestroyWindow")
func sdlDestroyWindow(window: lang.ptr[lang.i8])

@extern(.C, mangleName: "Kestrel_CreateRenderer")
func sdlCreateRenderer(window: lang.ptr[lang.i8]) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "SDL_DestroyRenderer")
func sdlDestroyRenderer(renderer: lang.ptr[lang.i8])

@extern(.C, mangleName: "Kestrel_SetRenderDrawColor")
func sdlSetRenderDrawColor(renderer: lang.ptr[lang.i8], r: Int64, g: Int64, b: Int64, a: Int64) -> Int64

@extern(.C, mangleName: "SDL_RenderClear")
func sdlRenderClear(renderer: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "SDL_RenderPresent")
func sdlRenderPresent(renderer: lang.ptr[lang.i8])

@extern(.C, mangleName: "Kestrel_FillRect")
func sdlFillRect(renderer: lang.ptr[lang.i8], x: Int64, y: Int64, w: Int64, h: Int64)

@extern(.C, mangleName: "SDL_PollEvent")
func sdlPollEvent(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetEventType")
func sdlGetEventType(event: lang.ptr[lang.i8]) -> UInt32

@extern(.C, mangleName: "Kestrel_GetKeyScancode")
func sdlGetKeyScancode(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetMouseX")
func sdlGetMouseX(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetMouseY")
func sdlGetMouseY(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetMouseButton")
func sdlGetMouseButton(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetMouseRelX")
func sdlGetMouseRelX(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetMouseRelY")
func sdlGetMouseRelY(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_GetWheelY")
func sdlGetWheelY(event: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "SDL_SetRelativeMouseMode")
func sdlSetRelativeMouseMode(enabled: Int32) -> Int32

@extern(.C, mangleName: "SDL_Delay")
func sdlDelay(ms: UInt32)

@extern(.C, mangleName: "Kestrel_IsNull")
func isNull(ptr: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_DrawText")
func sdlDrawText(renderer: lang.ptr[lang.i8], text: CString, x: Int64, y: Int64, scale: Int64)

@extern(.C, mangleName: "Kestrel_CreateGPUWindow")
func sdlCreateGPUWindow(title: CString, w: Int64, h: Int64) -> lang.ptr[lang.i8]

@extern(.C, mangleName: "Kestrel_GetDrawableSize")
func sdlGetDrawableSize(window: lang.ptr[lang.i8], w: &mutating Int64, h: &mutating Int64)

@extern(.C, mangleName: "Kestrel_GetNativeWindow")
func sdlGetNativeWindow(window: lang.ptr[lang.i8], a: &mutating RawPointer, b: &mutating RawPointer, id: &mutating UInt64) -> Int32

@extern(.C, mangleName: "Kestrel_GetWindowEvent")
func sdlGetWindowEvent(event: lang.ptr[lang.i8], w: &mutating Int64, h: &mutating Int64) -> Int32

@extern(.C, mangleName: "Kestrel_GetTicks")
public func getTicks() -> UInt32

@extern(.C, mangleName: "Kestrel_MonotonicMs")
public func monotonicMs() -> Int64

/// The monotonic clock in microseconds; needs no `SDL_Init`.
@extern(.C, mangleName: "Kestrel_MonotonicUs")
public func monotonicUs() -> Int64

// SDL_INIT_VIDEO
let sdlInitVideo: UInt32 = 0x20;

// SDL event type constants
let sdlEventQuit: UInt32 = 0x100;
let sdlEventWindow: UInt32 = 0x200;
// SDL_WINDOWEVENT_SIZE_CHANGED
let sdlWindowEventSizeChanged: Int32 = 6;
let sdlEventKeyDown: UInt32 = 0x300;
let sdlEventKeyUp: UInt32 = 0x301;
let sdlEventMouseMotion: UInt32 = 0x400;
let sdlEventMouseButtonDown: UInt32 = 0x401;
let sdlEventMouseButtonUp: UInt32 = 0x402;
let sdlEventMouseWheel: UInt32 = 0x403;

// --- High-Level Abstractions ---

public struct Color {
    public var r: Int64
    public var g: Int64
    public var b: Int64
    public var a: Int64

    public static func black() -> Color {
        Color(r: 0, g: 0, b: 0, a: 255)
    }

    public static func white() -> Color {
        Color(r: 255, g: 255, b: 255, a: 255)
    }

    public static func red() -> Color {
        Color(r: 255, g: 80, b: 80, a: 255)
    }

    public static func green() -> Color {
        Color(r: 80, g: 255, b: 120, a: 255)
    }

    public static func blue() -> Color {
        Color(r: 80, g: 160, b: 255, a: 255)
    }

    public static func yellow() -> Color {
        Color(r: 255, g: 220, b: 80, a: 255)
    }

    public static func cyan() -> Color {
        Color(r: 80, g: 255, b: 255, a: 255)
    }

    public static func magenta() -> Color {
        Color(r: 255, g: 80, b: 220, a: 255)
    }
}

public struct Rectangle {
    public var x: Int64
    public var y: Int64
    public var width: Int64
    public var height: Int64
}

public enum Key {
    // Letters
    case A
    case B
    case C
    case D
    case E
    case F
    case G
    case H
    case I
    case J
    case K
    case L
    case M
    case N
    case O
    case P
    case Q
    case R
    case S
    case T
    case U
    case V
    case W
    case X
    case Y
    case Z

    // Digits
    case Digit0
    case Digit1
    case Digit2
    case Digit3
    case Digit4
    case Digit5
    case Digit6
    case Digit7
    case Digit8
    case Digit9

    // Function keys
    case F1
    case F2
    case F3
    case F4
    case F5
    case F6
    case F7
    case F8
    case F9
    case F10
    case F11
    case F12

    // Arrow keys
    case UpArrow
    case DownArrow
    case LeftArrow
    case RightArrow

    // Modifiers
    case LeftShift
    case RightShift
    case LeftCtrl
    case RightCtrl
    case LeftAlt
    case RightAlt
    case LeftGui
    case RightGui
    case CapsLock

    // Navigation
    case Return
    case Escape
    case Backspace
    case Tab
    case Space
    case Insert
    case Delete
    case Home
    case End
    case PageUp
    case PageDown

    // Punctuation
    case Minus
    case Equals
    case LeftBracket
    case RightBracket
    case Backslash
    case Semicolon
    case Apostrophe
    case Grave
    case Comma
    case Period
    case Slash

    case Other(Int32)
}

/// A mouse button; `.Other` holds SDL's number for extra buttons.
public enum MouseButton {
    case Left
    case Middle
    case Right
    case Other(Int64)
}

/// The button for SDL button number `code`.
func mouseButton(code: Int32) -> MouseButton {
    match Int64(from: code) {
        1 => .Left,
        2 => .Middle,
        3 => .Right,
        n => .Other(n)
    }
}

public enum Event {
    case Quit
    case KeyDown(Key)
    case KeyUp(Key)
    /// A button went down or up at (x, y) in the window.
    case MouseDown(MouseButton, Int64, Int64)
    case MouseUp(MouseButton, Int64, Int64)
    /// The mouse moved to (x, y), by (dx, dy). With relative mouse mode on,
    /// only the movement is meaningful.
    case MouseMove(Int64, Int64, Int64, Int64)
    /// The wheel turned this many clicks, positive away from the user.
    case MouseWheel(Int64)
    /// The window was resized, to this size in screen units (use
    /// `SDLWindow.drawableSize` for pixels).
    case Resized(Int64, Int64)
}

public struct Renderer {
    var raw: lang.ptr[lang.i8]

    public func clear(color: Color) {
        unsafe sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
        unsafe sdlRenderClear(self.raw);
    }

    public func setColor(color: Color) {
        unsafe sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
    }

    public func fill(rect: Rectangle, color: Color) {
        unsafe sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
        unsafe sdlFillRect(self.raw, rect.x, rect.y, rect.width, rect.height);
    }

    // Use after a single `setColor` to avoid a per-rect syscall when
    // drawing many same-colored rects (e.g. a tile grid).
    public func fillRect(rect: Rectangle) {
        unsafe sdlFillRect(self.raw, rect.x, rect.y, rect.width, rect.height);
    }

    public func drawText(text: String, x: Int64, y: Int64, scale: Int64) {
        let cstr = text.toCString();
        unsafe sdlDrawText(self.raw, cstr, x, y, scale);
        cstr.free();
    }
}

public struct SDLApp : not Copyable {
    var window: lang.ptr[lang.i8]
    var rendererPtr: lang.ptr[lang.i8]
    var eventBuffer: Array[Int8]

    public init(title title: String, width width: Int64, height height: Int64) {
        if unsafe sdlInit(sdlInitVideo) < 0 {
            fatalError("SDL Init failed");
        }

        let titleCStr = title.toCString();
        let win = unsafe sdlCreateWindow(titleCStr, width, height);
        titleCStr.free();

        if unsafe isNull(win) != 0 {
            fatalError("Window creation failed");
        }

        let ren = unsafe sdlCreateRenderer(win);
        if unsafe isNull(ren) != 0 {
            fatalError("Renderer creation failed");
        }

        self.window = win;
        self.rendererPtr = ren;
        self.eventBuffer = Array[Int8](repeating: 0, count: 64);
    }

    deinit {
        unsafe sdlDestroyRenderer(self.rendererPtr);
        unsafe sdlDestroyWindow(self.window);
        unsafe sdlQuit();
    }

    public mutating func pollEvent() -> Event? {
        nextEvent(self.eventBuffer)
    }

    public func render(body: (Renderer) -> ()) {
        let renderer = Renderer(raw: self.rendererPtr);
        body(renderer);
        unsafe sdlRenderPresent(self.rendererPtr);
    }

    /// Pauses for `duration` (whole milliseconds), as in
    /// `app.delay(.milliseconds(16))`.
    public func delay(duration: Duration) {
        let ms = if duration.isNegative { 0 } else { duration.totalMilliseconds };
        unsafe sdlDelay(UInt32(truncating: ms));
    }
}

/// One SDL event from `eventBuffer`, or `None` when the queue is empty.
func nextEvent(mutating eventBuffer: Array[Int8]) -> Event? {
    let eventPtr = lang.cast_ptr[_, lang.i8](eventBuffer.asPointer().asRaw().raw);

    guard unsafe sdlPollEvent(eventPtr) != 0 else {
        return null;
    }

    let eventType = unsafe sdlGetEventType(eventPtr);

    // `if` rather than `match`: a bare name in a pattern binds, it doesn't
    // compare against the constant.
    if eventType == sdlEventQuit {
        return .Some(Event.Quit);
    }
    if eventType == sdlEventKeyDown {
        return .Some(Event.KeyDown(scancodeToKey(unsafe sdlGetKeyScancode(eventPtr))));
    }
    if eventType == sdlEventKeyUp {
        return .Some(Event.KeyUp(scancodeToKey(unsafe sdlGetKeyScancode(eventPtr))));
    }
    if eventType == sdlEventMouseMotion or eventType == sdlEventMouseButtonDown or eventType == sdlEventMouseButtonUp {
        let mx = Int64(from: unsafe sdlGetMouseX(eventPtr));
        let my = Int64(from: unsafe sdlGetMouseY(eventPtr));
        if eventType == sdlEventMouseMotion {
            let dx = Int64(from: unsafe sdlGetMouseRelX(eventPtr));
            let dy = Int64(from: unsafe sdlGetMouseRelY(eventPtr));
            return .Some(Event.MouseMove(mx, my, dx, dy));
        }
        let button = mouseButton(unsafe sdlGetMouseButton(eventPtr));
        if eventType == sdlEventMouseButtonDown {
            return .Some(Event.MouseDown(button, mx, my));
        }
        return .Some(Event.MouseUp(button, mx, my));
    }
    if eventType == sdlEventMouseWheel {
        return .Some(Event.MouseWheel(Int64(from: unsafe sdlGetWheelY(eventPtr))));
    }
    if eventType == sdlEventWindow {
        var w: Int64 = 0;
        var h: Int64 = 0;
        if unsafe sdlGetWindowEvent(eventPtr, w, h) == sdlWindowEventSizeChanged {
            return .Some(Event.Resized(w, h));
        }
    }
    null
}

/// The platform handles behind a window, for a GPU API such as `wgpu` to
/// create a surface from.
public enum NativeWindow {
    /// An X11 `Display*` and `Window` id.
    case Xlib(RawPointer, UInt64)
    /// A Wayland `wl_display*` and `wl_surface*`.
    case Wayland(RawPointer, RawPointer)
    /// A macOS `CAMetalLayer*`.
    case MetalLayer(RawPointer)
    /// A Windows `HINSTANCE` and `HWND`.
    case Windows(RawPointer, RawPointer)
    /// A platform SDL can't describe.
    case Unsupported
}

/// A resizable window with no SDL renderer, for a GPU API to draw into.
public struct SDLWindow: not Copyable {
    var window: lang.ptr[lang.i8]
    var eventBuffer: Array[Int8]

    public init(title title: String, width width: Int64, height height: Int64) {
        if unsafe sdlInit(sdlInitVideo) < 0 {
            fatalError("SDL Init failed");
        }
        let titleCStr = title.toCString();
        let win = unsafe sdlCreateGPUWindow(titleCStr, width, height);
        titleCStr.free();
        if unsafe isNull(win) != 0 {
            fatalError("Window creation failed");
        }
        self.window = win;
        self.eventBuffer = Array[Int8](repeating: 0, count: 64);
    }

    deinit {
        unsafe sdlDestroyWindow(self.window);
        unsafe sdlQuit();
    }

    /// The drawable area in pixels: what a GPU surface should be sized to.
    public var drawableSize: (Int64, Int64) {
        var w: Int64 = 0;
        var h: Int64 = 0;
        unsafe sdlGetDrawableSize(self.window, w, h);
        (w, h)
    }

    /// The platform handles behind the window. On macOS the first call
    /// attaches a Metal layer to the window.
    public var nativeWindow: NativeWindow {
        var a = RawPointer.nullPointer();
        var b = RawPointer.nullPointer();
        var id: UInt64 = 0;
        match unsafe sdlGetNativeWindow(self.window, a, b, id) {
            1 => .Xlib(a, id),
            2 => .Wayland(a, b),
            3 => .MetalLayer(a),
            4 => .Windows(a, b),
            _ => .Unsupported
        }
    }

    /// The next pending event, or `None`.
    public mutating func pollEvent() -> Event? {
        nextEvent(self.eventBuffer)
    }

    /// Capture the mouse: hide the cursor, keep it in the window, and
    /// report only movement, for mouse look. In a browser this asks for
    /// pointer lock, which the browser grants only after a click.
    public func setRelativeMouseMode(enabled: Bool) {
        unsafe sdlSetRelativeMouseMode(if enabled { 1 } else { 0 });
    }

    /// Pauses for `duration` (whole milliseconds), as in
    /// `app.delay(.milliseconds(16))`.
    public func delay(duration: Duration) {
        let ms = if duration.isNegative { 0 } else { duration.totalMilliseconds };
        unsafe sdlDelay(UInt32(truncating: ms));
    }
}

/// The key for SDL scancode `code`, or `.Other(code)` for one with no name.
func scancodeToKey(code: Int32) -> Key {
    match code {
        4 => .A,
        5 => .B,
        6 => .C,
        7 => .D,
        8 => .E,
        9 => .F,
        10 => .G,
        11 => .H,
        12 => .I,
        13 => .J,
        14 => .K,
        15 => .L,
        16 => .M,
        17 => .N,
        18 => .O,
        19 => .P,
        20 => .Q,
        21 => .R,
        22 => .S,
        23 => .T,
        24 => .U,
        25 => .V,
        26 => .W,
        27 => .X,
        28 => .Y,
        29 => .Z,
        30 => .Digit1,
        31 => .Digit2,
        32 => .Digit3,
        33 => .Digit4,
        34 => .Digit5,
        35 => .Digit6,
        36 => .Digit7,
        37 => .Digit8,
        38 => .Digit9,
        39 => .Digit0,
        40 => .Return,
        41 => .Escape,
        42 => .Backspace,
        43 => .Tab,
        44 => .Space,
        45 => .Minus,
        46 => .Equals,
        47 => .LeftBracket,
        48 => .RightBracket,
        49 => .Backslash,
        51 => .Semicolon,
        52 => .Apostrophe,
        53 => .Grave,
        54 => .Comma,
        55 => .Period,
        56 => .Slash,
        57 => .CapsLock,
        58 => .F1,
        59 => .F2,
        60 => .F3,
        61 => .F4,
        62 => .F5,
        63 => .F6,
        64 => .F7,
        65 => .F8,
        66 => .F9,
        67 => .F10,
        68 => .F11,
        69 => .F12,
        73 => .Insert,
        74 => .Home,
        75 => .PageUp,
        76 => .Delete,
        77 => .End,
        78 => .PageDown,
        79 => .RightArrow,
        80 => .LeftArrow,
        81 => .DownArrow,
        82 => .UpArrow,
        224 => .LeftCtrl,
        225 => .LeftShift,
        226 => .LeftAlt,
        227 => .LeftGui,
        228 => .RightCtrl,
        229 => .RightShift,
        230 => .RightAlt,
        231 => .RightGui,
        _ => .Other(code)
    }
}
