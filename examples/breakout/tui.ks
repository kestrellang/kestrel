// TUI - Terminal User Interface utilities
// Provides styled text, cursor control, and box drawing

module Tui

// ============================================
// Style Options
// ============================================

public enum StyleOption {
    // Text styles
    case Bold
    case Dim
    case Italic
    case Underline

    // Foreground colors
    case Black
    case Red
    case Green
    case Yellow
    case Blue
    case Magenta
    case Cyan
    case White
    case Gray

    // Background colors
    case OnBlack
    case OnRed
    case OnGreen
    case OnYellow
    case OnBlue
    case OnMagenta
    case OnCyan
    case OnWhite

    public func toAnsi() -> String {
        match self {
            .Bold => "\x1b[1m",
            .Dim => "\x1b[2m",
            .Italic => "\x1b[3m",
            .Underline => "\x1b[4m",
            .Black => "\x1b[30m",
            .Red => "\x1b[31m",
            .Green => "\x1b[32m",
            .Yellow => "\x1b[33m",
            .Blue => "\x1b[34m",
            .Magenta => "\x1b[35m",
            .Cyan => "\x1b[36m",
            .White => "\x1b[37m",
            .Gray => "\x1b[90m",
            .OnBlack => "\x1b[40m",
            .OnRed => "\x1b[41m",
            .OnGreen => "\x1b[42m",
            .OnYellow => "\x1b[43m",
            .OnBlue => "\x1b[44m",
            .OnMagenta => "\x1b[45m",
            .OnCyan => "\x1b[46m",
            .OnWhite => "\x1b[47m",
        }
    }
}

// ============================================
// Style - callable styling container
// Usage: let style: Style = [.Green, .Bold]
//        print(style("Hello"))
// ============================================

public struct Style: ExpressibleByArrayLiteral, _ExpressibleByArrayLiteral {
    type Element = StyleOption

    private var options: Array[StyleOption]

    public init() {
        self.options = Array[StyleOption]();
    }

    // _ExpressibleByArrayLiteral (compiler calls this)
    public init(_arrayLiteralPointer: lang.ptr[StyleOption], _arrayLiteralCount: lang.i64) {
        self.init(arrayLiteral: LiteralSlice(pointer: _arrayLiteralPointer, count: _arrayLiteralCount));
    }

    // ExpressibleByArrayLiteral
    public init(arrayLiteral elements: LiteralSlice[StyleOption]) {
        self.options = Array[StyleOption]();
        var iter = elements.iter();
        var done = false;
        while done == false {
            let item = iter.next();
            match item {
                .Some(opt) => self.options.append(opt),
                .None => { done = true; }
            }
        }
    }

    // Build the ANSI prefix codes
    func codes() -> String {
        var result = "";
        var i: Int64 = 0;
        while i < self.options.count() {
            result = result + self.options.getUnchecked(i).toAnsi();
            i = i + 1;
        }
        result
    }

    // Callable subscript: style("text") -> String
    public subscript[F](value: F) -> String where F: Formattable {
        get {
            self.codes() + value.format() + "\x1b[0m"
        }
    }
}

// ============================================
// Cursor Control
// ============================================

// Move cursor to position (0-indexed)
public func moveTo(x x: Int64, y y: Int64) -> String {
    "\x1b[" + (y + 1).format() + ";" + (x + 1).format() + "H"
}

// Move cursor to home position (0,0)
public func home() -> String { "\x1b[H" }

// Hide cursor
public func hideCursor() -> String { "\x1b[?25l" }

// Show cursor
public func showCursor() -> String { "\x1b[?25h" }

// Clear entire screen
public func clearScreen() -> String { "\x1b[2J" }

// Clear from cursor to end of line
public func clearLine() -> String { "\x1b[K" }

// Reset all attributes
public func reset() -> String { "\x1b[0m" }

// ============================================
// Box Drawing
// ============================================

public struct Box {
    public var x: Int64
    public var y: Int64
    public var width: Int64
    public var height: Int64
    public var style: Style

    public init(x x: Int64, y y: Int64, width width: Int64, height height: Int64, style style: Style) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self.style = style;
    }

    // Interior dimensions (inside the border)
    public var innerWidth: Int64 { self.width - 2 }
    public var innerHeight: Int64 { self.height - 2 }

    // Position cursor inside the box (0,0 = top-left of interior)
    public func at(x x: Int64, y y: Int64) -> String {
        moveTo(x: self.x + 1 + x, y: self.y + 1 + y)
    }

    // Render a complete closed box
    public func render() {
        // Top edge
        print(moveTo(x: self.x, y: self.y) + self.style("╔"));
        var i: Int64 = 0;
        while i < self.width - 2 {
            print(self.style("═"));
            i = i + 1;
        }
        print(self.style("╗"));

        // Side edges
        var row: Int64 = 1;
        while row < self.height - 1 {
            print(moveTo(x: self.x, y: self.y + row) + self.style("║"));
            print(moveTo(x: self.x + self.width - 1, y: self.y + row) + self.style("║"));
            row = row + 1;
        }

        // Bottom edge
        print(moveTo(x: self.x, y: self.y + self.height - 1) + self.style("╚"));
        i = 0;
        while i < self.width - 2 {
            print(self.style("═"));
            i = i + 1;
        }
        print(self.style("╝"));
    }

    // Render box with open bottom (for breakout - ball falls through)
    public func renderOpen() {
        // Top edge
        print(moveTo(x: self.x, y: self.y) + self.style("╔"));
        var i: Int64 = 0;
        while i < self.width - 2 {
            print(self.style("═"));
            i = i + 1;
        }
        print(self.style("╗"));

        // Side edges only (no bottom)
        var row: Int64 = 1;
        while row < self.height {
            print(moveTo(x: self.x, y: self.y + row) + self.style("║"));
            print(moveTo(x: self.x + self.width - 1, y: self.y + row) + self.style("║"));
            row = row + 1;
        }
    }

    // Render just the top and sides for a specific number of rows
    public func renderPartial(rows rows: Int64) {
        // Top edge
        print(moveTo(x: self.x, y: self.y) + self.style("╔"));
        var i: Int64 = 0;
        while i < self.width - 2 {
            print(self.style("═"));
            i = i + 1;
        }
        print(self.style("╗"));

        // Side edges for specified rows
        var row: Int64 = 1;
        while row <= rows {
            print(moveTo(x: self.x, y: self.y + row) + self.style("║"));
            print(moveTo(x: self.x + self.width - 1, y: self.y + row) + self.style("║"));
            row = row + 1;
        }
    }
}

// ============================================
// Helper: repeat a string n times
// ============================================

public func repeatStr(s s: String, count count: Int64) -> String {
    var result = "";
    var i: Int64 = 0;
    while i < count {
        result = result + s;
        i = i + 1;
    }
    result
}
