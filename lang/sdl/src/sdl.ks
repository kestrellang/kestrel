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

@extern(.C, mangleName: "SDL_Delay")
func sdlDelay(ms: UInt32)

@extern(.C, mangleName: "Kestrel_IsNull")
func isNull(ptr: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_DrawText")
func sdlDrawText(renderer: lang.ptr[lang.i8], text: CString, x: Int64, y: Int64, scale: Int64)

@extern(.C, mangleName: "Kestrel_GetTicks")
public func getTicks() -> UInt32

@extern(.C, mangleName: "Kestrel_MonotonicMs")
public func monotonicMs() -> Int64

// SDL_INIT_VIDEO
let sdlInitVideo: UInt32 = UInt32(intLiteral: 0x20);

// SDL event type constants
let sdlEventQuit: UInt32 = UInt32(intLiteral: 0x100);
let sdlEventKeyDown: UInt32 = UInt32(intLiteral: 0x300);
let sdlEventKeyUp: UInt32 = UInt32(intLiteral: 0x301);
let sdlEventMouseMotion: UInt32 = UInt32(intLiteral: 0x400);
let sdlEventMouseButtonDown: UInt32 = UInt32(intLiteral: 0x401);

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

public struct Milliseconds {
    public var value: UInt32

    public init(value: Int64) {
        self.value = UInt32(from: value);
    }
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

public enum Event {
    case Quit
    case KeyDown(Key)
    case KeyUp(Key)
    case MouseDown(Int64, Int64)
    case MouseMove(Int64, Int64)
}

public struct Renderer {
    var raw: lang.ptr[lang.i8]

    public func clear(color: Color) {
        sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
        sdlRenderClear(self.raw);
    }

    public func setColor(color: Color) {
        sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
    }

    public func fill(rect: Rectangle, color: Color) {
        sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
        sdlFillRect(self.raw, rect.x, rect.y, rect.width, rect.height);
    }

    // Use after a single `setColor` to avoid a per-rect syscall when
    // drawing many same-colored rects (e.g. a tile grid).
    public func fillRect(rect: Rectangle) {
        sdlFillRect(self.raw, rect.x, rect.y, rect.width, rect.height);
    }

    public func drawText(text: String, x: Int64, y: Int64, scale: Int64) {
        let cstr = text.toCString();
        sdlDrawText(self.raw, cstr, x, y, scale);
        cstr.free();
    }
}

public struct SDLApp : not Copyable {
    var window: lang.ptr[lang.i8]
    var rendererPtr: lang.ptr[lang.i8]
    var eventBuffer: Array[Int8]

    public init(title title: String, width width: Int64, height height: Int64) {
        if sdlInit(sdlInitVideo) < 0 {
            fatalError("SDL Init failed");
        }

        let titleCStr = title.toCString();
        let win = sdlCreateWindow(titleCStr, width, height);
        titleCStr.free();

        if isNull(win) != 0 {
            fatalError("Window creation failed");
        }

        let ren = sdlCreateRenderer(win);
        if isNull(ren) != 0 {
            fatalError("Renderer creation failed");
        }

        self.window = win;
        self.rendererPtr = ren;
        self.eventBuffer = Array[Int8](repeating: 0, count: 64);
    }

    deinit {
        sdlDestroyRenderer(self.rendererPtr);
        sdlDestroyWindow(self.window);
        sdlQuit();
    }

    public mutating func pollEvent() -> Event? {
        let eventPtr = lang.cast_ptr[_, lang.i8](self.eventBuffer.asPointer().asRaw().raw);

        guard sdlPollEvent(eventPtr) != 0 else {
            return null;
        }

        let eventType = sdlGetEventType(eventPtr);

        match eventType {
            sdlEventQuit => .Some(Event.Quit),
            sdlEventKeyDown => {
                let code = sdlGetKeyScancode(eventPtr);
                .Some(Event.KeyDown(scancodeToKey(code)))
            },
            sdlEventKeyUp => {
                let code = sdlGetKeyScancode(eventPtr);
                .Some(Event.KeyUp(scancodeToKey(code)))
            },
            sdlEventMouseMotion => {
                let mx = sdlGetMouseX(eventPtr);
                let my = sdlGetMouseY(eventPtr);
                .Some(Event.MouseMove(Int64(from: mx), Int64(from: my)))
            },
            sdlEventMouseButtonDown => {
                let mx = sdlGetMouseX(eventPtr);
                let my = sdlGetMouseY(eventPtr);
                .Some(Event.MouseDown(Int64(from: mx), Int64(from: my)))
            },
            _ => null
        }
    }

    public func render(body: (Renderer) -> ()) {
        let renderer = Renderer(raw: self.rendererPtr);
        body(renderer);
        sdlRenderPresent(self.rendererPtr);
    }

    public func delay(ms: Milliseconds) {
        sdlDelay(ms.value);
    }
}

let scancodeTable: Array[Key?] = buildScancodeTable();

func buildScancodeTable() -> Array[Key?] {
    var t = Array[Key?](repeating: null, count: 232);

    // Letters (scancodes 4–29)
    t(4) = .Some(Key.A);
    t(5) = .Some(Key.B);
    t(6) = .Some(Key.C);
    t(7) = .Some(Key.D);
    t(8) = .Some(Key.E);
    t(9) = .Some(Key.F);
    t(10) = .Some(Key.G);
    t(11) = .Some(Key.H);
    t(12) = .Some(Key.I);
    t(13) = .Some(Key.J);
    t(14) = .Some(Key.K);
    t(15) = .Some(Key.L);
    t(16) = .Some(Key.M);
    t(17) = .Some(Key.N);
    t(18) = .Some(Key.O);
    t(19) = .Some(Key.P);
    t(20) = .Some(Key.Q);
    t(21) = .Some(Key.R);
    t(22) = .Some(Key.S);
    t(23) = .Some(Key.T);
    t(24) = .Some(Key.U);
    t(25) = .Some(Key.V);
    t(26) = .Some(Key.W);
    t(27) = .Some(Key.X);
    t(28) = .Some(Key.Y);
    t(29) = .Some(Key.Z);

    // Digits (scancodes 30–39: 1-9 then 0)
    t(30) = .Some(Key.Digit1);
    t(31) = .Some(Key.Digit2);
    t(32) = .Some(Key.Digit3);
    t(33) = .Some(Key.Digit4);
    t(34) = .Some(Key.Digit5);
    t(35) = .Some(Key.Digit6);
    t(36) = .Some(Key.Digit7);
    t(37) = .Some(Key.Digit8);
    t(38) = .Some(Key.Digit9);
    t(39) = .Some(Key.Digit0);

    // Navigation & editing (scancodes 40–49)
    t(40) = .Some(Key.Return);
    t(41) = .Some(Key.Escape);
    t(42) = .Some(Key.Backspace);
    t(43) = .Some(Key.Tab);
    t(44) = .Some(Key.Space);
    t(45) = .Some(Key.Minus);
    t(46) = .Some(Key.Equals);
    t(47) = .Some(Key.LeftBracket);
    t(48) = .Some(Key.RightBracket);
    t(49) = .Some(Key.Backslash);

    // Punctuation (scancodes 51–57)
    t(51) = .Some(Key.Semicolon);
    t(52) = .Some(Key.Apostrophe);
    t(53) = .Some(Key.Grave);
    t(54) = .Some(Key.Comma);
    t(55) = .Some(Key.Period);
    t(56) = .Some(Key.Slash);
    t(57) = .Some(Key.CapsLock);

    // Function keys (scancodes 58–69)
    t(58) = .Some(Key.F1);
    t(59) = .Some(Key.F2);
    t(60) = .Some(Key.F3);
    t(61) = .Some(Key.F4);
    t(62) = .Some(Key.F5);
    t(63) = .Some(Key.F6);
    t(64) = .Some(Key.F7);
    t(65) = .Some(Key.F8);
    t(66) = .Some(Key.F9);
    t(67) = .Some(Key.F10);
    t(68) = .Some(Key.F11);
    t(69) = .Some(Key.F12);

    // Navigation block (scancodes 73–82)
    t(73) = .Some(Key.Insert);
    t(74) = .Some(Key.Home);
    t(75) = .Some(Key.PageUp);
    t(76) = .Some(Key.Delete);
    t(77) = .Some(Key.End);
    t(78) = .Some(Key.PageDown);
    t(79) = .Some(Key.RightArrow);
    t(80) = .Some(Key.LeftArrow);
    t(81) = .Some(Key.DownArrow);
    t(82) = .Some(Key.UpArrow);

    // Modifiers (scancodes 224–231)
    t(224) = .Some(Key.LeftCtrl);
    t(225) = .Some(Key.LeftShift);
    t(226) = .Some(Key.LeftAlt);
    t(227) = .Some(Key.LeftGui);
    t(228) = .Some(Key.RightCtrl);
    t(229) = .Some(Key.RightShift);
    t(230) = .Some(Key.RightAlt);
    t(231) = .Some(Key.RightGui);

    t
}

func scancodeToKey(code: Int32) -> Key {
    let idx = Int64(from: code);

    guard idx >= 0 and idx < 232 else {
        return .Other(code);
    }

    guard let some key = scancodeTable(idx) else {
        return .Other(code);
    }

    return key;
}
