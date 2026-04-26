# Tour: Turtle Graphics

Drive a turtle around an ASCII grid. Each step adds one new idea, and step 5 prints a drawing. Showpieces: closures and **executing data as a program**.

## Step 1 — Values & functions

```swift
module Main
import std.io.stdio.println

func report(at x: Int, y: Int, facing heading: Int) {
    println("turtle at (\(x), \(y)) facing \(heading)°");
}

func main() -> Int {
    report(at: 0, 0, facing: 0);
    0
}
```

`at x` and `facing heading` are labeled (`at` and `facing` are the labels; `x` and `heading` are the bind names). `y: Int` between them is positional — bare name, no label — so the call site is `report(at: 0, 0, facing: 0)`.

## Step 2 — Structs & methods

```swift
struct Turtle {
    var x: Int
    var y: Int
    var heading: Int
}

extend Turtle {
    mutating func forward(distance: Int) {
        match self.heading {
            0   => self.x = self.x + distance,
            90  => self.y = self.y + distance,
            180 => self.x = self.x - distance,
            270 => self.y = self.y - distance,
            _   => {}
        }
    }

    mutating func turn(degrees: Int) {
        self.heading = (self.heading + degrees) % 360;
    }
}
```

Both methods take a positional `Int`. Calls look like `t.forward(10)` and `t.turn(90)`. `var` fields plus `mutating func` and the turtle has a body — note that `match` works on integers here, not just enums.

## Step 3 — Enums & pattern matching

A program for the turtle is a list of commands. `Repeat` is the showpiece: it holds a list of *more commands*, so the data structure is recursive.

```swift
indirect enum Command {
    case Move(distance: Int)
    case Turn(degrees: Int)
    case PenUp
    case PenDown
    case Repeat(count: Int, body: [Command])
}
```

`indirect` is the keyword that lets a variant hold the enum itself (or an array of it).

## Step 4 — Collections

Now an interpreter. Walk the array, execute each `Command`, recurse into `Repeat`.

```swift
func run(program: [Command], on mutating turtle: Turtle) {
    for command in program {
        match command {
            .Move(distance) => turtle.forward(distance),
            .Turn(degrees) => turtle.turn(degrees),
            .PenUp => {},
            .PenDown => {},
            .Repeat(count, body) => {
                var i = 0;
                while i < count {
                    run(body, on: turtle);
                    i = i + 1;
                }
            }
        }
    }
}
```

`run` takes `program: [Command]` positionally and `on mutating turtle: Turtle` with a label. Recursive calls `run(body, on: turtle)` thread the same `mutating` turtle through every nested invocation, so the drawing accumulates correctly.

## Step 5 — Protocols

A `Shape` is anything that produces a `[Command]`. With a protocol, you can define `Square`, `Triangle`, and `Spiral` as types — and feed them to `run` interchangeably.

```swift
protocol Shape {
    func commands() -> [Command]
}

struct Square {
    let size: Int
}

extend Square: Shape {
    public func commands() -> [Command] {
        [.Repeat(count: 4, body: [
            .Move(distance: self.size),
            .Turn(degrees: 90)
        ])]
    }
}

struct Spiral {
    let turns: Int
}

extend Spiral: Shape {
    public func commands() -> [Command] {
        var result: [Command] = [];
        var i = 1;
        while i <= self.turns {
            result.append(.Move(distance: i * 5));
            result.append(.Turn(degrees: 90));
            i = i + 1;
        }
        result
    }
}
```

A `Shape` produces commands; `run` consumes commands. The two halves never meet directly — the protocol decouples them. That's the same pattern you'll see throughout Kestrel libraries.

## What you saw

| Step | Feature |
|---|---|
| 1 | Functions, positional vs labeled parameters |
| 2 | `var` fields, `mutating func`, `match` on integers |
| 3 | Enums with payloads, **`indirect` recursive enums** |
| 4 | `for` loop, recursive interpreter, `mutating` parameters |
| 5 | Protocols, multiple conforming types |

The takeaway: **programs are data**. A `[Command]` is just a list, but the moment you walk it with `match`, it's executable. That same trick — represent behavior as data, interpret it later — is how Kestrel handles parsers, query DSLs, animation timelines, and most things that look like "small languages embedded in code."

---

[← Wizard Duel](wizard-duel.md) · [↑ A Tour of Kestrel](index.md) · [Values & Variables →](../values-and-variables.md)
