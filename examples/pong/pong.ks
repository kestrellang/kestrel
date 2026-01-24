// A terminal-based Pong game in Kestrel using shared game libraries.
// Demonstrates: Structs, Methods, ANSI Graphics, and External C Calls.

module Pong

import std.core.Range
import Tui.(Style, StyleOption, Box, moveTo, home, clearScreen, hideCursor, showCursor, clearLine, repeatStr)
import Input.(Key, InputManager)

// ============================================
// Configuration
// ============================================

struct Config {
    static var gameWidth: Int64 { 60 }
    static var gameHeight: Int64 { 20 }
    static var paddleSize: Int64 { 4 }
    static var holdTimeout: Int64 { 16 }
}

// ============================================
// Styles
// ============================================

struct Styles {
    static var border: Style { [.White, .Dim] }
    static var paddle1: Style { [.Green, .Bold] }
    static var paddle2: Style { [.Cyan, .Bold] }
    static var ball: Style { [.Yellow, .Bold] }
    static var trail: Style { [.Gray] }
    static var label: Style { [.Gray] }
    static var value: Style { [.White, .Bold] }
    static var centerLine: Style { [.White, .Dim] }
}

// ============================================
// Ball
// ============================================

struct Ball {
    var x: Int64
    var y: Int64
    var dx: Int64
    var dy: Int64

    // Trail tracking (3 positions)
    var trailX1: Int64
    var trailY1: Int64
    var trailX2: Int64
    var trailY2: Int64
    var trailX3: Int64
    var trailY3: Int64

    init() {
        self.x = Config.gameWidth / 2;
        self.y = Config.gameHeight / 2;
        self.dx = 1;
        self.dy = 1;

        // Initialize trail to ball starting position
        self.trailX1 = self.x;
        self.trailY1 = self.y;
        self.trailX2 = self.x;
        self.trailY2 = self.y;
        self.trailX3 = self.x;
        self.trailY3 = self.y;
    }

    func isTrail(x x: Int64, y y: Int64) -> Bool {
        (x == self.trailX1 and y == self.trailY1) or
        (x == self.trailX2 and y == self.trailY2) or
        (x == self.trailX3 and y == self.trailY3)
    }

    mutating func updateTrail() {
        self.trailX3 = self.trailX2;
        self.trailY3 = self.trailY2;
        self.trailX2 = self.trailX1;
        self.trailY2 = self.trailY1;
        self.trailX1 = self.x;
        self.trailY1 = self.y;
    }

    mutating func move() {
        self.x = self.x + self.dx;
        self.y = self.y + self.dy;
    }

    mutating func bounceHorizontal() {
        self.dx = -self.dx;
    }

    mutating func bounceVertical() {
        self.dy = -self.dy;
    }

    mutating func reset() {
        self.x = Config.gameWidth / 2;
        self.y = Config.gameHeight / 2;
        self.dx = 0 - self.dx;

        // Reset trail to new ball position
        self.trailX1 = self.x;
        self.trailY1 = self.y;
        self.trailX2 = self.x;
        self.trailY2 = self.y;
        self.trailX3 = self.x;
        self.trailY3 = self.y;
    }
}

// ============================================
// Paddle
// ============================================

struct Paddle {
    var y: Int64

    init() {
        self.y = (Config.gameHeight - Config.paddleSize) / 2;
    }

    func contains(ballY ballY: Int64) -> Bool {
        ballY >= self.y and ballY < self.y + Config.paddleSize
    }

    // Returns hit position relative to paddle (0=top, paddleSize-1=bottom)
    func hitPosition(ballY ballY: Int64) -> Int64 {
        ballY - self.y
    }

    mutating func move(direction direction: Int64) {
        self.y = self.y + direction;
        self.clamp();
    }

    mutating func clamp() {
        if self.y < 0 {
            self.y = 0;
        }
        let maxY = Config.gameHeight - Config.paddleSize;
        if self.y > maxY {
            self.y = maxY;
        }
    }

    mutating func reset() {
        self.y = (Config.gameHeight - Config.paddleSize) / 2;
    }
}

// ============================================
// Game
// ============================================

struct Game: not Copyable {
    var ball: Ball
    var paddle1: Paddle
    var paddle2: Paddle
    var box: Box
    var score1: Int64
    var score2: Int64

    // Player controls paddle 1 (left)
    var lastDirection: Int64
    var framesSinceInput: Int64

    // Frame counter for timing
    var frameCount: Int64

    // Previous positions for clearing
    var prevPaddle1Y: Int64
    var prevPaddle2Y: Int64
    var prevTrailX3: Int64
    var prevTrailY3: Int64

    var input: InputManager

    init() {
        self.ball = Ball();
        self.paddle1 = Paddle();
        self.paddle2 = Paddle();
        self.box = Box(x: 0, y: 1, width: Config.gameWidth + 2, height: Config.gameHeight + 2, style: Styles.border);
        self.score1 = 0;
        self.score2 = 0;
        self.lastDirection = 0;
        self.framesSinceInput = 0;
        self.frameCount = 0;
        self.prevPaddle1Y = self.paddle1.y;
        self.prevPaddle2Y = self.paddle2.y;
        self.prevTrailX3 = self.ball.trailX3;
        self.prevTrailY3 = self.ball.trailY3;
        self.input = InputManager();

        print(hideCursor() + clearScreen());
    }

    deinit {
        print(showCursor() + clearScreen() + home());
        println("Thanks for playing Pong!");
    }

    // ----------------------------------------
    // Input Handling
    // ----------------------------------------

    mutating func handleInput() {
        var gotInput = false;

        for key in self.input.drainAll() {
            match key {
                .Up => {
                    self.lastDirection = -1;
                    gotInput = true;
                },
                .Down => {
                    self.lastDirection = 1;
                    gotInput = true;
                },
                _ => {}
            }
        }

        if gotInput {
            self.framesSinceInput = 0;
        } else {
            self.framesSinceInput = self.framesSinceInput + 1;
        }

        if self.framesSinceInput > Config.holdTimeout {
            self.lastDirection = 0;
        }

        if self.lastDirection != 0 {
            self.paddle1.move(direction: self.lastDirection);
        }
    }

    // ----------------------------------------
    // Update Logic
    // ----------------------------------------

    mutating func update() {
        self.frameCount = self.frameCount + 1;

        // Handle player input every frame for responsiveness
        self.handleInput();

        // Move ball every 2nd frame (slower ball)
        if self.frameCount % 2 == 0 {
            self.ball.updateTrail();
            self.ball.move();

            // Bounce off top/bottom
            if self.ball.y <= 0 {
                self.ball.y = 0;
                self.ball.dy = 1;
            } else if self.ball.y >= Config.gameHeight - 1 {
                self.ball.y = Config.gameHeight - 1;
                self.ball.dy = -1;
            }
        }

        // AI for paddle 2 (right) - every 2nd frame
        if self.frameCount % 2 == 0 {
            if self.ball.x >= (3 * Config.gameWidth / 4) {
                if self.ball.y > self.paddle2.y + 2 and self.paddle2.y < Config.gameHeight - Config.paddleSize {
                    self.paddle2.move(direction: 1);
                } else if self.ball.y < self.paddle2.y + 1 and self.paddle2.y > 0 {
                    self.paddle2.move(direction: -1);
                }
            }
        }

        // Paddle collision with angle based on hit position
        if self.ball.x == 1 {
            if self.paddle1.contains(ballY: self.ball.y) {
                self.ball.dx = 1;
                let hitPos = self.paddle1.hitPosition(ballY: self.ball.y);
                if hitPos == 0 {
                    self.ball.dy = -1;  // Top of paddle -> up
                } else if hitPos == Config.paddleSize - 1 {
                    self.ball.dy = 1;   // Bottom of paddle -> down
                }
            }
        } else if self.ball.x == Config.gameWidth - 2 {
            if self.paddle2.contains(ballY: self.ball.y) {
                self.ball.dx = -1;
                let hitPos = self.paddle2.hitPosition(ballY: self.ball.y);
                if hitPos == 0 {
                    self.ball.dy = -1;  // Top of paddle -> up
                } else if hitPos == Config.paddleSize - 1 {
                    self.ball.dy = 1;   // Bottom of paddle -> down
                }
            }
        }

        // Score detection
        if self.ball.x < 0 {
            self.score2 = self.score2 + 1;
            self.ball.reset();
        } else if self.ball.x >= Config.gameWidth {
            self.score1 = self.score1 + 1;
            self.ball.reset();
        }
    }

    // ----------------------------------------
    // Rendering
    // ----------------------------------------

    mutating func render() {
        print(home());

        // Score display
        print(moveTo(x: 2, y: 0));
        print(Styles.label("PLAYER 1: ") + Styles.paddle1(self.score1));
        print("                    ");
        print(Styles.label("PLAYER 2: ") + Styles.paddle2(self.score2));
        print(clearLine());

        // Game box (only needs to be drawn once, but redrawing is fine)
        self.box.render();

        // Clear old trail position (the one that's no longer displayed)
        let centerX = Config.gameWidth / 2;
        if self.prevTrailX3 == centerX and self.prevTrailY3 % 2 == 0 {
            print(self.box.at(x: self.prevTrailX3, y: self.prevTrailY3) + Styles.centerLine("╎"));
        } else {
            print(self.box.at(x: self.prevTrailX3, y: self.prevTrailY3) + " ");
        }

        // Clear old paddle 1 positions that are no longer occupied
        if self.prevPaddle1Y < self.paddle1.y {
            // Paddle moved down, clear top
            for i in Range[Int64](self.prevPaddle1Y, self.paddle1.y) {
                print(self.box.at(x: 0, y: i) + " ");
            }
        } else if self.prevPaddle1Y > self.paddle1.y {
            // Paddle moved up, clear bottom
            for i in Range[Int64](self.paddle1.y + Config.paddleSize, self.prevPaddle1Y + Config.paddleSize) {
                print(self.box.at(x: 0, y: i) + " ");
            }
        }

        // Clear old paddle 2 positions that are no longer occupied
        if self.prevPaddle2Y < self.paddle2.y {
            for i in Range[Int64](self.prevPaddle2Y, self.paddle2.y) {
                print(self.box.at(x: Config.gameWidth - 1, y: i) + " ");
            }
        } else if self.prevPaddle2Y > self.paddle2.y {
            for i in Range[Int64](self.paddle2.y + Config.paddleSize, self.prevPaddle2Y + Config.paddleSize) {
                print(self.box.at(x: Config.gameWidth - 1, y: i) + " ");
            }
        }

        // Draw center line (only where not covered by ball/trail)
        for y in Range[Int64](0, Config.gameHeight) {
            if y % 2 == 0 {
                print(self.box.at(x: centerX, y: y) + Styles.centerLine("╎"));
            }
        }

        // Draw paddles
        for i in Range[Int64](0, Config.paddleSize) {
            print(self.box.at(x: 0, y: self.paddle1.y + i) + Styles.paddle1("█"));
            print(self.box.at(x: Config.gameWidth - 1, y: self.paddle2.y + i) + Styles.paddle2("█"));
        }

        // Draw ball trail
        print(self.box.at(x: self.ball.trailX1, y: self.ball.trailY1) + Styles.trail("·"));
        print(self.box.at(x: self.ball.trailX2, y: self.ball.trailY2) + Styles.trail("·"));
        print(self.box.at(x: self.ball.trailX3, y: self.ball.trailY3) + Styles.trail("·"));

        // Draw ball
        print(self.box.at(x: self.ball.x, y: self.ball.y) + Styles.ball("●"));

        // Store previous positions for next frame
        self.prevPaddle1Y = self.paddle1.y;
        self.prevPaddle2Y = self.paddle2.y;
        self.prevTrailX3 = self.ball.trailX3;
        self.prevTrailY3 = self.ball.trailY3;

        // Instructions
        print(moveTo(x: 2, y: Config.gameHeight + 3));
        print(Styles.label("W/S or Arrow Keys to move | Ctrl+C to exit") + clearLine());
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

func main() -> Result[(), Error] {
    var game = Game();

    // Run until Ctrl+C
    while true {
        game.update();
        game.render();
        usleep(16667); // ~60 FPS
    }

    .Ok(())
}
