// A terminal-based Snake game in Kestrel using shared game libraries.
// Demonstrates: Structs, Methods, Arrays, TUI library, and Input handling.

module Snake

import std.core.Range
import Tui.(Style, StyleOption, Box, moveTo, home, clearScreen, hideCursor, showCursor, clearLine, repeatStr)
import Input.(Key, InputManager)

// Workaround for cross-module ExpressibleByArrayLiteral compiler bug:
// Array literal syntax for Style doesn't resolve element types across modules.
func makeStyle(options: Array[StyleOption]) -> Style {
    let ptr = lang.cast_ptr[_, StyleOption](options.asPointer().asRaw().raw);
    Style(_arrayLiteralPointer: ptr, _arrayLiteralCount: options.count.raw)
}

// ============================================
// Configuration
// ============================================

struct Config {
    static var gameWidth: Int64 { 40 }
    static var gameHeight: Int64 { 20 }
}

// ============================================
// Styles
// ============================================

struct Styles : Cloneable {
    static var border: Style { makeStyle([.White]) }
    static var title: Style { makeStyle([.Yellow, .Bold]) }
    static var score: Style { makeStyle([.White, .Bold]) }
    static var label: Style { makeStyle([.Gray]) }
    static var head: Style { makeStyle([.Green, .Bold]) }
    static var body: Style { makeStyle([.Green]) }
    static var food: Style { makeStyle([.Red, .Bold]) }
    static var gameOver: Style { makeStyle([.Red, .Bold]) }
    static var prompt: Style { makeStyle([.Yellow]) }

    func clone() -> Styles { Styles() }
}

// ============================================
// Point
// ============================================

struct Point : Cloneable {
    var x: Int64
    var y: Int64

    func clone() -> Point {
        Point(x: self.x, y: self.y)
    }
}

// ============================================
// Game State
// ============================================

enum GameState {
    case Playing
    case GameOver
}

enum GameAction {
    case Continue
    case Restart
    case Quit
}

// ============================================
// Snake Game
// ============================================

struct SnakeGame: not Copyable {
    // Snake position and movement
    var headX: Int64
    var headY: Int64
    var direction: Int64      // 0=up, 1=right, 2=down, 3=left
    var nextDirection: Int64  // Buffered direction to prevent 180-degree turns

    // Body segments
    var body: Array[Point]

    // Food position
    var foodX: Int64
    var foodY: Int64

    // Game state
    var state: GameState
    var score: Int64
    var box: Box

    // Simple random seed
    var seed: Int64

    var input: InputManager

    init() {
        // Start snake in center, moving right
        self.headX = Config.gameWidth / 2;
        self.headY = Config.gameHeight / 2;
        self.direction = 1;  // Right
        self.nextDirection = 1;

        // Initialize body with 3 segments behind the head
        self.body = Array[Point]();
        self.body.append(Point(x: self.headX - 1, y: self.headY));
        self.body.append(Point(x: self.headX - 2, y: self.headY));
        self.body.append(Point(x: self.headX - 3, y: self.headY));

        self.state = .Playing;
        self.score = 0;
        self.box = Box(x: 0, y: 1, width: Config.gameWidth + 2, height: Config.gameHeight + 2, style: Styles.border);

        // Initialize random seed
        self.seed = 12345;

        // Spawn initial food
        self.foodX = 0;
        self.foodY = 0;

        self.input = InputManager();

        print(hideCursor() + clearScreen());

        // Spawn food after init
        self.spawnFood();
    }

    deinit {
        print(showCursor() + clearScreen() + home());
        println("Thanks for playing Snake!");
    }

    // ----------------------------------------
    // Random & Food
    // ----------------------------------------

    // Simple LCG pseudo-random number generator
    mutating func randomNext() -> Int64 {
        self.seed = (self.seed * 1103515245 + 12345) % 2147483648;
        if self.seed < 0 {
            self.seed = 0 - self.seed;
        }
        self.seed
    }

    // Spawn food at a random position not occupied by snake
    mutating func spawnFood() {
        var attempts = 0;
        while attempts < 1000 {
            let rx = self.randomNext() % Config.gameWidth;
            let ry = self.randomNext() % Config.gameHeight;

            // Check if position is clear (not head or body)
            if rx != self.headX or ry != self.headY {
                if not self.isBody(x: rx, y: ry) {
                    self.foodX = rx;
                    self.foodY = ry;
                    return;
                }
            }
            attempts = attempts + 1;
        }
        // Fallback: place at (0,0) if we can't find a spot
        self.foodX = 0;
        self.foodY = 0;
    }

    // Check if a position is part of the snake body
    func isBody(x x: Int64, y y: Int64) -> Bool {
        for p in self.body {
            if p.x == x and p.y == y {
                return true;
            }
        }
        false
    }

    // ----------------------------------------
    // Input Handling
    // ----------------------------------------

    mutating func handlePlayingInput() {
        for key in self.input.drainAll() {
            match key {
                .Up => {
                    // Can't reverse into self (can't go up if going down)
                    if self.direction != 2 {
                        self.nextDirection = 0;
                    }
                },
                .Right => {
                    if self.direction != 3 {
                        self.nextDirection = 1;
                    }
                },
                .Down => {
                    if self.direction != 0 {
                        self.nextDirection = 2;
                    }
                },
                .Left => {
                    if self.direction != 1 {
                        self.nextDirection = 3;
                    }
                },
                _ => {}
            }
        }
    }

    mutating func handleGameOverInput() -> GameAction {
        for key in self.input.drainAll() {
            match key {
                .Space => return .Restart,
                .Quit => return .Quit,
                _ => {}
            }
        }
        .Continue
    }

    // ----------------------------------------
    // Update Logic
    // ----------------------------------------

    mutating func update() {
        match self.state {
            .Playing => {
                self.handlePlayingInput();

                // Apply buffered direction
                self.direction = self.nextDirection;

                // Store old head position (becomes first body segment)
                let oldHeadX = self.headX;
                let oldHeadY = self.headY;

                // Move head based on direction
                if self.direction == 0 {
                    self.headY = self.headY - 1;  // Up
                } else if self.direction == 1 {
                    self.headX = self.headX + 1;  // Right
                } else if self.direction == 2 {
                    self.headY = self.headY + 1;  // Down
                } else {
                    self.headX = self.headX - 1;  // Left
                }

                // Check wall collision
                if self.headX < 0 or self.headX >= Config.gameWidth or
                   self.headY < 0 or self.headY >= Config.gameHeight {
                    self.state = .GameOver;
                    return;
                }

                // Check self collision
                if self.isBody(x: self.headX, y: self.headY) {
                    self.state = .GameOver;
                    return;
                }

                // Check if eating food
                let ateFood = self.headX == self.foodX and self.headY == self.foodY;

                if ateFood {
                    self.score = self.score + 10;
                    // Add old head position to front of body (snake grows)
                    self.body.insert(Point(x: oldHeadX, y: oldHeadY), at: 0);
                    // Spawn new food
                    self.spawnFood();
                } else {
                    // Move body: shift all segments, add old head at front
                    var i = self.body.count - 1;
                    while i > 0 {
                        let prev = self.body(unchecked: i - 1);
                        self.body(unchecked: i) = prev;
                        i = i - 1;
                    }
                    self.body(unchecked: 0) = Point(x: oldHeadX, y: oldHeadY);
                }
            },
            .GameOver => {
                let action = self.handleGameOverInput();
                match action {
                    .Restart => self.reset(),
                    .Quit => {},  // Handle in main loop
                    .Continue => {}
                }
            }
        }
    }

    // Reset game state for restart
    mutating func reset() {
        self.headX = Config.gameWidth / 2;
        self.headY = Config.gameHeight / 2;
        self.direction = 1;
        self.nextDirection = 1;

        // Clear and reinitialize body
        self.body = Array[Point]();
        self.body.append(Point(x: self.headX - 1, y: self.headY));
        self.body.append(Point(x: self.headX - 2, y: self.headY));
        self.body.append(Point(x: self.headX - 3, y: self.headY));

        self.state = .Playing;
        self.score = 0;

        self.spawnFood();
    }

    // ----------------------------------------
    // Rendering
    // ----------------------------------------

    func render() {
        print(home());

        // Title and score
        print(moveTo(x: 2, y: 0));
        print(Styles.title("SNAKE") + "  " + Styles.label("Score: ") + Styles.score(self.score));
        print(clearLine());

        // Game box
        self.box.render();

        // Draw game field
        for y in Range[Int64](0, Config.gameHeight) {
            for x in Range[Int64](0, Config.gameWidth) {
                if x == self.headX and y == self.headY {
                    print(self.box.at(x: x, y: y) + Styles.head("◆"));
                } else if x == self.foodX and y == self.foodY {
                    print(self.box.at(x: x, y: y) + Styles.food("●"));
                } else if self.isBody(x: x, y: y) {
                    print(self.box.at(x: x, y: y) + Styles.body("█"));
                } else {
                    print(self.box.at(x: x, y: y) + " ");
                }
            }
        }

        // Instructions
        print(moveTo(x: 2, y: Config.gameHeight + 3));
        print(Styles.label("WASD or Arrow Keys to move | Ctrl+C to exit") + clearLine());
    }

    func renderGameOver() {
        print(home());

        // Title and score
        print(moveTo(x: 2, y: 0));
        print(Styles.title("SNAKE") + "  " + Styles.label("Score: ") + Styles.score(self.score));
        print(clearLine());

        // Game box
        self.box.render();

        // Clear interior
        for y in Range[Int64](0, Config.gameHeight) {
            print(self.box.at(x: 0, y: y) + repeatStr(s: " ", count: Config.gameWidth));
        }

        // Center messages
        let centerY = Config.gameHeight / 2;

        // GAME OVER
        let msg1 = "GAME OVER";
        let msg1X = (Config.gameWidth - 9) / 2;
        print(self.box.at(x: msg1X, y: centerY - 1) + Styles.gameOver(msg1));

        // Final Score
        let msg2X = (Config.gameWidth - 16) / 2;
        print(self.box.at(x: msg2X, y: centerY + 1) + Styles.score("Final Score: ") + Styles.score(self.score));

        // Prompt
        let msg3 = "SPACE = Restart  Q = Quit";
        let msg3X = (Config.gameWidth - 25) / 2;
        print(self.box.at(x: msg3X, y: centerY + 3) + Styles.prompt(msg3));

        // Clear instructions line
        print(moveTo(x: 2, y: Config.gameHeight + 3));
        print(clearLine());
    }

}

// ============================================
// External
// ============================================

@extern(.C, mangleName: "usleep")
func usleep(usec: UInt32) -> Int32

// ============================================
// Main
// ============================================

@main
func main() {
    var game = SnakeGame();

    // Game loop
    while true {
        game.update();

        match game.state {
            .Playing => game.render(),
            .GameOver => game.renderGameOver(),
        }

    // ~10 FPS for classic snake feel
    usleep(100000);
    }
}
