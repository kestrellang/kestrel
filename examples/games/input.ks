// Input - Terminal input handling for games
// Provides type-safe key handling with RAII terminal management

module Input

// ============================================
// Key Enum - Type-safe key representation
// ============================================

public enum Key {
    case Left
    case Right
    case Up
    case Down
    case Space
    case Escape
    case Quit       // Q or q
    case Char(value: Int32)
}

// ============================================
// InputManager - RAII terminal input handler
// ============================================

public struct InputManager: not Copyable {
    var initialized: Bool

    public init() {
        initTerminal();
        self.initialized = true;
    }

    deinit {
        if self.initialized {
            restoreTerminal();
        }
    }

    // Poll for the next key (non-blocking)
    // Returns .None if no key is available
    public mutating func poll() -> Optional[Key] {
        let code = checkKey();
        if code == -1 {
            return .None
        }
        .Some(keyFromCode(code))
    }

    // Drain all pending keys into an array
    public mutating func drainAll() -> Array[Key] {
        var keys = Array[Key]();
        var code = checkKey();
        while code != -1 {
            keys.append(keyFromCode(code));
            code = checkKey();
        }
        keys
    }
}

// ============================================
// Key code conversion
// ============================================

func keyFromCode(code: Int32) -> Key {
    match code {
        1001 => .Up,
        1002 => .Down,
        1003 => .Right,
        1004 => .Left,
        32 => .Space,
        27 => .Escape,
        81 => .Quit,    // Q
        113 => .Quit,   // q
        97 => .Left,    // a
        100 => .Right,  // d
        65 => .Left,    // A
        68 => .Right,   // D
        119 => .Up,     // w
        115 => .Down,   // s
        87 => .Up,      // W
        83 => .Down,    // S
        _ => .Char(value: code),
    }
}

// ============================================
// External C functions
// ============================================

@extern(.C, mangleName: "game_init_terminal")
func initTerminal() -> Int32

@extern(.C, mangleName: "game_restore_terminal")
func restoreTerminal() -> Int32

@extern(.C, mangleName: "game_check_key")
func checkKey() -> Int32
