module SdlPong

// --- SDL2 Bindings (Using Int64 for everything to ensure alignment) ---

@extern(.C, mangleName: "SDL_Init")
func sdlInit(flags: UInt32) -> Int32

@extern(.C, mangleName: "SDL_Quit")
func sdlQuit()

@extern(.C, mangleName: "Kestrel_CreateWindow")
func sdlCreateWindow(title: lang.ptr[lang.i8], w: Int64, h: Int64) -> lang.ptr[lang.i8]

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
func sdlDrawText(renderer: lang.ptr[lang.i8], text: lang.ptr[lang.i8], x: Int64, y: Int64, scale: Int64)

// --- Game ---

struct SDLApp : not Copyable {
    var window: lang.ptr[lang.i8]
    var renderer: lang.ptr[lang.i8]

    static func create() -> SDLApp {
        if sdlInit(UInt32(intLiteral: 0x20)) < Int32(intLiteral: 0) {
            lang.panic("SDL Init failed");
        }

        var title = Array[Int8]();
        title.append(Int8(intLiteral: 80)); // P
        title.append(Int8(intLiteral: 111)); // o
        title.append(Int8(intLiteral: 110)); // n
        title.append(Int8(intLiteral: 103)); // g
        title.append(Int8(intLiteral: 0));

        let win = sdlCreateWindow(lang.cast_ptr[lang.i8](title.pointer().asRaw().raw), 800, 600);
        if isNull(win) != Int32(intLiteral: 0) {
            lang.panic("Window failed");
        }

        let ren = sdlCreateRenderer(win);
        if isNull(ren) != Int32(intLiteral: 0) {
            lang.panic("Renderer failed");
        }

        SDLApp(window: win, renderer: ren)
    }

    deinit {
        sdlDestroyRenderer(self.renderer);
        sdlDestroyWindow(self.window);
        sdlQuit();
    }
}

struct State {
    var ballX: Float64
    var ballY: Float64
    var ballVX: Float64
    var ballVY: Float64
    var p1Y: Float64
    var p2Y: Float64
    var p1Up: Bool
    var p1Down: Bool
    var score1: Int64
    var score2: Int64
    var waiting: Bool

    init() {
        self.ballX = 400.0;
        self.ballY = 300.0;
        self.ballVX = 12.0;
        self.ballVY = 8.0;
        self.p1Y = 250.0;
        self.p2Y = 250.0;
        self.p1Up = false;
        self.p1Down = false;
        self.score1 = 0;
        self.score2 = 0;
        self.waiting = true;
    }

    mutating func resetBall(winner winner: Int64) {
        self.ballX = 400.0;
        self.ballY = 300.0;
        if winner == 1 {
            self.ballVX = 0.0 - 12.0;
        } else {
            self.ballVX = 12.0;
        }
        self.ballVY = 8.0;
        self.waiting = true;
    }
}

func main() -> Int32 {
    var app = SDLApp.create();
    var s = State();
    var running = true;

    var eventBuf = Array[Int8]();
    var j: Int64 = 0;
    while j < 64 {
        eventBuf.append(Int8(intLiteral: 0));
        j = j + 1
    }
    let eventPtr = lang.cast_ptr[lang.i8](eventBuf.pointer().asRaw().raw);

    let zero = Int32(intLiteral: 0);
    let zero64 = Int64(intLiteral: 0);
    let white64 = Int64(intLiteral: 255);
    
    let QUIT = UInt32(intLiteral: 0x100);
    let KEYDOWN = UInt32(intLiteral: 0x300);
    let KEYUP = UInt32(intLiteral: 0x301);

    var startMsg = Array[Int8]();
    let msg = "PRESS [SPACE] TO START\0";
    var mi: Int64 = 0;
    // msg.byteCount() or similar? String doesn't have many public methods.
    // Let's just hardcode the append for the message.
    startMsg.append(Int8(intLiteral: 80)); // P
    startMsg.append(Int8(intLiteral: 82)); // R
    startMsg.append(Int8(intLiteral: 69)); // E
    startMsg.append(Int8(intLiteral: 83)); // S
    startMsg.append(Int8(intLiteral: 83)); // S
    startMsg.append(Int8(intLiteral: 32)); //  
    startMsg.append(Int8(intLiteral: 91)); // [
    startMsg.append(Int8(intLiteral: 83)); // S
    startMsg.append(Int8(intLiteral: 80)); // P
    startMsg.append(Int8(intLiteral: 65)); // A
    startMsg.append(Int8(intLiteral: 67)); // C
    startMsg.append(Int8(intLiteral: 69)); // E
    startMsg.append(Int8(intLiteral: 93)); // ]
    startMsg.append(Int8(intLiteral: 32)); //  
    startMsg.append(Int8(intLiteral: 84)); // T
    startMsg.append(Int8(intLiteral: 79)); // O
    startMsg.append(Int8(intLiteral: 32)); //  
    startMsg.append(Int8(intLiteral: 83)); // S
    startMsg.append(Int8(intLiteral: 84)); // T
    startMsg.append(Int8(intLiteral: 65)); // A
    startMsg.append(Int8(intLiteral: 82)); // R
    startMsg.append(Int8(intLiteral: 84)); // T
    startMsg.append(Int8(intLiteral: 0));
    let startMsgPtr = lang.cast_ptr[lang.i8](startMsg.pointer().asRaw().raw);

    while running {
        while sdlPollEvent(eventPtr) != zero {
            let eventType = sdlGetEventType(eventPtr);
            if eventType == QUIT {
                running = false;
            } else if eventType == KEYDOWN {
                let code = sdlGetKeyScancode(eventPtr);
                if code == Int32(intLiteral: 26) or code == Int32(intLiteral: 82) { s.p1Up = true; }
                else if code == Int32(intLiteral: 22) or code == Int32(intLiteral: 81) { s.p1Down = true; }
                else if code == Int32(intLiteral: 44) { s.waiting = false; }
                else if code == Int32(intLiteral: 41) { running = false; }
            } else if eventType == KEYUP {
                let code = sdlGetKeyScancode(eventPtr);
                if code == Int32(intLiteral: 26) or code == Int32(intLiteral: 82) { s.p1Up = false; }
                else if code == Int32(intLiteral: 22) or code == Int32(intLiteral: 81) { s.p1Down = false; }
            }
        }

        if s.p1Up { s.p1Y = s.p1Y - 10.0; }
        if s.p1Down { s.p1Y = s.p1Y + 10.0; }
        if s.p1Y < 0.0 { s.p1Y = 0.0; } else if s.p1Y > 500.0 { s.p1Y = 500.0; }

        if s.waiting == false {
            // Paddle 2: Simple AI (Fast speed and reacts much earlier)
            if s.ballX > 200.0 {
                if s.ballY > s.p2Y + 50.0 { s.p2Y = s.p2Y + 7.2; } else { s.p2Y = s.p2Y - 7.2; }
            }
            if s.p2Y < 0.0 { s.p2Y = 0.0; } else if s.p2Y > 500.0 { s.p2Y = 500.0; }

            s.ballX = s.ballX + s.ballVX;
            s.ballY = s.ballY + s.ballVY;

            if s.ballY < 0.0 { s.ballY = 0.0; s.ballVY = 0.0 - s.ballVY; }
            else if s.ballY > 590.0 { s.ballY = 590.0; s.ballVY = 0.0 - s.ballVY; }
            
            if s.ballX < 0.0 {
                s.score2 = s.score2 + 1;
                s.resetBall(winner: 2);
            }
            else if s.ballX > 800.0 {
                s.score1 = s.score1 + 1;
                s.resetBall(winner: 1);
            }

            if s.ballX < 30.0 and s.ballY >= s.p1Y and s.ballY <= s.p1Y + 100.0 { s.ballVX = 12.0; }
            if s.ballX > 760.0 and s.ballY >= s.p2Y and s.ballY <= s.p2Y + 100.0 { s.ballVX = 0.0 - 12.0; }
        }

        let _ = sdlSetRenderDrawColor(app.renderer, zero64, zero64, zero64, white64);
        let _ = sdlRenderClear(app.renderer);
        let _ = sdlSetRenderDrawColor(app.renderer, white64, white64, white64, white64);

        sdlFillRect(app.renderer, 10, s.p1Y.toInt64().unwrap(), 20, 100);
        sdlFillRect(app.renderer, 770, s.p2Y.toInt64().unwrap(), 20, 100);
        
        // Draw score text
        var scoreStr = Array[Int8]();
        // Format: "00 - 00"
        let s1 = s.score1;
        let s2 = s.score2;
        let ten = Int64(intLiteral: 10);
        let fortyEight = Int64(intLiteral: 48);
        scoreStr.append(Int8(intLiteral: ((s1 / ten % ten) + fortyEight).raw));
        scoreStr.append(Int8(intLiteral: ((s1 % ten) + fortyEight).raw));
        scoreStr.append(Int8(intLiteral: 32)); //  
        scoreStr.append(Int8(intLiteral: 45)); // -
        scoreStr.append(Int8(intLiteral: 32)); //  
        scoreStr.append(Int8(intLiteral: ((s2 / ten % ten) + fortyEight).raw));
        scoreStr.append(Int8(intLiteral: ((s2 % ten) + fortyEight).raw));
        scoreStr.append(Int8(intLiteral: 0));
        sdlDrawText(app.renderer, lang.cast_ptr[lang.i8](scoreStr.pointer().asRaw().raw), 350, 20, 4);

        if s.waiting {
            sdlDrawText(app.renderer, startMsgPtr, 180, 200, 3);
        }

        sdlFillRect(app.renderer, s.ballX.toInt64().unwrap(), s.ballY.toInt64().unwrap(), 10, 10);

        sdlRenderPresent(app.renderer);
        sdlDelay(UInt32(intLiteral: 16));
    }

    Int32(intLiteral: 0)
}
