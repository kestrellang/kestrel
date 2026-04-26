# Tour: Text Adventure

Build a tiny text adventure across 5 steps. By the end you'll have a 4-room dungeon you can walk around in. Showpieces: dictionaries, `Optional`, and how Kestrel handles things that might not be there.

## Step 1 — Values & functions

```swift
module Main
import std.io.stdio.println

func describe(location: String) {
    println("You stand in \(location).");
}

func main() -> Int {
    describe("a damp cave");
    0
}
```

A function and a positional call. `location` is the bind name with no label, so the call site is just `describe("a damp cave")`. Run it, see the output. That's the loop you'll repeat for every step.

## Step 2 — Structs & methods

```swift
struct Room {
    let name: String
    let description: String
    var exits: [String]
}

extend Room {
    func describe() {
        println("\(self.name): \(self.description)");
        println("Exits: \(self.exits)");
    }
}
```

`let` fields are fixed at construction; `var` lets you change them later (we'll need that when the player picks up an item). The implicit memberwise initializer means you can write `Room(name: "Cave", description: "...", exits: ["north"])` without writing an `init`.

## Step 3 — Enums & pattern matching

The player needs to do things — go places, look around, take items, quit. That's an enum with payloads.

```swift
enum Action {
    case Go(direction: String)
    case Look
    case Take(item: String)
    case Quit
}

func handle(action: Action, in room: Room) {
    match action {
        .Go(direction) => println("You head \(direction)."),
        .Look => room.describe(),
        .Take(item) => println("You pick up the \(item)."),
        .Quit => println("Goodbye.")
    }
}
```

Every payload variant unpacks inline in the `match`. Exhaustiveness means the compiler refuses to compile if you forget a case — add a new `Action`, every `match` over it lights up red until you handle the new variant.

## Step 4 — Collections

A real adventure has more than one room. We'll keep them in a dictionary keyed by name.

```swift
import std.collections.Dict

func play(rooms: Dict[String, Room], start: String) {
    var current = start;

    let lookup = rooms[current];
    if let .Some(room) = lookup {
        room.describe();
    } else {
        println("You wandered off the map.");
    }
}
```

`rooms[current]` returns an `Optional[Room]` — the key might not exist. The `if let .Some(room) = ...` shape unwraps it; the `else` branch handles the missing case. Optional is just an enum (`.Some(T)` and `.None`), but the compiler treats unwrap patterns specially so you can't forget to handle absence.

## Step 5 — Protocols

Right now `describe` only works on `Room`. But the dungeon has shrines and vaults too. Abstracting over "places you can describe" is what protocols are for.

```swift
protocol Place {
    func name() -> String
    func describe()
}

extend Room: Place {
    public func name() -> String { self.name }
    public func describe() { /* as before */ }
}

struct Shrine {
    let deity: String
    let blessing: String
}

extend Shrine: Place {
    public func name() -> String { "Shrine of \(self.deity)" }
    public func describe() {
        println("A shrine to \(self.deity). It offers: \(self.blessing).");
    }
}
```

Now any function that takes a `Place` works on either type. Add a `Vault` later, conform it to `Place`, and every existing function picks it up for free.

## What you saw

| Step | Feature |
|---|---|
| 1 | Functions, positional parameters, `println` |
| 2 | Structs, methods via `extend`, memberwise init |
| 3 | Enums with payloads, exhaustive `match` |
| 4 | **Dictionaries**, **`Optional`** and `if let` unwrap |
| 5 | Protocols, multiple types conforming to one contract |

The `Optional` story is the takeaway. There is no `null`, no `nil`, no `undefined` — anything that might be absent is an `Optional[T]` that the compiler forces you to unwrap. Whole categories of bug just don't exist.

---

[← A Tour of Kestrel](index.md) · [↑ A Tour of Kestrel](index.md) · [Wizard Duel →](wizard-duel.md)
