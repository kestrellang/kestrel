module Sdl

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

@extern(.C, mangleName: "SDL_Delay")
func sdlDelay(ms: UInt32)

@extern(.C, mangleName: "Kestrel_IsNull")
func isNull(ptr: lang.ptr[lang.i8]) -> Int32

@extern(.C, mangleName: "Kestrel_DrawText")
func sdlDrawText(renderer: lang.ptr[lang.i8], text: CString, x: Int64, y: Int64, scale: Int64)

// --- High-Level Abstractions ---

public struct Color {
    var r: Int64
    var g: Int64
    var b: Int64
    var a: Int64

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
        self.value = UInt32(intLiteral: value.raw);
    }
}

public enum Key {
    case W
    case S
    case UpArrow
    case DownArrow
    case Space
    case Escape
    case Other(Int32)
}

public enum Event {
    case Quit
    case KeyDown(Key)
    case KeyUp(Key)
}

public struct Renderer {
    var raw: lang.ptr[lang.i8]

    public func clear(color: Color) {
        let _ = sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
        let _ = sdlRenderClear(self.raw);
    }

    public func setColor(color: Color) {
        let _ = sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
    }

    public func fill(rect: Rectangle, color: Color) {
        let _ = sdlSetRenderDrawColor(self.raw, color.r, color.g, color.b, color.a);
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

    public init() {
        if sdlInit(UInt32(intLiteral: 0x20)) < 0 {
            lang.panic("SDL Init failed");
        }

        let titleCStr = "Pong".toCString();
        let win = sdlCreateWindow(titleCStr, 800, 600);
        titleCStr.free();

        if isNull(win) != 0 {
            lang.panic("Window failed");
        }

        let ren = sdlCreateRenderer(win);
        if isNull(ren) != 0 {
            lang.panic("Renderer failed");
        }

        var buf = Array[Int8]();
        var j: Int64 = 0;
        while j < 64 {
            buf.append(Int8(intLiteral: 0));
            j = j + 1;
        }

        self.window = win;
        self.rendererPtr = ren;
        self.eventBuffer = buf;
    }

    deinit {
        sdlDestroyRenderer(self.rendererPtr);
        sdlDestroyWindow(self.window);
        sdlQuit();
    }

    public mutating func pollEvent() -> Event? {
        let eventPtr = lang.cast_ptr[lang.i8](self.eventBuffer.asPointer().asRaw().raw);

        if sdlPollEvent(eventPtr) == 0 {
            return null;
        }

        let eventType = sdlGetEventType(eventPtr);

        match eventType {
            0x100 => .Some(Event.Quit),
            0x300 => {
                let code = sdlGetKeyScancode(eventPtr);

                .Some(Event.KeyDown(scancodeToKey(code)))
            },
            0x301 => {
                let code = sdlGetKeyScancode(eventPtr);

                .Some(Event.KeyUp(scancodeToKey(code)))
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

func scancodeToKey(code: Int32) -> Key {
    if code == 26 {
        Key.W
    } else if code == 22 {
        Key.S
    } else if code == 82 {
        Key.UpArrow
    } else if code == 81 {
        Key.DownArrow
    } else if code == 44 {
        Key.Space
    } else if code == 41 {
        Key.Escape
    } else {
        Key.Other(code)
    }
}
