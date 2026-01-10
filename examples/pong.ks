// A terminal-based Pong game in Kestrel using the standard library.
// Demonstrates: Structs, Methods, Dictionary, Optional, ANSI Graphics, and External C Calls.

module Pong

import io
import std.core.(Int, UInt32)
import std.collections.(Dictionary)
import std.result.(Result, Optional)

// Player keys for the score dictionary
enum Player {
    case player1
    case player2

    func description() -> String {
        match self {
            .player1 => "Player 1",
            .player2 => "Player 2",
        }
    }
}

struct Pong {
    var ballX: Int
    var ballY: Int
    var ballDX: Int
    var ballDY: Int
    var paddle1Y: Int
    var paddle2Y: Int
    var scores: Dictionary[Player, Int]
    
    // Constants as computed properties
    var width: Int { 60 }
    var height: Int { 20 }
    var paddleSize: Int { 4 }

    init() {
        self.ballX = 30;
        self.ballY = 10;
        self.ballDX = 1;
        self.ballDY = 1;
        self.paddle1Y = 8;
        self.paddle2Y = 8;
        
        // Using Dictionary from std.collections
        var scores = Dictionary[Player, Int]();
        scores.insert(value: 0, for: .player1);
        scores.insert(value: 0, for: .player2);
        self.scores = scores;
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
            let current = self.scores.getOrInsert(key: .player2, default: 0);
            self.scores.insert(value: current + 1, for: .player2);
            self.resetBall();
        } else if self.ballX >= self.width {
            let current = self.scores.getOrInsert(key: .player1, default: 0);
            self.scores.insert(value: current + 1, for: .player1);
            self.resetBall();
        }
    }

    mutating func resetBall() {
        self.ballX = self.width / 2;
        self.ballY = self.height / 2;
        self.ballDX = 0 - self.ballDX;
    }

    func render() -> Result[Unit] {
        // ANSI: Move cursor to home position
        try io.print(s: "\x1b[H");

        // Top border
        var topBorder = "+";
        var i = 0;
        while i < self.width {
            topBorder = topBorder + "-";
            i = i + 1;
        }
        try io.println(s: topBorder + "+");

        // Game field
        var y = 0;
        while y < self.height {
            var line = "|";
            var x = 0;
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
        
        // Display scores using Dictionary and primitive toString()
        let s1 = self.scores.getOrInsert(key: .player1, default: 0);
        let s2 = self.scores.getOrInsert(key: .player2, default: 0);
        
        try io.println(s: " " + Player.player1.description() + ": " + s1.toString() + " | " + Player.player2.description() + ": " + s2.toString());
        try io.println(s: " (Press Ctrl+C to exit)");
        
        .Ok(())
    }
}

// Import usleep from C for timing
@extern(.C, mangleName: "usleep")
func usleep(usec: UInt32) -> Int32

func main() -> Result[Unit] {
    // ANSI: Hide cursor and clear screen
    try io.print(s: "\x1b[?25l");
    try io.print(s: "\x1b[2J");

    var game = Pong();
    
    // Run for a fixed number of frames for this demo
    var frames = 0;
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
