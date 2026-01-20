// A terminal-based Pong game in Kestrel using std2.
// Demonstrates: Structs, Methods, ANSI Graphics, and External C Calls.

module Pong

struct Pong {
    var ballX: Int64
    var ballY: Int64
    var ballDX: Int64
    var ballDY: Int64
    var paddle1Y: Int64
    var paddle2Y: Int64
    var score1: Int64
    var score2: Int64

    // Trail tracking (3 positions)
    var trailX1: Int64
    var trailY1: Int64
    var trailX2: Int64
    var trailY2: Int64
    var trailX3: Int64
    var trailY3: Int64

    // Configurable settings
    var width: Int64
    var height: Int64
    var paddleSize: Int64

    // Player control (1 = left paddle, 2 = right paddle)
    var playerPaddle: Int64

    // Input state: lastDirection (-1=up, 0=none, 1=down), framesSinceInput
    var lastDirection: Int64
    var framesSinceInput: Int64

    // Frame counter for timing
    var frameCount: Int64

    init(width: Int64, height: Int64, paddleSize: Int64) {
        self.width = width;
        self.height = height;
        self.paddleSize = paddleSize;

        self.ballX = width / 2;
        self.ballY = height / 2;
        self.ballDX = 1;
        self.ballDY = 1;
        self.paddle1Y = (height - paddleSize) / 2;
        self.paddle2Y = (height - paddleSize) / 2;

        // Initialize trail to ball starting position
        self.trailX1 = self.ballX;
        self.trailY1 = self.ballY;
        self.trailX2 = self.ballX;
        self.trailY2 = self.ballY;
        self.trailX3 = self.ballX;
        self.trailY3 = self.ballY;

        // Player controls left paddle by default
        self.playerPaddle = 1;

        // Input state
        self.lastDirection = 0;
        self.framesSinceInput = 0;

        // Frame counter
        self.frameCount = 0;

        // Scores (testing with non-zero values)
        self.score1 = 42;
        self.score2 = 7;
    }

    // Convenience init with defaults
    init() {
        self.init(width: 60, height: 20, paddleSize: 4);
    }

    // Helper: check if position is part of the ball trail
    func isTrail(x x: Int64, y y: Int64) -> Bool {
        (x == self.trailX1 and y == self.trailY1) or
        (x == self.trailX2 and y == self.trailY2) or
        (x == self.trailX3 and y == self.trailY3)
    }

    // Helper: check if y position is within paddle 1
    func isPaddle1(y y: Int64) -> Bool {
        y >= self.paddle1Y and y < self.paddle1Y + self.paddleSize
    }

    // Helper: check if y position is within paddle 2
    func isPaddle2(y y: Int64) -> Bool {
        y >= self.paddle2Y and y < self.paddle2Y + self.paddleSize
    }

    // Handle keyboard input for player paddle
    mutating func handleInput() {
        // Player moves 1 unit per frame while key is held
        let speed = 1;

        // How many frames to keep moving after last key (prevents stutter)
        let holdTimeout = 16;

        // Read all queued keys and update direction
        var gotInput = false;
        var key = checkKey();
        while key != -1 {
            // Check for W/S or arrow keys
            if key == KEY_W() or key == KEY_UP() {
                self.lastDirection = -1;  // Up
                gotInput = true;
            } else if key == KEY_S() or key == KEY_DOWN() {
                self.lastDirection = 1;   // Down
                gotInput = true;
            }
            key = checkKey();
        }

        // Update frames since input
        if gotInput {
            self.framesSinceInput = 0;
        } else {
            self.framesSinceInput = self.framesSinceInput + 1;
        }

        // Stop if no input for too long
        if self.framesSinceInput > holdTimeout {
            self.lastDirection = 0;
        }

        // Move paddle based on current direction
        if self.lastDirection != 0 {
            if self.playerPaddle == 1 {
                self.paddle1Y = self.paddle1Y + self.lastDirection * speed;
            } else {
                self.paddle2Y = self.paddle2Y + self.lastDirection * speed;
            }
        }

        // Clamp paddle positions to screen bounds
        if self.paddle1Y < 0 {
            self.paddle1Y = 0;
        }
        if self.paddle1Y > self.height - self.paddleSize {
            self.paddle1Y = self.height - self.paddleSize;
        }
        if self.paddle2Y < 0 {
            self.paddle2Y = 0;
        }
        if self.paddle2Y > self.height - self.paddleSize {
            self.paddle2Y = self.height - self.paddleSize;
        }
    }

    mutating func update() {
        // Increment frame counter
        self.frameCount = self.frameCount + 1;

        // Handle player input first (every frame for responsiveness)
        self.handleInput();

        // Move ball every 2nd frame (slower ball)
        if self.frameCount % 2 == 0 {
            // Shift trail positions before moving ball
            self.trailX3 = self.trailX2;
            self.trailY3 = self.trailY2;
            self.trailX2 = self.trailX1;
            self.trailY2 = self.trailY1;
            self.trailX1 = self.ballX;
            self.trailY1 = self.ballY;

            // Move ball
            self.ballX = self.ballX + self.ballDX;
            self.ballY = self.ballY + self.ballDY;

            // Bounce off top/bottom
            if self.ballY <= 0 {
                self.ballY = 0;
                self.ballDY = 1;
            } else if self.ballY >= self.height - 1 {
                self.ballY = self.height - 1;
                self.ballDY = -1;
            }
        }

        // AI for the non-player paddle only (every 2nd frame)
        if self.frameCount % 2 == 0 {
            // Paddle 2 (AI) follows the ball when it's on its side
            if self.ballX >= (3 * self.width / 4) {
                if self.ballY > self.paddle2Y + 2 and self.paddle2Y < self.height - self.paddleSize {
                    self.paddle2Y = self.paddle2Y + 1;
                } else if self.ballY < self.paddle2Y + 1 and self.paddle2Y > 0 {
                    self.paddle2Y = self.paddle2Y - 1;
                }
            }
        }

        // Bounce off paddles with angle based on hit position
        if self.ballX == 1 {
            if self.ballY >= self.paddle1Y and self.ballY < self.paddle1Y + self.paddleSize {
                self.ballDX = 1;
                // Angle based on hit position
                let hitPos = self.ballY - self.paddle1Y;
                if hitPos == 0 {
                    self.ballDY = -1;  // Top of paddle → up
                } else if hitPos == self.paddleSize - 1 {
                    self.ballDY = 1;   // Bottom of paddle → down
                }
                // Middle keeps current ballDY
            }
        } else if self.ballX == self.width - 2 {
            if self.ballY >= self.paddle2Y and self.ballY < self.paddle2Y + self.paddleSize {
                self.ballDX = -1;
                // Angle based on hit position
                let hitPos = self.ballY - self.paddle2Y;
                if hitPos == 0 {
                    self.ballDY = -1;  // Top of paddle → up
                } else if hitPos == self.paddleSize - 1 {
                    self.ballDY = 1;   // Bottom of paddle → down
                }
                // Middle keeps current ballDY
            }
        }

        // Score detection
        if self.ballX < 0 {
            self.score2 = self.score2 + 1;
            self.resetBall();
        } else if self.ballX >= self.width {
            self.score1 = self.score1 + 1;
            self.resetBall();
        }
    }

    mutating func resetBall() {
        self.ballX = self.width / 2;
        self.ballY = self.height / 2;
        self.ballDX = 0 - self.ballDX;

        // Reset trail to new ball position
        self.trailX1 = self.ballX;
        self.trailY1 = self.ballY;
        self.trailX2 = self.ballX;
        self.trailY2 = self.ballY;
        self.trailX3 = self.ballX;
        self.trailY3 = self.ballY;
    }

    func render() -> Result[(), Error] {
        // ANSI: Move cursor to home position
        print("\x1b[H");

        // Top border: ╔═══...═══╗
        print("\x1b[37m╔");
        var i: Int64 = 0;
        while i < self.width {
            print("═");
            i = i + 1;
        }
        println("╗\x1b[0m\x1b[K");

        // Game field
        let centerX = self.width / 2;
        var y: Int64 = 0;
        while y < self.height {
            print("\x1b[37m║\x1b[0m");

            var x: Int64 = 0;
            while x < self.width {
                if x == self.ballX and y == self.ballY {
                    // Yellow ball
                    print("\x1b[33m●\x1b[0m");
                } else if self.isTrail(x: x, y: y) {
                    // Gray trail
                    print("\x1b[90m·\x1b[0m");
                } else if x == 0 and self.isPaddle1(y: y) {
                    // Green paddle 1
                    print("\x1b[32m█\x1b[0m");
                } else if x == self.width - 1 and self.isPaddle2(y: y) {
                    // Cyan paddle 2
                    print("\x1b[36m█\x1b[0m");
                } else if x == centerX and y % 2 == 0 {
                    // Center line (every other row, ball/trail takes priority)
                    print("╎");
                } else {
                    print(" ");
                }
                x = x + 1;
            }

            println("\x1b[37m║\x1b[0m\x1b[K");
            y = y + 1;
        }

        // Bottom border: ╚═══...═══╝
        print("\x1b[37m╚");
        i = 0;
        while i < self.width {
            print("═");
            i = i + 1;
        }
        println("╝\x1b[0m\x1b[K");

        // Score box
        self.renderScoreBox();

        .Ok(())
    }

    func renderScoreBox() {
        // Simple score line
        print("  \x1b[32mPLAYER 1:\x1b[0m \x1b[1;37m");
        print(intToString(self.score1));
        print("\x1b[0m");
        print("                    ");
        print("\x1b[36mPLAYER 2:\x1b[0m \x1b[1;37m");
        print(intToString(self.score2));
        println("\x1b[0m\x1b[K");

        println("  W/S or Arrow Keys to move | Ctrl+C to exit\x1b[K");
    }
}

// Convert Int64 to String
func intToString(n: Int64) -> String {
    if n == 0 {
        return "0"
    }

    var num = n;
    var negative = false;
    if num < 0 {
        negative = true;
        num = 0 - num;
    }

    var result = "";
    while num > 0 {
        let digit = num % 10;
        let ch = match digit {
            0 => "0",
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            5 => "5",
            6 => "6",
            7 => "7",
            8 => "8",
            _ => "9",
        };
        result = ch + result;
        num = num / 10;
    }

    if negative {
        result = "-" + result;
    }

    result
}

// Import usleep from C for timing
@extern(.C, mangleName: "usleep")
func usleep(usec: UInt32) -> Int32

// Keyboard input helpers (from pong_input.c)
@extern(.C, mangleName: "pong_init_terminal")
func initTerminal() -> Int32

@extern(.C, mangleName: "pong_restore_terminal")
func restoreTerminal() -> Int32

@extern(.C, mangleName: "pong_check_key")
func checkKey() -> Int32

// Key codes
func KEY_W() -> Int32 { 119 }
func KEY_S() -> Int32 { 115 }
func KEY_UP() -> Int32 { 1001 }
func KEY_DOWN() -> Int32 { 1002 }

func main() -> Result[(), Error] {
    // Initialize terminal for non-blocking input
    initTerminal();

    // ANSI: Hide cursor and clear screen
    print("\x1b[?25l");
    print("\x1b[2J");

    var game = Pong();

    // Run until Ctrl+C
    while true {
        game.update();
        game.render();
        usleep(16667); // ~60 FPS
    }

    // Cleanup (unreachable with Ctrl+C, but good practice)
    restoreTerminal();
    print("\x1b[?25h");
    println("\nGame demo complete.");

    .Ok(())
}
