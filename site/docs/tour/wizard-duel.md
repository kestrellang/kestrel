# Tour: Wizard Duel

Two wizards take turns trading spells. By the end of this tour you'll have built a working duel and seen Kestrel's most distinctive feature — recursive enums with pattern matching — earn its keep.

Each step adds one new idea. Copy, run, then move on.

## Step 1 — Values & functions

```swift
module Main
import std.io.stdio.println

func cast(spell: String, at target: String, for damage: Int) {
    println("\(spell) hits \(target) for \(damage) damage!");
}

func main() -> Int {
    cast("Fireball", at: "Morgana", for: 8);
    0
}
```

`cast` takes three parameters. `spell: String` is positional — `spell` is the bind name, no label, so the call site doesn't write one. The next two have labels (`at target`, `for damage`), so the call site uses them: `cast("Fireball", at: "Morgana", for: 8)`. String interpolation uses `\(...)`.

## Step 2 — Structs & methods

```swift
struct Wizard {
    let name: String
    var hp: Int
}

extend Wizard {
    mutating func takeDamage(amount: Int) {
        self.hp = self.hp - amount;
    }

    func isAlive() -> Bool {
        self.hp > 0
    }
}
```

`let` fields can't change after init; `var` fields can. `mutating func` is required to write to a `var` field — the caller has to pass the wizard via a `var` binding. Methods live in `extend` blocks rather than inside the struct definition.

`takeDamage` takes a positional `amount: Int`, so callers write `wiz.takeDamage(5)`.

## Step 3 — Enums & pattern matching

This is the moment Kestrel earns its keep.

```swift
indirect enum Spell {
    case Fireball(damage: Int)
    case Heal(amount: Int)
    case Counterspell
    case Combo(Spell, Spell)
}

func describe(spell: Spell) -> String {
    match spell {
        .Fireball(damage) => "Fireball (\(damage) dmg)",
        .Heal(amount) => "Heal (\(amount) hp)",
        .Counterspell => "Counterspell",
        .Combo(a, b) => "\(describe(a)) + \(describe(b))"
    }
}
```

`Combo` holds *two more spells* — that's a recursive enum, and it's why the `indirect` keyword is required. `match` destructures payloads inline; each arm produces a value, and the compiler checks every case is handled.

## Step 4 — Collections

Each wizard now has a deck.

```swift
struct Wizard {
    let name: String
    var hp: Int
    var deck: [Spell]
}

func resolve(spell: Spell, by mutating attacker: Wizard, on mutating defender: Wizard) {
    match spell {
        .Fireball(damage) => defender.takeDamage(damage),
        .Heal(amount) => attacker.hp = attacker.hp + amount,
        .Counterspell => println("\(defender.name) is silenced!"),
        .Combo(a, b) => {
            resolve(a, by: attacker, on: defender);
            resolve(b, by: attacker, on: defender);
        }
    }
}
```

`[Spell]` is array-of-`Spell`. `spell: Spell` is positional; `by mutating attacker` and `on mutating defender` are labeled with `mutating` access mode. Each recursive call hands the same wizards down, with mutations visible across the whole chain — that's why `Combo` accumulates effects correctly.

## Step 5 — Protocols

Spells aren't the only thing wizards can `cast`. A potion should work too. Abstracting over "things that can be cast" is what protocols are for.

```swift
protocol Castable {
    func describe() -> String
    mutating func apply(to mutating target: Wizard)
}

extend Spell: Castable {
    public func describe() -> String { /* same as before */ }

    public mutating func apply(to mutating target: Wizard) {
        match self {
            .Fireball(damage) => target.takeDamage(damage),
            .Heal(amount) => target.hp = target.hp + amount,
            .Counterspell => {},
            .Combo(a, b) => {
                a.apply(to: target);
                b.apply(to: target);
            }
        }
    }
}
```

A type "conforms" to a protocol with `: ProtocolName`. Now `apply(to:)` works on any `Castable`, and adding `Potion: Castable` later costs nothing in the rest of the program.

## What you saw

| Step | Feature |
|---|---|
| 1 | Functions, positional vs labeled parameters, string interpolation |
| 2 | Structs, `let`/`var` fields, `mutating` methods |
| 3 | Enums with payloads, **`indirect` recursive enums**, `match` |
| 4 | Arrays, `mutating` parameters, recursion across mutated state |
| 5 | Protocols and conformance |

The recursive `Combo` variant is the takeaway. That same shape — an algebraic type that holds itself — is how Kestrel models expressions, JSON, ASTs, and decision trees, all with exhaustively-checked pattern matching for free.

---

[← Text Adventure](text-adventure.md) · [↑ A Tour of Kestrel](index.md) · [Turtle Graphics →](turtle-graphics.md)
