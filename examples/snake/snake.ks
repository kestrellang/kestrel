// A terminal-based Snake game in Kestrel.
// Demonstrates: Structs, Methods, Arrays, ANSI Graphics, and External C Calls.

module Snake

struct Snake {
    // Snake position and movement
    var headX: Int64
    var headY: Int64
    var direction: Int64      // 0=up, 1=right, 2=down, 3=left
    var nextDirection: Int64  // Buffered direction to prevent 180-degree turns

    // Body segments (parallel arrays for x and y coordinates)
    var bodyX: Array[Int64]
    var bodyY: Array[Int64]
    var length: Int64

    // Food position
    var foodX: Int64
    var foodY: Int64

    // Game state
    var score: Int64
    var gameOver: Bool
    var width: Int64
    var height: Int64
    var frameCount: Int64

    // Simple random seed
    var seed: Int64

    init(width width: Int64, height height: Int64) {
        self.width = width;
        self.height = height;

        // Start snake in center, moving right
        self.headX = width / 2;
        self.headY = height / 2;
        self.direction = 1;  // Right
        self.nextDirection = 1;

        // Initialize body with 3 segments behind the head
        self.bodyX = Array[Int64]();
        self.bodyY = Array[Int64]();
        self.bodyX.append(self.headX - 1);
        self.bodyY.append(self.headY);
        self.bodyX.append(self.headX - 2);
        self.bodyY.append(self.headY);
        self.bodyX.append(self.headX - 3);
        self.bodyY.append(self.headY);
        self.length = 3;

        self.score = 0;
        self.gameOver = false;
        self.frameCount = 0;

        // Initialize random seed based on initial position
        self.seed = 12345;

        // Spawn initial food
        self.foodX = 0;
        self.foodY = 0;
        self.spawnFood();
    }

    // Convenience init with defaults
    init() {
        self.init(width: 40, height: 20);
    }

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
            let rx = self.randomNext() % self.width;
            let ry = self.randomNext() % self.height;

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
        var i: Int64 = 0;
        while i < self.length {
            let bx = self.bodyX.getUnchecked(i);
            let by = self.bodyY.getUnchecked(i);
            if bx == x and by == y {
                return true;
            }
            i = i + 1;
        }
        false
    }

    // Handle keyboard input
    mutating func handleInput() {
        var key = checkKey();
        while key != -1 {
            // W or Up arrow
            if key == KEY_W() or key == KEY_UP() {
                // Can't reverse into self (can't go up if going down)
                if self.direction != 2 {
                    self.nextDirection = 0;
                }
            }
            // D or Right arrow
            else if key == KEY_D() or key == KEY_RIGHT() {
                if self.direction != 3 {
                    self.nextDirection = 1;
                }
            }
            // S or Down arrow
            else if key == KEY_S() or key == KEY_DOWN() {
                if self.direction != 0 {
                    self.nextDirection = 2;
                }
            }
            // A or Left arrow
            else if key == KEY_A() or key == KEY_LEFT() {
                if self.direction != 1 {
                    self.nextDirection = 3;
                }
            }
            key = checkKey();
        }
    }

    // Handle input during game over state
    // Returns: 0 = continue waiting, 1 = restart, 2 = quit
    mutating func handleGameOverInput() -> Int64 {
        var key = checkKey();
        while key != -1 {
            // Space to restart
            if key == KEY_SPACE() {
                return 1;
            }
            // Q to quit
            if key == KEY_Q() or key == KEY_q() {
                return 2;
            }
            key = checkKey();
        }
        0
    }

    mutating func update() {
        if self.gameOver {
            return;
        }

        self.frameCount = self.frameCount + 1;

        // Handle input every frame
        self.handleInput();

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
        if self.headX < 0 or self.headX >= self.width or
           self.headY < 0 or self.headY >= self.height {
            self.gameOver = true;
            return;
        }

        // Check self collision
        if self.isBody(x: self.headX, y: self.headY) {
            self.gameOver = true;
            return;
        }

        // Check if eating food
        let ateFood = self.headX == self.foodX and self.headY == self.foodY;

        if ateFood {
            self.score = self.score + 10;
            // Add old head position to front of body (snake grows)
            self.bodyX.insert(oldHeadX, at: 0);
            self.bodyY.insert(oldHeadY, at: 0);
            self.length = self.length + 1;
            // Spawn new food
            self.spawnFood();
        } else {
            // Move body: shift all segments, add old head at front
            var i = self.length - 1;
            while i > 0 {
                let prevX = self.bodyX.getUnchecked(i - 1);
                let prevY = self.bodyY.getUnchecked(i - 1);
                self.bodyX.setUnchecked(i, prevX);
                self.bodyY.setUnchecked(i, prevY);
                i = i - 1;
            }
            self.bodyX.setUnchecked(0, oldHeadX);
            self.bodyY.setUnchecked(0, oldHeadY);
        }
    }

    // Reset game state for restart
    mutating func reset() {
        self.headX = self.width / 2;
        self.headY = self.height / 2;
        self.direction = 1;
        self.nextDirection = 1;

        // Clear and reinitialize body
        self.bodyX = Array[Int64]();
        self.bodyY = Array[Int64]();
        self.bodyX.append(self.headX - 1);
        self.bodyY.append(self.headY);
        self.bodyX.append(self.headX - 2);
        self.bodyY.append(self.headY);
        self.bodyX.append(self.headX - 3);
        self.bodyY.append(self.headY);
        self.length = 3;

        self.score = 0;
        self.gameOver = false;
        self.frameCount = 0;

        self.spawnFood();
    }

    func render() -> Result[(), Error] {
        // ANSI: Move cursor to home position
        print("\x1b[H");

        // Score line
        print("  \x1b[1;33mSNAKE\x1b[0m  Score: \x1b[1;37m");
        print(self.score);
        println("\x1b[0m\x1b[K");

        // Top border
        print("\x1b[37m╔");
        var i: Int64 = 0;
        while i < self.width {
            print("═");
            i = i + 1;
        }
        println("╗\x1b[0m\x1b[K");

        // Game field
        var y: Int64 = 0;
        while y < self.height {
            print("\x1b[37m║\x1b[0m");

            var x: Int64 = 0;
            while x < self.width {
                if x == self.headX and y == self.headY {
                    // Green snake head
                    print("\x1b[1;32m◆\x1b[0m");
                } else if x == self.foodX and y == self.foodY {
                    // Red food
                    print("\x1b[1;31m●\x1b[0m");
                } else if self.isBody(x: x, y: y) {
                    // Green snake body
                    print("\x1b[32m█\x1b[0m");
                } else {
                    print(" ");
                }
                x = x + 1;
            }

            println("\x1b[37m║\x1b[0m\x1b[K");
            y = y + 1;
        }

        // Bottom border
        print("\x1b[37m╚");
        i = 0;
        while i < self.width {
            print("═");
            i = i + 1;
        }
        println("╝\x1b[0m\x1b[K");

        // Controls hint
        println("  WASD or Arrow Keys to move | Ctrl+C to exit\x1b[K");

        .Ok(())
    }

    func renderGameOver() -> Result[(), Error] {
        // ANSI: Move cursor to home position
        print("\x1b[H");

        // Score line
        print("  \x1b[1;33mSNAKE\x1b[0m  Score: \x1b[1;37m");
        print(self.score);
        println("\x1b[0m\x1b[K");

        // Top border
        print("\x1b[37m╔");
        var i: Int64 = 0;
        while i < self.width {
            print("═");
            i = i + 1;
        }
        println("╗\x1b[0m\x1b[K");

        // Game field with game over message
        let msgY = self.height / 2 - 1;
        var y: Int64 = 0;
        while y < self.height {
            print("\x1b[37m║\x1b[0m");

            if y == msgY {
                // Center "GAME OVER" message
                let msg = "GAME OVER";
                let msgLen: Int64 = 9;
                let padding = (self.width - msgLen) / 2;
                var x: Int64 = 0;
                while x < padding {
                    print(" ");
                    x = x + 1;
                }
                print("\x1b[1;31mGAME OVER\x1b[0m");
                x = padding + msgLen;
                while x < self.width {
                    print(" ");
                    x = x + 1;
                }
            } else if y == msgY + 2 {
                // Center score message - "Final Score: X" is 14+ chars
                // Estimate score digits (1-3 digits typically)
                var scoreLen: Int64 = 1;
                if self.score >= 10 { scoreLen = 2; }
                if self.score >= 100 { scoreLen = 3; }
                if self.score >= 1000 { scoreLen = 4; }
                let msgLen = 13 + scoreLen;  // "Final Score: " = 13 chars
                let padding = (self.width - msgLen) / 2;
                var x: Int64 = 0;
                while x < padding {
                    print(" ");
                    x = x + 1;
                }
                print("\x1b[1;37mFinal Score: ");
                print(self.score);
                print("\x1b[0m");
                // Pad to end
                x = padding + msgLen;
                while x < self.width {
                    print(" ");
                    x = x + 1;
                }
            } else if y == msgY + 4 {
                // Center restart prompt - visible text is 26 chars
                // "SPACE = Restart  Q = Quit" = 25 chars visible
                let msgLen: Int64 = 25;
                let padding = (self.width - msgLen) / 2;
                var x: Int64 = 0;
                while x < padding {
                    print(" ");
                    x = x + 1;
                }
                print("\x1b[33mSPACE\x1b[0m = Restart  \x1b[33mQ\x1b[0m = Quit");
                x = padding + msgLen;
                while x < self.width {
                    print(" ");
                    x = x + 1;
                }
            } else {
                var x: Int64 = 0;
                while x < self.width {
                    print(" ");
                    x = x + 1;
                }
            }

            println("\x1b[37m║\x1b[0m\x1b[K");
            y = y + 1;
        }

        // Bottom border
        print("\x1b[37m╚");
        i = 0;
        while i < self.width {
            print("═");
            i = i + 1;
        }
        println("╝\x1b[0m\x1b[K");

        println("\x1b[K");  // Clear the controls line

        .Ok(())
    }
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
func KEY_A() -> Int32 { 97 }
func KEY_S() -> Int32 { 115 }
func KEY_D() -> Int32 { 100 }
func KEY_Q() -> Int32 { 81 }   // Uppercase Q
func KEY_q() -> Int32 { 113 }  // Lowercase q
func KEY_SPACE() -> Int32 { 32 }
func KEY_UP() -> Int32 { 1001 }
func KEY_DOWN() -> Int32 { 1002 }
func KEY_RIGHT() -> Int32 { 1003 }
func KEY_LEFT() -> Int32 { 1004 }

func main() -> Result[(), Error] {
    // Initialize terminal for non-blocking input
    initTerminal();

    // ANSI: Hide cursor and clear screen
    print("\x1b[?25l");
    print("\x1b[2J");

    var game = Snake();

    // Game loop
    var running = true;
    while running {
        if game.gameOver {
            let _ = game.renderGameOver();

            let action = game.handleGameOverInput();
            if action == 1 {
                // Restart
                game.reset();
            } else if action == 2 {
                // Quit
                running = false;
            }
        } else {
            game.update();
            let _ = game.render();
        }

        // ~10 FPS for classic snake feel
        usleep(100000);
    }

    // Cleanup
    restoreTerminal();
    print("\x1b[?25h");  // Show cursor
    print("\x1b[2J");    // Clear screen
    print("\x1b[H");     // Move to home
    println("Thanks for playing Snake!");

    .Ok(())
}
