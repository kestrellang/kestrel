# Ownership: copy, clone, move

Every value has one owner. When the owner goes out of scope the value is
dropped, and its `deinit` runs, if it has one. How a value travels depends on
what kind of type it is:

| Kind | Examples | `let b = a` | Passing to a param |
|---|---|---|---|
| `Copyable` | numbers, `Bool`, structs of copyable fields | bit copy, both usable | borrowed (default) |
| `Cloneable` | `String`, `Array`, `Dictionary`, structs holding them | the compiler inserts `clone()`, both usable | borrowed (default) |
| `not Copyable` | file handles, sockets, your RAII types | **move**: `a` is dead afterwards (E500 on use) | borrowed (default), or moved with `consuming` |

Parameters borrow unless they say otherwise: `x: T` reads, `mutating x: T`
writes through to the caller's `var`, and `consuming x: T` takes ownership.
A borrowed argument costs nothing, even for a `String`.

## Worked example

```kestrel
module Ownership

struct Point { var x: Int64; var y: Int64 }

@derive(Cloneable)
struct Profile { var name: String; var tags: Array[String] }

struct Handle: not Copyable {
    var fd: Int64
    deinit { println("close \(self.fd)"); }
}

struct Pair: not Copyable { var a: Handle; var b: Handle }

func inspect(h: Handle) -> Int64 { h.fd }
func close(consuming h: Handle) { }
func open(fd: Int64) -> Handle { Handle(fd: fd) }

@main
func main() -> Int64 {
    let p = Point(x: 1, y: 2);
    var q = p;                    // copy
    q.x = 9;
    let a = Profile(name: "ana", tags: ["x"]);
    var b = a;                    // clone inserted: b owns its own buffers
    b.tags.append("y");
    println("\(p.x) \(a.tags.count) \(b.tags.count)");
    let h = open(3);
    println(inspect(h));          // borrow: h still usable
    close(h);                     // move: deinit runs at the end of close
    let pair = Pair(a: open(4), b: open(5));
    let first = pair.a;           // partial move
    println(first.fd);
    0
}
```

## Rules

- **Copyability comes from the fields.** A struct is `Copyable` when every
  field is. One `String` field makes it `Cloneable`, which needs a `clone()`:
  write `@derive(Cloneable)` or conform by hand.
- **`not Copyable` opts out** and makes the type move-only. Give it a
  `deinit` for cleanup. `deinit` runs exactly once, when the last owner drops.
- **`consuming func` on a method** takes `self`. The receiver is dead afterwards.
  Use it for `close()`, `build()`, `finish()`.
- **Partial moves:** you can move one field out of a move-only struct
  (`let first = pair.a`). The rest is dropped at scope end. After a partial
  move you can't use the whole value again.
- **You can't move out of a borrow** (E503). If a function only borrows `w`,
  it can't return `w.handle`. Take the param `consuming`, return a reference
  (`-> &Handle`), or clone.
- **A live closure freezes what it views** (E507). Finish with the closure
  before moving a captured value.

## Writing `clone()` by hand

Inside `clone()`, `self` is a bitwise copy: the compiler doesn't insert
clones there, or `clone()` would call itself forever. So
`func clone() -> T { self }` shares the heap buffers, and both copies free
them (a double free). Clone each heap field:

```kestrel
enum Token: Cloneable {
    case Plain
    case Literal(String)
    func clone() -> Token {
        match self { .Literal(s) => .Literal(s.clone()), _ => self }
    }
}
```

Returning `self` is fine only for cases with no heap payload, like `.Plain`
above. Prefer `@derive(Cloneable)`, which gets this right.

## Generic code

Type parameters are `Copyable` by default, so `func f[T](x: T)` won't accept a
move-only `T`. Write `where T: not Copyable` and avoid copying `x` in the
body: pass it on borrowed, or move it with `consuming`.
