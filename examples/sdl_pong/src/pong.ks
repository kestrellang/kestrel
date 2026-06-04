module SdlPong

import sdl.(Color, Rectangle, Milliseconds, Key, Event, Renderer, SDLApp)

// --- Game Components ---

struct Vector2 {
    var x: Float64
    var y: Float64

    static func zero() -> Vector2 {
        Vector2(x: 0.0, y: 0.0)
    }
}

struct Ball {
    var position: Vector2
    var velocity: Vector2
    var size: Int64

    static func create() -> Ball {
        Ball(
            position: Vector2(x: 400.0, y: 300.0),
            velocity: Vector2(x: 12.0, y: 8.0),
            size: 10
        )
    }

    mutating func update() {
        self.position.x = self.position.x + self.velocity.x;
        self.position.y = self.position.y + self.velocity.y;

        // Bounce off top/bottom walls
        if self.position.y < 0.0 {
            self.position.y = 0.0;
            self.velocity.y = 0.0 - self.velocity.y;
        } else if self.position.y > 590.0 {
            self.position.y = 590.0;
            self.velocity.y = 0.0 - self.velocity.y;
        }
    }

    mutating func reset(direction direction: Float64) {
        self.position = Vector2(x: 400.0, y: 300.0);
        self.velocity = Vector2(x: 12.0 * direction, y: 8.0);
    }

    func render(renderer: Renderer) {
        let rect = Rectangle(
            x: self.position.x.toInt64().unwrap(),
            y: self.position.y.toInt64().unwrap(),
            width: self.size,
            height: self.size
        );
        renderer.fill(rect, Color.yellow());
    }
}

struct Paddle {
    var x: Int64
    var y: Float64
    var width: Int64
    var height: Int64
    var speed: Float64
    var color: Color

    static func left() -> Paddle {
        Paddle(x: 10, y: 250.0, width: 20, height: 100, speed: 10.0, color: Color.cyan())
    }

    static func right() -> Paddle {
        Paddle(x: 770, y: 250.0, width: 20, height: 100, speed: 7.2, color: Color.magenta())
    }

    mutating func moveUp() {
        self.y = self.y - self.speed;
        self.clamp();
    }

    mutating func moveDown() {
        self.y = self.y + self.speed;
        self.clamp();
    }

    mutating func clamp() {
        if self.y < 0.0 {
            self.y = 0.0;
        } else if self.y > 500.0 {
            self.y = 500.0;
        }
    }

    mutating func trackBall(ball: Ball) {
        let paddleCenter = self.y + 50.0;
        if ball.position.y > paddleCenter {
            self.moveDown();
        } else {
            self.moveUp();
        }
    }

    func containsY(y: Float64) -> Bool {
        y >= self.y and y <= self.y + Float64(from: self.height)
    }

    func render(renderer: Renderer) {
        let rect = Rectangle(x: self.x, y: self.y.toInt64().unwrap(), width: self.width, height: self.height);
        renderer.fill(rect, self.color);
    }
}

struct Score {
    var player1: Int64
    var player2: Int64

    init() {
        self.player1 = 0;
        self.player2 = 0;
    }

    mutating func player1Scores() {
        self.player1 = self.player1 + 1;
    }

    mutating func player2Scores() {
        self.player2 = self.player2 + 1;
    }

    func render(renderer: Renderer) {
        let text = "\(self.player1) - \(self.player2)";
        renderer.drawText(text, 350, 20, 4);
    }

    func format() -> String {
        "\(self.player1) - \(self.player2)"
    }
}

struct Message : Cloneable {
    var text: String
    var x: Int64
    var y: Int64
    var scale: Int64

    func render(renderer: Renderer) {
        renderer.drawText(self.text, self.x, self.y, self.scale);
    }

    func clone() -> Message {
        Message(text: self.text.clone(), x: self.x, y: self.y, scale: self.scale)
    }
}

struct InputState {
    var up: Bool
    var down: Bool

    init() {
        self.up = false;
        self.down = false;
    }
}

// --- Main Game ---

func main() -> Int32 {
    var app = SDLApp(title: "Pong", width: 800, height: 600);
    var ball = Ball.create();
    var paddle1 = Paddle.left();
    var paddle2 = Paddle.right();
    var score = Score();
    var input = InputState();
    var waiting = true;
    var running = true;

    let startMessage = Message(
        text: "PRESS [SPACE] TO START",
        x: 180,
        y: 200,
        scale: 3
    );

    while running {
        // Handle events
        while let .Some(event) = app.pollEvent() {
            match event {
                .Quit => { running = false },
                .KeyDown(key) => {
                    match key {
                        .W or .UpArrow => { input.up = true },
                        .S or .DownArrow => { input.down = true },
                        .Space => { waiting = false },
                        .Escape => { running = false },
                        _ => {}
                    }
                },
                .KeyUp(key) => {
                    match key {
                        .W or .UpArrow => { input.up = false },
                        .S or .DownArrow => { input.down = false },
                        _ => {}
                    }
                },
                _ => {}
            }
        }

        // Update paddle 1 based on input
        if input.up { paddle1.moveUp(); }
        if input.down { paddle1.moveDown(); }

        if not waiting {
            // AI for paddle 2
            if ball.position.x > 200.0 {
                paddle2.trackBall(ball);
            }

            // Update ball
            ball.update();

            // Check scoring
            if ball.position.x < 0.0 {
                score.player2Scores();
                ball.reset(direction: 1.0);
                waiting = true;
            } else if ball.position.x > 800.0 {
                score.player1Scores();
                ball.reset(direction: 0.0 - 1.0);
                waiting = true;
            }

            // Paddle collisions
            if ball.position.x < 30.0 and paddle1.containsY(ball.position.y) {
                ball.velocity.x = 12.0;
            }
            if ball.position.x > 760.0 and paddle2.containsY(ball.position.y) {
                ball.velocity.x = 0.0 - 12.0;
            }
        }

        // Render
        app.render { (renderer) in
            renderer.clear(Color.black());

            paddle1.render(renderer);
            paddle2.render(renderer);
            score.render(renderer);

            if waiting {
                startMessage.render(renderer);
            }

            ball.render(renderer);
        };

        app.delay(Milliseconds(16));
    }

    0
}
