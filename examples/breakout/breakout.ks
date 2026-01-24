// Breakout - A terminal-based brick breaker game in Kestrel
// Demonstrates: TUI library, input handling, game loops, collision detection

module Breakout

import std.core.Range
import Tui.(Style, StyleOption, Box, moveTo, home, clearScreen, hideCursor, showCursor, clearLine, repeatStr)
import Input.(Key, InputManager)

// ============================================
// Configuration
// ============================================

struct Config {
    // Game area
    static var gameWidth: Int64 { 60 }
    static var gameHeight: Int64 { 24 }

    // Paddle
    static var paddleWidth: Int64 { 8 }
    static var paddleSpeed: Int64 { 2 }

    // Bricks
    static var brickRows: Int64 { 5 }
    static var brickCols: Int64 { 14 }
    static var brickWidth: Int64 { 4 }
    static var brickAreaTop: Int64 { 1 }

    // Gameplay
    static var initialLives: Int64 { 3 }
    static var holdTimeout: Int64 { 8 }

    // Points per row (top rows worth more)
    static func brickPoints(row row: Int64) -> Int64 {
        match row {
            0 => 50,
            1 => 40,
            2 => 30,
            3 => 20,
            _ => 10,
        }
    }

    // Brick style by row
    static func brickStyle(row row: Int64) -> Style {
        match row {
            0 => [.Red, .Bold],
            1 => [.Magenta, .Bold],
            2 => [.Yellow, .Bold],
            3 => [.Green, .Bold],
            _ => [.Cyan],
        }
    }
}

// ============================================
// Styles
// ============================================

struct Styles {
    static var border: Style { [.White, .Dim] }
    static var paddle: Style { [.White, .Bold] }
    static var ball: Style { [.Yellow, .Bold] }
    static var label: Style { [.Gray] }
    static var value: Style { [.White, .Bold] }
    static var lives: Style { [.Red, .Bold] }
    static var gameOver: Style { [.Red, .Bold] }
    static var win: Style { [.Green, .Bold] }
    static var prompt: Style { [.Yellow] }
}

// ============================================
// Ball
// ============================================

struct Ball {
    var x: Int64
    var y: Int64
    var dx: Int64
    var dy: Int64

    init(x x: Int64, y y: Int64) {
        self.x = x;
        self.y = y;
        self.dx = 1;
        self.dy = -1;
    }

    static func initial() -> Ball {
        Ball(
            x: Config.gameWidth / 2,
            y: Config.gameHeight - 4
        )
    }

    mutating func bounceHorizontal() {
        self.dx = -self.dx;
    }

    mutating func bounceVertical() {
        self.dy = -self.dy;
    }

    mutating func reset() {
        self.x = Config.gameWidth / 2;
        self.y = Config.gameHeight - 4;
        self.dx = 1;
        self.dy = -1;
    }
}

// ============================================
// Paddle
// ============================================

struct Paddle {
    var x: Int64

    var y: Int64 { Config.gameHeight - 2 }
    var width: Int64 { Config.paddleWidth }

    init() {
        self.x = (Config.gameWidth - Config.paddleWidth) / 2;
    }

    func contains(ballX ballX: Int64) -> Bool {
        ballX >= self.x and ballX < self.x + self.width
    }

    // Returns hit position relative to center (-1 = left, 0 = center, 1 = right)
    func hitPosition(ballX ballX: Int64) -> Int64 {
        let hitPos = ballX - self.x;
        let center = self.width / 2;
        if hitPos < center - 1 {
            -1  // Left side
        } else if hitPos > center + 1 {
            1   // Right side
        } else {
            0   // Center
        }
    }

    mutating func move(direction direction: Int64) {
        self.x = self.x + direction * Config.paddleSpeed;
        self.clamp();
    }

    mutating func clamp() {
        if self.x < 0 {
            self.x = 0;
        }
        let maxX = Config.gameWidth - self.width;
        if self.x > maxX {
            self.x = maxX;
        }
    }

    mutating func reset() {
        self.x = (Config.gameWidth - Config.paddleWidth) / 2;
    }
}

// ============================================
// BrickGrid
// ============================================

struct BrickGrid {
    var bricks: Array[Bool]
    var remaining: Int64

    init() {
        self.bricks = Array[Bool]();
        self.remaining = Config.brickRows * Config.brickCols;

        for _ in Range[Int64](0, self.remaining) {
            self.bricks.append(true);
        }
    }

    func hasBrick(row row: Int64, col col: Int64) -> Bool {
        if row < 0 or row >= Config.brickRows or col < 0 or col >= Config.brickCols {
            return false
        }
        self.bricks.getUnchecked(row * Config.brickCols + col)
    }

    // Remove brick and return points earned
    mutating func removeBrick(row row: Int64, col col: Int64) -> Int64 {
        if row < 0 or row >= Config.brickRows or col < 0 or col >= Config.brickCols {
            return 0
        }

        let idx = row * Config.brickCols + col;
        if self.bricks.getUnchecked(idx) {
            self.bricks.setUnchecked(idx, false);
            self.remaining = self.remaining - 1;
            return Config.brickPoints(row: row)
        }
        0
    }

    var allCleared: Bool { self.remaining <= 0 }

    mutating func reset() {
        for i in Range[Int64](0, Config.brickRows * Config.brickCols) {
            self.bricks.setUnchecked(i, true);
        }
        self.remaining = Config.brickRows * Config.brickCols;
    }
}

// ============================================
// Game State
// ============================================

enum GameState {
    case Playing
    case GameOver
    case Won
}

enum GameAction {
    case Continue
    case Restart
    case Quit
}

// ============================================
// Game
// ============================================

struct Game: Iterator {
    var state: GameState
    var ball: Ball
    var paddle: Paddle
    var bricks: BrickGrid
    var box: Box
    var score: Int64
    var lives: Int64
    var lastDirection: Int64
    var framesSinceInput: Int64
    var input: InputManager
    var running: Bool

    init() {
        self.state = .Playing;
        self.ball = Ball.initial();
        self.paddle = Paddle();
        self.bricks = BrickGrid();
        self.box = Box(x: 0, y: 1, width: Config.gameWidth + 2, height: Config.gameHeight, style: Styles.border);
        self.score = 0;
        self.lives = Config.initialLives;
        self.lastDirection = 0;
        self.framesSinceInput = 0;
        self.input = InputManager();
        self.running = true;

        print(hideCursor() + clearScreen());
    }

    // ----------------------------------------
    // Iterator
    // ----------------------------------------

    type Item = ()

    mutating func next() -> ()? {
        if not self.running {
            return null
        }

        match self.state {
            .Playing => {
                self.handlePlayingInput();
                self.updateBall();
                self.render();
            },
            _ => {
                print("Ended");
                self.renderGameOver();
                let action = self.handleGameOverInput();
                match action {
                    .Restart => self.reset(),
                    .Quit => { self.running = false; },
                    .Continue => {}
                }
            }
        }

        if self.running {
            print("Some In");
            .Some(())
        } else {
            print("None in");
            .None
        }
    }

    deinit {
        print(showCursor() + clearScreen() + home());
        println("Thanks for playing Breakout!");
    }

    // ----------------------------------------
    // Input Handling
    // ----------------------------------------

    mutating func handlePlayingInput() {
        var gotInput = false;

        for key in self.input.drainAll() {
            match key {
                .Left => {
                    self.lastDirection = -1;
                    gotInput = true;
                },
                .Right => {
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
            self.paddle.move(direction: self.lastDirection);
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

    mutating func updateBall() {
        let newX = self.ball.x + self.ball.dx;
        let newY = self.ball.y + self.ball.dy;

        // Wall collisions (left/right)
        if newX < 0 {
            self.ball.x = 0;
            self.ball.bounceHorizontal();
        } else if newX >= Config.gameWidth {
            self.ball.x = Config.gameWidth - 1;
            self.ball.bounceHorizontal();
        } else {
            self.ball.x = newX;
        }

        // Top wall collision
        if newY < 0 {
            self.ball.y = 0;
            self.ball.bounceVertical();
        } else if newY >= Config.gameHeight - 1 {
            // Ball fell through bottom
            self.loseLife();
            return
        } else {
            self.ball.y = newY;
        }

        // Paddle collision
        if self.ball.y == self.paddle.y and self.ball.dy > 0 {
            if self.paddle.contains(ballX: self.ball.x) {
                self.ball.bounceVertical();

                // Adjust angle based on hit position
                let hitPos = self.paddle.hitPosition(ballX: self.ball.x);
                if hitPos != 0 {
                    self.ball.dx = hitPos;
                }
            }
        }

        // Brick collision
        self.checkBrickCollision();
    }

    mutating func checkBrickCollision() {
        let brickAreaBottom = Config.brickAreaTop + Config.brickRows;

        if self.ball.y >= Config.brickAreaTop and self.ball.y < brickAreaBottom {
            let row = self.ball.y - Config.brickAreaTop;
            let col = (self.ball.x - 1) / Config.brickWidth;

            if col >= 0 and col < Config.brickCols {
                if self.bricks.hasBrick(row: row, col: col) {
                    let points = self.bricks.removeBrick(row: row, col: col);
                    self.score = self.score + points;
                    self.ball.bounceVertical();

                    if self.bricks.allCleared {
                        self.state = .Won;
                    }
                }
            }
        }
    }

    mutating func loseLife() {
        self.lives = self.lives - 1;
        if self.lives <= 0 {
            self.state = .GameOver;
        } else {
            self.ball.reset();
            self.paddle.reset();
        }
    }

    mutating func reset() {
        self.state = .Playing;
        self.ball.reset();
        self.paddle.reset();
        self.bricks.reset();
        self.score = 0;
        self.lives = Config.initialLives;
        self.lastDirection = 0;
        self.framesSinceInput = 0;
    }

    // ----------------------------------------
    // Rendering
    // ----------------------------------------

    func render() {
        print(home());

        // Score and lives
        print(moveTo(x: 2, y: 0));
        print(Styles.label("Score: ") + Styles.value(self.score) + "    ");
        print(Styles.label("Lives: "));
        for _ in Range[Int64](0, self.lives) {
            print(Styles.lives("♥"));
        }
        print(clearLine());

        // Game box
        self.box.renderOpen();

        // Clear entire play area inside box (prevents ball trail artifacts)
        for clearRow in Range[Int64](0, Config.gameHeight - 1) {
            print(self.box.at(x: 0, y: clearRow) + repeatStr(s: " ", count: Config.gameWidth));
        }

        // Bricks
        self.renderBricks();

        // Ball
        print(self.box.at(x: self.ball.x, y: self.ball.y) + Styles.ball("●"));

        // Paddle
        print(self.box.at(x: self.paddle.x, y: self.paddle.y) + Styles.paddle(repeatStr(s: "▄", count: self.paddle.width)));

        // Instructions
        print(moveTo(x: 2, y: Config.gameHeight + 1));
        print(Styles.label("A/D or Arrow Keys to move | Ctrl+C to exit") + clearLine());
    }

    func renderBricks() {
        for row in Range[Int64](0, Config.brickRows) {
            let style = Config.brickStyle(row: row);
            for col in Range[Int64](0, Config.brickCols) {
                let brickX = 1 + col * Config.brickWidth;
                let brickY = row + 1;
                if self.bricks.hasBrick(row: row, col: col) {
                    print(self.box.at(x: brickX, y: brickY) + style("████"));
                } else {
                    print(self.box.at(x: brickX, y: brickY) + "    ");
                }
            }
        }
    }

    func renderGameOver() {
        print(home());

        // Score
        print(moveTo(x: 2, y: 0));
        print(Styles.label("Score: ") + Styles.value(self.score) + clearLine());

        // Box
        self.box.render();

        // Center message
        let centerY = Config.gameHeight / 2;

        match self.state {
            .Won => {
                let msg = "YOU WIN!";
                let msgX = (Config.gameWidth - 8) / 2;
                print(self.box.at(x: msgX, y: centerY) + Styles.win(msg));
            },
            _ => {
                let msg = "GAME OVER";
                let msgX = (Config.gameWidth - 9) / 2;
                print(self.box.at(x: msgX, y: centerY) + Styles.gameOver(msg));
            }
        }

        // Final score
        let scoreMsg = "Final Score: ";
        let scoreMsgX = (Config.gameWidth - 16) / 2;
        print(self.box.at(x: scoreMsgX, y: centerY + 2) + Styles.value(scoreMsg) + Styles.value(self.score));

        // Prompt
        let promptMsg = "SPACE = Restart  Q = Quit";
        let promptX = (Config.gameWidth - 25) / 2;
        print(self.box.at(x: promptX, y: centerY + 4) + Styles.prompt(promptMsg));
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

func main() -> () throws Error {
    var game = Game();

    let iterator = game.iter();

    while let .Some(x) = iterator.next() {
        usleep(16667);
    }

    //for _ in game {
    //    usleep(16667);  // ~60 FPS
    //}

    .Ok(())
}
