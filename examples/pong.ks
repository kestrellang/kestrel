// A terminal-based Pong game in Kestrel using std2.
// Demonstrates: Structs, Methods, Dictionary, Optional, ANSI Graphics, and External C Calls.

module Pong

import io
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

    // Constants as computed properties
    var width: Int64 { 60 }
    var height: Int64 { 20 }
    var paddleSize: Int64 { 4 }

    init() {
        self.ballX = 30;
        self.ballY = 10;
        self.ballDX = 1;
        self.ballDY = 1;
        self.paddle1Y = 8;
        self.paddle2Y = 8;

        // Using Dictionary from std.collections
        self.scores = Dictionary[Player, Int64]();
        self.scores.insert(.player1, 0);
        self.scores.insert(.player2, 0);
    }

    func getScore(player player: Player) -> Int64 {
        match self.scores.getValue(player) {
            .Some(s) => s,
            .None => 0
        }
    }

    mutating func addScore(player player: Player) {
        let current = self.getScore(player: player);
        self.scores.insert(player, current + 1);
    }

    mutating func update() {
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
        self.ballX = self.width / 2;
        self.ballY = self.height / 2;
        self.ballDX = 0 - self.ballDX;
    }

    func render() -> Result[(), Error] {
        // ANSI: Move cursor to home position
        try io.print(s: "\x1b[H");

        // Top border
        var topBorder = "+";
        var i: Int64 = 0;
        while i < self.width {
            topBorder = topBorder + "-";
            i = i + 1;
        }
        try io.println(s: topBorder + "+");

        // Game field
        var y: Int64 = 0;
        while y < self.height {
            var line = "|";
            var x: Int64 = 0;
            while x < self.width {
                if x == self.ballX and y == self.ballY {
                    line = line + "O"; // Ball
                } else if x == 0 and y >= self.paddle1Y and y < self.paddle1Y + self.paddleSize {
                    line = line + "#"; // Paddle 1
                } else if x == self.width - 1 and y >= self.paddle2Y and y < self.paddle2Y + self.paddleSize {
                    line = line + "#"; // Paddle 2
                } else {
                    line = line + " ";
                }
                x = x + 1;
            }
            try io.println(s: line + "|");
            y = y + 1;
        }

        try io.println(s: topBorder + "+");

        // Display scores
        let s1 = self.getScore(player: .player1);
        let s2 = self.getScore(player: .player2);

        try io.println(s: " " + Player.player1.description() + ": " + s1.toString() + " | " + Player.player2.description() + ": " + s2.toString());
        try io.println(s: " (Press Ctrl+C to exit)");

        .Ok(())
    }
}

// Import usleep from C for timing
@extern(.C, mangleName: "usleep")
func usleep(usec: UInt32) -> Int32

func main() -> Result[(), Error] {
    // ANSI: Hide cursor and clear screen
    try io.print(s: "\x1b[?25l");
    try io.print(s: "\x1b[2J");

    var game = Pong();

    // Run for a fixed number of frames for this demo
    var frames: Int64 = 0;
    while frames < 500 {
        game.update();
        try game.render();
        usleep(33333); // ~30 FPS
        frames = frames + 1;
    }

    // ANSI: Show cursor again
    try io.print(s: "\x1b[?25h");
    try io.println(s: "\nGame demo complete.");

    .Ok(())
}
