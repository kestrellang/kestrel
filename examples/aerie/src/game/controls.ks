module Aerie

import sdl.(Key)

/// Which movement keys are held.
struct Controls {
    var forward: Bool = false
    var back: Bool = false
    var left: Bool = false
    var right: Bool = false
    var up: Bool = false
    var down: Bool = false
    /// Breaking: the left mouse button, held.
    var breakHeld: Bool = false
    /// Presses waiting for the next frame: place, toggle flying, and a
    /// hotbar slot (or -1).
    var placePressed: Bool = false
    var flyPressed: Bool = false
    var slotPressed: Int64 = -1
    var fpsPressed: Bool = false
    /// Mouse movement since the last frame, in pixels, and wheel clicks.
    var mouseX: Float32 = 0.0
    var mouseY: Float32 = 0.0
    var wheel: Int64 = 0

    init() {}

    mutating func setKey(key: Key, held held: Bool) {
        match key {
            .W => { self.forward = held },
            .S => { self.back = held },
            .A => { self.left = held },
            .D => { self.right = held },
            .Space => { self.up = held },
            .LeftShift => { self.down = held },
            .F => { if held { self.flyPressed = true } },
            .Digit1 => { if held { self.slotPressed = 0 } },
            .Digit2 => { if held { self.slotPressed = 1 } },
            .Digit3 => { if held { self.slotPressed = 2 } },
            .Digit4 => { if held { self.slotPressed = 3 } },
            .Digit5 => { if held { self.slotPressed = 4 } },
            .Digit6 => { if held { self.slotPressed = 5 } },
            .Digit7 => { if held { self.slotPressed = 6 } },
            .Digit8 => { if held { self.slotPressed = 7 } },
            .Digit9 => { if held { self.slotPressed = 8 } },
            .Z => { if held { self.fpsPressed = true } },
            _ => {}
        }
    }

    /// Forget presses once a frame has handled them.
    mutating func clearPresses() {
        self.placePressed = false;
        self.flyPressed = false;
        self.slotPressed = -1;
        self.fpsPressed = false;
        self.mouseX = 0.0;
        self.mouseY = 0.0;
        self.wheel = 0;
    }
}

func axis(positive: Bool, negative: Bool) -> Float32 {
    (if positive { 1.0 } else { 0.0 }) - (if negative { 1.0 } else { 0.0 })
}

/// Radians of turn per pixel of mouse movement.
let mouseSensitivity: Float32 = 0.0025;
