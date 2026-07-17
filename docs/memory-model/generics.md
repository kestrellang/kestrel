# Generics and Copy Semantics

Kestrel generics follow a "copy-by-default" rule to maintain ergonomics for application developers, with `not Copyable` bounds and per-instantiation conditional copyability for library code.

## The Default: Copyable

A generic type parameter `T` is bounded by `Copyable` by default:

```kestrel
func duplicate[T](item: T) -> (T, T) {
    (item, item)   // OK! T is Copyable by default
}

let p = Point(x: 1, y: 1);
duplicate(p);   // Works — Point is Copyable

let f = FileHandle(...);
duplicate(f);   // ERROR: FileHandle !: Copyable
```

The bound is enforced wherever the instantiation is formed — at generic calls, at type annotations, and at container construction:

```kestrel
struct Wrap[T] { var inner: T }          // implicit T: Copyable

func mk(r: FileHandle) -> Wrap[FileHandle] {  // ERROR: FileHandle !: Copyable
    Wrap(inner: r)                             // ERROR here too
}
```

If the argument is *Cloneable*, generic copies dispatch to its `clone()` at monomorphization; the body doesn't change.

## Relaxing the Bound: `not Copyable`

Code that must work with **any** type (including move-only resources) removes the default bound with `not Copyable`, either inline or in a `where` clause:

```kestrel
struct Slot[T: not Copyable] {
    var value: T
}

struct Slot2[T] where T: not Copyable {   // equivalent
    var value: T
}

func swap[T](mutating a: T, mutating b: T) where T: not Copyable { ... }
```

Inside such code, values of type `T` may only be borrowed or moved — never duplicated:

```kestrel
func process[T](consuming x: T) where T: not Copyable {
    let a = x;
    let b = x;   // ERROR(E500): use of moved value
}
```

A `not Copyable`-bounded generic accepts *both* copyable and non-copyable arguments — the bound removes a capability from the body, it does not restrict callers.

## Summary of Generic Bounds

| Syntax | Meaning | Body can copy `T`? |
|--------|---------|--------------------|
| `[T]` | implicit `T: Copyable` | Yes |
| `[T: Copyable]` / `where T: Copyable` | explicit | Yes |
| `[T: Cloneable]` / `where T: Cloneable` | requires `clone()` | Yes (via clone) |
| `[T: not Copyable]` / `where T: not Copyable` | no Copyable requirement | No |
| `where T: not Static` | `T` may contain references (see below) | — |

Bounds combine with ordinary protocol bounds: `func printAll[T](items: ...) where T: not Copyable, T: Printable`.

## Conditional Copyability

A generic container can be move-only *by default* and Copyable *when its arguments are*, via conditional conformance:

```kestrel
enum Box[T]: not Copyable {
    case Of(T)
}
extend Box[T]: Copyable where T: Copyable { }

let a: Box[Int64] = .Of(7);
let b = a;                    // copy — Box[Int64] is Copyable
let c = a;                    // still valid

let d: Box[FileHandle] = .Of(FileHandle(...));
let e = d;                    // MOVE — Box[FileHandle] is not Copyable
```

The per-instantiation rule (implemented once, in `kestrel-copy-fold`, and used identically by inference, the move checker, and monomorphization):

1. If the base type is unconditionally Copyable/Cloneable/NotCopyable, that wins.
2. A `not Copyable` base with a conditional `extend ...: Copyable where ...` folds the **gating type arguments**: NotCopyable dominates; else any Cloneable argument makes the instantiation Cloneable; else it is Copyable.

Note the Cloneable case is derived automatically: the conditional `extend` declares only `Copyable`, but `Box[String]` classifies as *Cloneable* (copying it clones the payload deeply), and it satisfies `Cloneable` bounds. Multiple gating parameters fold together — `MyResult[T, E]` with `extend MyResult[T, E]: Copyable where T: Copyable, E: Copyable` is Copyable only when both are.

This is exactly how the standard library declares its core containers:

```kestrel
// lang/std/result/optional.ks
public enum Optional[T]: not Copyable where T: not Static {
    case Some(T)
    case None
}
extend Optional[T]: Copyable where T: Copyable { }
```

Conditional method availability works the same way and is used extensively:

```kestrel
struct List[T: not Copyable] {
    mutating func push(item: consuming T) { ... }        // always available
    func cloned() -> List[T] where T: Cloneable { ... }  // only for cloneable T
}
```

## The `Static` Bound and References

Alongside `Copyable`, there is a second implicit-by-default marker: `Static` (`@builtin(.Static)`), meaning "contains no references". Type parameters require `Static` arguments by default; `where T: not Static` relaxes that so a reference can be a type argument (`Optional[&Int64]`, from the declaration above). Like copyability, staticness of a container instantiation follows its arguments. Globals and `static` members must be of Static types (E505) — references cannot hide in program-lifetime storage.

## Known Gap: Instantiation-Time Enforcement Is Incomplete

The `T: Copyable` default is enforced at annotation, call, and construction sites (see above), but **not yet on every inferred instantiation path**. Notably, an array *literal* of non-Copyable elements currently forms `Array[T]` without tripping the bound, and Copyable-default element reads then bit-copy the element:

```kestrel
let arr = [Res(id: 1), Res(id: 2)];   // Res: not Copyable — currently NOT rejected
let x = arr(0);                        // bit-copies the element → double-deinit
```

(The explicit form `Array[Res]()` *is* rejected.) Until instantiation-time enforcement lands, do not put non-Copyable values in `Array` — the supported containers for move-only elements are ones declared with `not Copyable` element bounds.

---

## Design Notes

- **Inverted mental model**: other languages add capability bounds (`T: Clone`); Kestrel removes the default (`T: not Copyable`). This matches the application-first philosophy — only library authors opt out.
- **Forgetting `not Copyable`** makes a container silently Copyable-only. The error appears at the user's instantiation site (`FileHandle !: Copyable`), pointing back at the implicit bound.
- **Variance**: Kestrel generics are invariant; copyability bounds do not introduce subtyping between instantiations.
