// A terminal-based Pong game in Kestrel using std2.
// Demonstrates: Structs, Methods, Dictionary, Optional, ANSI Graphics, and External C Calls.

module Pong

import io.stdio.(println, print)
import io.error.(Error)
import std.num.(Int64, Int32, UInt32)
import std.collections.(Dictionary)
import std.result.(Result, Optional)
import std.text.(String)
import std.core.(Bool, Equatable)

// Player keys for the score dictionary
public enum Player: Equatable {
    case player1
    case player2

    func description() -> String {
        match self {
            .player1 => "Player 1",
            .player2 => "Player 2",
        }
    }

    public func equals(other: Player) -> Bool {
        match (self, other) {
            (.player1, .player1) => true,
            (.player2, .player2) => true,
            _ => false
        }
    }
}

struct Pong {
    var ballX: Int64
    var ballY: Int64
    var ballDX: Int64
    var ballDY: Int64
    var paddle1Y: Int64
    var paddle2Y: Int64
    var scores: Dictionary[Player, Int64]

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

    init(width: Int64, height: Int64, paddleSize: Int64) {
        self.width = width;
        self.height = height;
        self.paddleSize = paddleSize;

        self.ballX = width / Int64(intLiteral: 2);
        self.ballY = height / Int64(intLiteral: 2);
        self.ballDX = Int64(intLiteral: 1);
        self.ballDY = Int64(intLiteral: 1);
        self.paddle1Y = (height - paddleSize) / Int64(intLiteral: 2);
        self.paddle2Y = (height - paddleSize) / Int64(intLiteral: 2);

        // Initialize trail to ball starting position
        self.trailX1 = self.ballX;
        self.trailY1 = self.ballY;
        self.trailX2 = self.ballX;
        self.trailY2 = self.ballY;
        self.trailX3 = self.ballX;
        self.trailY3 = self.ballY;

        // Using Dictionary from std.collections
        self.scores = Dictionary[Player, Int64](placeholderKey: .player1, placeholderValue: Int64(intLiteral: 0));
        self.scores.insert(.player1, Int64(intLiteral: 0));
        self.scores.insert(.player2, Int64(intLiteral: 0));
    }

    // Convenience init with defaults
    init() {
        self.init(width: Int64(intLiteral: 60), height: Int64(intLiteral: 20), paddleSize: Int64(intLiteral: 4));
    }

    func getScore(player player: Player) -> Int64 {
        match self.scores.getValue(player) {
            .Some(s) => s,
            .None => Int64(intLiteral: 0)
        }
    }

    mutating func addScore(player player: Player) {
        let current = self.getScore(player: player);
        self.scores.insert(player, current + Int64(intLiteral: 1));
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

    mutating func update() {
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

        // Simple AI for paddles
        // Paddle 1 follows the ball when it's on its side
        if self.ballX < 30 {
            if self.ballY > self.paddle1Y + 2 and self.paddle1Y < self.height - self.paddleSize {
                self.paddle1Y = self.paddle1Y + 1;
            } else if self.ballY < self.paddle1Y + 1 and self.paddle1Y > 0 {
                self.paddle1Y = self.paddle1Y - 1;
            }
        }

        // Paddle 2 follows the ball when it's on its side
        if self.ballX >= 30 {
            if self.ballY > self.paddle2Y + 2 and self.paddle2Y < self.height - self.paddleSize {
                self.paddle2Y = self.paddle2Y + 1;
            } else if self.ballY < self.paddle2Y + 1 and self.paddle2Y > 0 {
                self.paddle2Y = self.paddle2Y - 1;
            }
        }

        // Bounce off paddles
        if self.ballX == 1 {
            if self.ballY >= self.paddle1Y and self.ballY < self.paddle1Y + self.paddleSize {
                self.ballDX = 1;
            }
        } else if self.ballX == self.width - 2 {
            if self.ballY >= self.paddle2Y and self.ballY < self.paddle2Y + self.paddleSize {
                self.ballDX = -1;
            }
        }

        // Score detection
        if self.ballX < 0 {
            self.addScore(player: .player2);
            self.resetBall();
        } else if self.ballX >= self.width {
            self.addScore(player: .player1);
            self.resetBall();
        }
    }

    mutating func resetBall() {
        self.ballX = self.width / Int64(intLiteral: 2);
        self.ballY = self.height / Int64(intLiteral: 2);
        self.ballDX = Int64(intLiteral: 0) - self.ballDX;

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
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.width {
            print("═");
            i = i + Int64(intLiteral: 1);
        }
        println("╗\x1b[0m");

        // Game field
        let centerX = self.width / Int64(intLiteral: 2);
        var y: Int64 = Int64(intLiteral: 0);
        while y < self.height {
            print("\x1b[37m║\x1b[0m");

            var x: Int64 = Int64(intLiteral: 0);
            while x < self.width {
                if x == self.ballX and y == self.ballY {
                    // Yellow ball
                    print("\x1b[33m●\x1b[0m");
                } else if self.isTrail(x: x, y: y) {
                    // Gray trail
                    print("\x1b[90m·\x1b[0m");
                } else if x == Int64(intLiteral: 0) and self.isPaddle1(y: y) {
                    // Green paddle 1
                    print("\x1b[32m█\x1b[0m");
                } else if x == self.width - Int64(intLiteral: 1) and self.isPaddle2(y: y) {
                    // Cyan paddle 2
                    print("\x1b[36m█\x1b[0m");
                } else if x == centerX and y % Int64(intLiteral: 2) == Int64(intLiteral: 0) {
                    // Center line (every other row, ball/trail takes priority)
                    print("╎");
                } else {
                    print(" ");
                }
                x = x + Int64(intLiteral: 1);
            }

            println("\x1b[37m║\x1b[0m");
            y = y + Int64(intLiteral: 1);
        }

        // Bottom border: ╚═══...═══╝
        print("\x1b[37m╚");
        i = Int64(intLiteral: 0);
        while i < self.width {
            print("═");
            i = i + Int64(intLiteral: 1);
        }
        println("╝\x1b[0m");

        // Score box
        self.renderScoreBox();

        .Ok(())
    }

    func renderScoreBox() {
        let s1 = self.getScore(player: .player1);
        let s2 = self.getScore(player: .player2);

        // Score box top border
        print("\x1b[37m╔");
        var i: Int64 = Int64(intLiteral: 0);
        while i < self.width {
            print("═");
            i = i + Int64(intLiteral: 1);
        }
        println("╗\x1b[0m");

        // Score line with colored player names
        // Calculate padding for centered scores
        let halfWidth = self.width / Int64(intLiteral: 2);

        print("\x1b[37m║\x1b[0m");
        print("     \x1b[32mPLAYER 1:\x1b[0m \x1b[1;37m");
        print(intToString(s1));
        print("\x1b[0m");

        // Middle divider with padding
        var pad: Int64 = Int64(intLiteral: 0);
        while pad < halfWidth - Int64(intLiteral: 18) {
            print(" ");
            pad = pad + Int64(intLiteral: 1);
        }
        print("\x1b[37m║\x1b[0m");
        pad = Int64(intLiteral: 0);
        while pad < halfWidth - Int64(intLiteral: 18) {
            print(" ");
            pad = pad + Int64(intLiteral: 1);
        }

        print("\x1b[36mPLAYER 2:\x1b[0m \x1b[1;37m");
        print(intToString(s2));
        println("\x1b[0m     \x1b[37m║\x1b[0m");

        // Score box bottom border
        print("\x1b[37m╚");
        i = Int64(intLiteral: 0);
        while i < self.width {
            print("═");
            i = i + Int64(intLiteral: 1);
        }
        println("╝\x1b[0m");

        println("              (Press Ctrl+C to exit)");
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

func main() -> Result[(), Error] {
    // ANSI: Hide cursor and clear screen
    print("\x1b[?25l");
    print("\x1b[2J");

    var game = Pong();

    // Run for a fixed number of frames for this demo
    var frames: Int64 = Int64(intLiteral: 0);
    while frames < Int64(intLiteral: 200) {
        game.update();
        game.render();
        usleep(UInt32(intLiteral: 33333)); // ~30 FPS
        frames = frames + Int64(intLiteral: 1);
    }

    // ANSI: Show cursor again
    print("\x1b[?25h");
    println("\nGame demo complete.");

    .Ok(())
}
