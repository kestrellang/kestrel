# Organization

Kestrel programs are made of **modules**. A module is a unit of code with its own namespace; modules are made of files; files declare which module they belong to and import names from other modules.

## Modules

Every `.ks` file starts with a `module` declaration:

```swift
module Game.Player

// ...declarations live here
```

The module name is dotted to suggest hierarchy — `Game.Player` is conceptually inside `Game`. Hierarchy here is naming convention; modules are otherwise flat. A program can have many files in one module; all declarations across those files share the namespace.

A common shape for a small project:

```
src/
  main.ks       — module Main
  game.ks       — module Game
  player.ks     — module Game.Player
  enemy.ks      — module Game.Enemy
```

## Visibility

Three levels:

```swift
public func openToEveryone() {}
internal func sameModuleOnly() {}   // default
private func sameFileOnly() {}
```

`internal` is the default — if you don't write a modifier, that's what you get. `public` is opt-in: anything you want callable from another module needs `public` on it. `private` restricts to the file it's declared in, useful for helpers and implementation details.

The same modifiers apply to types, fields, methods, and protocol requirements. A `public func` on an `internal struct` is still only reachable from the module — visibility is the *minimum* of the chain.

## Imports

`import` brings names from another module into scope:

```swift
import std.io.stdio.println
import std.collections.Dict
```

You can import a specific name (as above) or a whole module:

```swift
import std.collections   // brings the module in; reference as Dict, Array, ...
```

The Kestrel standard library auto-imports its most-used names — `Int`, `String`, `Bool`, `Optional`, `Result`, `Array`, etc. You shouldn't need to write `import std.num.Int` manually; if a basic name resolves, it's because the prelude already imported it.

Import only what you use. Wildcards aren't supported; if you want every name from a module, import the module itself and use the prefix.

## A complete example

```swift
// game/player.ks
module Game.Player

import std.io.stdio.println

public struct Player {
    public let name: String
    public var hp: Int
}

public extend Player {
    public mutating func takeDamage(amount: Int) {
        self.hp = self.hp - amount
        println("\(self.name) takes \(amount) damage")
    }
}
```

```swift
// main.ks
module Main

import Game.Player.Player

func main() -> Int {
    var p = Player(name: "Morgana", hp: 100)
    p.takeDamage(amount: 25)
    0
}
```

---

[← Extending Types](extending-types.md) · [↑ The Kestrel Language](index.md) · [FFI →](ffi.md)
