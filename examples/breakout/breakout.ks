// Breakout - A terminal-based brick breaker game in Kestrel
// Demonstrates: TUI library usage, game loops, collision detection

module Breakout

import Tui.(Style, StyleOption, Box, moveTo, home, clearScreen, hideCursor, showCursor, clearLine, repeatStr)

// ============================================
// Game Constants
// ============================================

func GAME_WIDTH() -> Int64 { 60 }
func GAME_HEIGHT() -> Int64 { 24 }
func PADDLE_WIDTH() -> Int64 { 8 }
func BRICK_ROWS() -> Int64 { 5 }
func BRICK_COLS() -> Int64 { 14 }
func BRICK_WIDTH() -> Int64 { 4 }  // Each brick is 4 chars wide (2 "██")
func INITIAL_LIVES() -> Int64 { 3 }

// ============================================
// Styles
// ============================================

func borderStyle() -> Style { [.White, .Dim] }
func paddleStyle[F](f: F) -> String where F: Formattable { let style: Style = [.White, .Bold]; return style(f) }
func ballStyle[F](f: F) -> String where F: Formattable { let style: Style = [.Yellow, .Bold]; return style(f) }
func labelStyle[F](f: F) -> String where F: Formattable { let style: Style = [.Gray]; return style(f) }
func valueStyle[F](f: F) -> String where F: Formattable { let style: Style = [.White, .Bold]; return style(f) }
func livesStyle[F](f: F) -> String where F: Formattable { let style: Style = [.Red, .Bold]; return style(f) }
func gameOverStyle[F](f: F) -> String where F: Formattable { let style: Style = [.Red, .Bold]; return style(f) }
func winStyle[F](f: F) -> String where F: Formattable { let style: Style = [.Green, .Bold]; return style(f) }
func promptStyle[F](f: F) -> String where F: Formattable { let style: Style = [.Yellow]; return style(f) }

// Brick styles by row (top = most points, brightest colors)
func getBrickStyle(row row: Int64) -> Style {
    match row {
        0 => [.Red, .Bold],
        1 => [.Magenta, .Bold],
        2 => [.Yellow, .Bold],
        3 => [.Green, .Bold],
        _ => [.Cyan],
    }
}

// Points per row (top rows worth more)
func getBrickPoints(row row: Int64) -> Int64 {
    match row {
        0 => 50,
        1 => 40,
        2 => 30,
        3 => 20,
        _ => 10,
    }
}

// ============================================
// Game State
// ============================================

struct Breakout {
    // Game area
    var box: Box

    // Ball position and velocity (using fixed-point: multiply by 10)
    var ballX: Int64
    var ballY: Int64
    var ballDX: Int64
    var ballDY: Int64

    // Paddle
    var paddleX: Int64

    // Bricks: 2D array stored as 1D (row * BRICK_COLS + col)
    var bricks: Array[Bool]
    var bricksRemaining: Int64

    // Game state
    var score: Int64
    var lives: Int64
    var gameOver: Bool
    var won: Bool

    // Input state
    var lastDirection: Int64  // -1 = left, 0 = none, 1 = right
    var framesSinceInput: Int64

    // Frame counter
    var frameCount: Int64

    init() {
        self.box = Box(x: 0, y: 1, width: GAME_WIDTH() + 2, height: GAME_HEIGHT(), style: borderStyle());

        // Initialize paddle in center
        self.paddleX = (GAME_WIDTH() - PADDLE_WIDTH()) / 2;

        // Initialize ball above paddle
        self.ballX = GAME_WIDTH() / 2;
        self.ballY = GAME_HEIGHT() - 4;
        self.ballDX = 1;
        self.ballDY = -1;

        // Initialize bricks
        self.bricks = Array[Bool]();
        var i: Int64 = 0;
        while i < BRICK_ROWS() * BRICK_COLS() {
            self.bricks.append(true);
            i = i + 1;
        }
        self.bricksRemaining = BRICK_ROWS() * BRICK_COLS();

        self.score = 0;
        self.lives = INITIAL_LIVES();
        self.gameOver = false;
        self.won = false;

        self.lastDirection = 0;
        self.framesSinceInput = 0;
        self.frameCount = 0;
    }

    // Check if brick exists at row, col
    func hasBrick(row row: Int64, col col: Int64) -> Bool {
        if row < 0 or row >= BRICK_ROWS() or col < 0 or col >= BRICK_COLS() {
            return false
        }
        self.bricks.getUnchecked(row * BRICK_COLS() + col)
    }

    // Remove brick at row, col
    mutating func removeBrick(row row: Int64, col col: Int64) {
        if row >= 0 and row < BRICK_ROWS() and col >= 0 and col < BRICK_COLS() {
            let idx = row * BRICK_COLS() + col;
            if self.bricks.getUnchecked(idx) {
                self.bricks.setUnchecked(idx, false);
                self.bricksRemaining = self.bricksRemaining - 1;
                self.score = self.score + getBrickPoints(row: row);
            }
        }
    }

    // Handle keyboard input
    mutating func handleInput() {
        let speed = 2;
        let holdTimeout = 8;

        var gotInput = false;
        var key = checkKey();
        while key != -1 {
            if key == KEY_A() or key == KEY_LEFT() {
                self.lastDirection = -1;
                gotInput = true;
            } else if key == KEY_D() or key == KEY_RIGHT() {
                self.lastDirection = 1;
                gotInput = true;
            }
            key = checkKey();
        }

        if gotInput {
            self.framesSinceInput = 0;
        } else {
            self.framesSinceInput = self.framesSinceInput + 1;
        }

        if self.framesSinceInput > holdTimeout {
            self.lastDirection = 0;
        }

        // Move paddle
        if self.lastDirection != 0 {
            self.paddleX = self.paddleX + self.lastDirection * speed;
        }

        // Clamp paddle to game bounds
        if self.paddleX < 0 {
            self.paddleX = 0;
        }
        if self.paddleX > GAME_WIDTH() - PADDLE_WIDTH() {
            self.paddleX = GAME_WIDTH() - PADDLE_WIDTH();
        }
    }

    // Handle input during game over
    mutating func handleGameOverInput() -> Int64 {
        var key = checkKey();
        while key != -1 {
            if key == KEY_SPACE() {
                return 1  // Restart
            }
            if key == KEY_Q() or key == KEY_q() {
                return 2  // Quit
            }
            key = checkKey();
        }
        0  // Continue waiting
    }

    // Update game state
    mutating func update() {
        if self.gameOver {
            return
        }

        self.frameCount = self.frameCount + 1;
        self.handleInput();

        // Move ball every frame
        let newX = self.ballX + self.ballDX;
        let newY = self.ballY + self.ballDY;

        // Wall collisions (left/right)
        if newX < 0 {
            self.ballX = 0;
            self.ballDX = 0 - self.ballDX;
        } else if newX >= GAME_WIDTH() {
            self.ballX = GAME_WIDTH() - 1;
            self.ballDX = 0 - self.ballDX;
        } else {
            self.ballX = newX;
        }

        // Top wall collision
        if newY < 0 {
            self.ballY = 0;
            self.ballDY = 0 - self.ballDY;
        } else if newY >= GAME_HEIGHT() - 1 {
            // Ball fell through bottom - lose life
            self.lives = self.lives - 1;
            if self.lives <= 0 {
                self.gameOver = true;
                self.won = false;
            } else {
                self.resetBall();
            }
            return
        } else {
            self.ballY = newY;
        }

        // Paddle collision
        let paddleY = GAME_HEIGHT() - 2;
        if self.ballY == paddleY and self.ballDY > 0 {
            if self.ballX >= self.paddleX and self.ballX < self.paddleX + PADDLE_WIDTH() {
                self.ballDY = 0 - self.ballDY;

                // Angle based on where ball hits paddle
                let hitPos = self.ballX - self.paddleX;
                let center = PADDLE_WIDTH() / 2;
                if hitPos < center - 1 {
                    self.ballDX = -1;  // Hit left side - go left
                } else if hitPos > center + 1 {
                    self.ballDX = 1;   // Hit right side - go right
                }
                // Center keeps current direction
            }
        }

        // Brick collision
        // Bricks start at row 2 (after title row and top border padding)
        let brickAreaTop: Int64 = 1;
        let brickAreaBottom = brickAreaTop + BRICK_ROWS();

        if self.ballY >= brickAreaTop and self.ballY < brickAreaBottom {
            let brickRow = self.ballY - brickAreaTop;
            // Calculate which brick column based on ball X position
            // Bricks start with some padding from left wall
            let brickStartX: Int64 = 1;
            let brickCol = (self.ballX - brickStartX) / BRICK_WIDTH();

            if brickCol >= 0 and brickCol < BRICK_COLS() {
                if self.hasBrick(row: brickRow, col: brickCol) {
                    self.removeBrick(row: brickRow, col: brickCol);
                    self.ballDY = 0 - self.ballDY;

                    // Check for win
                    if self.bricksRemaining <= 0 {
                        self.gameOver = true;
                        self.won = true;
                    }
                }
            }
        }
    }

    // Reset ball after losing a life
    mutating func resetBall() {
        self.ballX = GAME_WIDTH() / 2;
        self.ballY = GAME_HEIGHT() - 4;
        self.ballDX = 1;
        self.ballDY = -1;
        self.paddleX = (GAME_WIDTH() - PADDLE_WIDTH()) / 2;
    }

    // Reset entire game
    mutating func reset() {
        self.resetBall();

        // Reset bricks
        var i: Int64 = 0;
        while i < BRICK_ROWS() * BRICK_COLS() {
            self.bricks.setUnchecked(i, true);
            i = i + 1;
        }
        self.bricksRemaining = BRICK_ROWS() * BRICK_COLS();

        self.score = 0;
        self.lives = INITIAL_LIVES();
        self.gameOver = false;
        self.won = false;
        self.lastDirection = 0;
        self.framesSinceInput = 0;
    }

    // Render the game
    func render() {
        print(home());

        // Score and lives line
        print(moveTo(x: 2, y: 0));
        print(labelStyle("Score: ") + valueStyle(self.score) + "    ");

        // Lives as hearts
        print(labelStyle("Lives: "));
        var l: Int64 = 0;
        while l < self.lives {
            print(livesStyle("♥"));
            l = l + 1;
        }
        print(clearLine());

        // Game box (open bottom)
        self.box.renderOpen();

        // Render bricks
        var row: Int64 = 0;
        while row < BRICK_ROWS() {
            let style = getBrickStyle(row: row);
            var col: Int64 = 0;
            while col < BRICK_COLS() {
                if self.hasBrick(row: row, col: col) {
                    let brickX = 1 + col * BRICK_WIDTH();
                    let brickY = row + 1;
                    print(self.box.at(x: brickX, y: brickY) + style("████"));
                }
                col = col + 1;
            }
            row = row + 1;
        }

        // Clear the play area below bricks (to erase old ball/paddle positions)
        var clearRow = BRICK_ROWS() + 1;
        while clearRow < GAME_HEIGHT() - 1 {
            print(self.box.at(x: 0, y: clearRow) + repeatStr(s: " ", count: GAME_WIDTH()));
            clearRow = clearRow + 1;
        }

        // Render ball
        print(self.box.at(x: self.ballX, y: self.ballY) + ballStyle("●"));

        // Render paddle
        let paddleY = GAME_HEIGHT() - 2;
        print(self.box.at(x: self.paddleX, y: paddleY) + paddleStyle(repeatStr(s: "▄", count: PADDLE_WIDTH())));

        // Instructions
        print(moveTo(x: 2, y: GAME_HEIGHT() + 1));
        print(labelStyle("A/D or Arrow Keys to move | Ctrl+C to exit") + clearLine());
    }

    // Render game over screen
    func renderGameOver() {
        print(home());

        // Score line
        print(moveTo(x: 2, y: 0));
        print(labelStyle("Score: ") + valueStyle(self.score) + clearLine());

        // Render box
        self.box.render();

        // Center message
        let centerY = GAME_HEIGHT() / 2;

        if self.won {
            let msg = "YOU WIN!";
            let msgX = (GAME_WIDTH() - 8) / 2;
            print(self.box.at(x: msgX, y: centerY) + winStyle(msg));
        } else {
            let msg = "GAME OVER";
            let msgX = (GAME_WIDTH() - 9) / 2;
            print(self.box.at(x: msgX, y: centerY) + gameOverStyle(msg));
        }

        // Final score
        let scoreMsg = "Final Score: ";
        let scoreMsgX = (GAME_WIDTH() - 16) / 2;
        print(self.box.at(x: scoreMsgX, y: centerY + 2) + valueStyle(scoreMsg) + valueStyle(self.score));

        // Prompt
        let promptMsg = "SPACE = Restart  Q = Quit";
        let promptX = (GAME_WIDTH() - 25) / 2;
        print(self.box.at(x: promptX, y: centerY + 4) + promptStyle(promptMsg));
    }
}

// ============================================
// External C functions
// ============================================

@extern(.C, mangleName: "usleep")
func usleep(usec: UInt32) -> Int32

@extern(.C, mangleName: "breakout_init_terminal")
func initTerminal() -> Int32

@extern(.C, mangleName: "breakout_restore_terminal")
func restoreTerminal() -> Int32

@extern(.C, mangleName: "breakout_check_key")
func checkKey() -> Int32

// Key codes
func KEY_A() -> Int32 { 97 }
func KEY_D() -> Int32 { 100 }
func KEY_Q() -> Int32 { 81 }
func KEY_q() -> Int32 { 113 }
func KEY_SPACE() -> Int32 { 32 }
func KEY_LEFT() -> Int32 { 1004 }
func KEY_RIGHT() -> Int32 { 1003 }

// ============================================
// Main
// ============================================

func main() -> Result[(), Error] {
    initTerminal();

    // Hide cursor and clear screen
    print(hideCursor() + clearScreen());

    var game = Breakout();

    // Game loop
    var running = true;
    while running {
        if game.gameOver {
            game.renderGameOver();

            let action = game.handleGameOverInput();
            if action == 1 {
                game.reset();
            } else if action == 2 {
                running = false;
            }
        } else {
            game.update();
            game.render();
        }

        usleep(16667);  // ~60 FPS
    }

    // Cleanup
    restoreTerminal();
    print(showCursor() + clearScreen() + home());
    println("Thanks for playing Breakout!");

    .Ok(())
}
