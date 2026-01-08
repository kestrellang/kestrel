# Standard Library Issues

Catalog of issues found when building `lang/std/` files.

## Categories

- **Not Supported Yet** - Features that need to be implemented in the compiler
- **Wrong Syntax** - Stdlib uses incorrect syntax; needs to be fixed in the stdlib files
- **Implement** - New feature that needs both design and implementation

---

## 1. Computed Properties
**Status: Not Supported Yet**

```kestrel
// Current (not supported)
public static var zero: Int64 { Int64(value: 0) }
var description: String { get }
```

Files affected: All numeric types (`int64.ks`, `int32.ks`, etc.), `protocols.ks`, `serde.ks`

---

## 2. Type Parameter Constraints with Colon
**Status: Wrong Syntax** - Use `where` clauses instead

```kestrel
// Current (wrong)
public struct Buffer[T, A: Allocator] { }
public func hash[H: Hasher](into hasher: ref H) { }

// Correct
public struct Buffer[T, A] where A: Allocator { }
public func hash[H](into hasher: ref H) where H: Hasher { }
```

Files affected: `Buffer`, `Array`, `String`, `Dictionary`, `Set`, `Bool.hash`, all generic types with constraints

---

## 3. Default Type Arguments
**Status: Not Supported Yet**

```kestrel
// Current (not supported)
public struct Array[T, A: Allocator = GlobalAllocator] { }
```

Files affected: `Buffer`, `Array`, `String`, `Dictionary`, `Set`

---

## 4. Public Import / Re-export
**Status: Not Supported Yet**

```kestrel
// Current (not supported)
public import std.core.ordering.(Ordering)
```

Files affected: `std.ks`

---

## 5. Operator Protocol Attributes
**Status: Implement + Not Supported Yet**

Will use `@builtin` attributes instead of `@operator`:

```kestrel
// Current (wrong)
@operator(+)
public protocol Addable { }

// Planned
@builtin(.AddOperatorProtocol)
public protocol Addable { }

@builtin(.AddOperatorMethod)
public func add(other: Self) -> Output
```

Files affected: All operator protocols in `ops/` directory

---

## 6. Protocol Methods Without Body
**Status: Wrong Syntax** - Protocol methods should NOT have a body

```kestrel
// Current (wrong - has body in protocol)
protocol Iterator {
    func next() -> Optional[Item] { }  // Wrong - has empty body
}

// Correct
protocol Iterator {
    func next() -> Optional[Item]  // No body, just declaration
}
```

Files affected: `iterator.ks`, `error.ks`, protocols

---

## 7. Enum Case with Unnamed Associated Values
**Status: Not Supported Yet** - WORKAROUND APPLIED

```kestrel
// Not supported yet
case Some(T)

// Workaround: use labeled form (with TODO comment)
case Some(value: T)
```

Files affected: `optional.ks`, `result.ks`, `error.ks`

**Workaround applied**: Added parameter names with TODO comments to remove when feature is implemented.

---

## 8. Extension Declarations
**Status: Allowed** (already supported)

```kestrel
extension Equatable: Equal[Self], NotEqual[Self] { }
```

Note: The errors seen may be due to other issues in the file, not the extension itself.

---

## 9. Global Constants with Initialization
**Status: Allowed** (already supported)

```kestrel
public let nil: Nil = Nil()
```

Note: The error seen may be due to other issues in the file.

---

## 10. Self Assignment in Init
**Status: Allowed** (already supported)

```kestrel
init(alignment: Int) {
    self.alignment = alignment
}
```

Note: The errors seen are likely due to other parsing issues earlier in the file.

---

## 11. Bitwise Not Operator `~`
**Status: Wrong Syntax** - Use method call instead

```kestrel
// Current (wrong - ~ is not a token)
let mask = ~(layout.alignment - 1)

// Correct
let mask = (layout.alignment - 1).bitwiseNot()
```

Files affected: `allocator.ks`

---

## 12. $0 Shorthand for Closure Parameters
**Status: Wrong Syntax** - Use explicit parameter names

```kestrel
// Current (wrong)
result = result.map { combine($0, item) }

// Correct
result = result.map { |x| combine(x, item) }
```

Files affected: `extensions.ks`

---

## 13. `null` as Function Name
**Status: Wrong Syntax** - `null` is reserved, use different name

```kestrel
// Current (wrong - null is reserved)
public static func null() -> RawPointer { }

// Correct
public static func nilPointer() -> RawPointer { }
// or
public static func zero() -> RawPointer { }
```

Files affected: `pointer.ks`

---

## 14. @throws Attribute with Type
**Status: Wrong Syntax** - Replace with `Result` return type

```kestrel
// Current (wrong)
@throws(defaultError: any Error)
func parse() -> Json { }

// Correct
func parse() -> Result[Json, Error] { }
```

Files affected: `result.ks`, potentially others

---

## 15. Protocol Conformance on Type Declaration
**Status: Allowed** (already supported)

```kestrel
public struct CodePoint: Equatable, Comparable, Hashable { }
```

Note: The errors seen are likely due to other parsing issues earlier in the file.

---

## Summary

### To Implement in Compiler
1. Computed Properties
3. Default Type Arguments
4. Public Import / Re-export
5. `@builtin` attributes for operators
7. Enum Case with Unnamed Associated Values (workaround applied in stdlib)
16. `self.init()` Delegation (see #16 below)

### Additional Compiler Features Discovered During Testing
- **Associated types in structs** - `type Item = T` inside structs
- **Protocol method declarations** - `func next() -> T` without body in protocols
- **where clauses on functions/inits** - `where A == GlobalAllocator` 
- **ref parameters with generic constraints** - `(into hasher: ref H) where H: Hasher`
- **Extensions on protocols** - `extension Iterator { ... }`

### To Fix in Stdlib (Wrong Syntax) - COMPLETED
2. Type Parameter Constraints - use `where` clauses - DONE
6. Protocol Methods - changed to method syntax - DONE  
11. Bitwise Not - use `.bitwiseNot()` method - DONE
12. $0 Shorthand - use explicit closure parameters - DONE
13. `null` function name - renamed to `nilPointer` - DONE
14. @throws - removed attribute - DONE

### Already Supported (False Positives)
8. Extension Declarations
9. Global Constants with Initialization
10. Self Assignment in Init
15. Protocol Conformance on Type Declaration

---

## 16. `self.init()` Delegation
**Status: Not Supported Yet**

Calling another initializer from within an initializer.

```kestrel
// Current (not supported)
public init(arrayLiteral elements: [T]) {
    self.init(capacity: elements.count)  // Error: found 'Init' expected something else
    // ...
}

// Workaround: duplicate initialization logic (not ideal)
public init(arrayLiteral elements: [T]) {
    self.storage = ArcBox(value: ArrayStorage(
        buffer: Buffer(capacity: elements.count),
        count: 0
    ))
    // ...
}
```

Files affected: `collections/array.ks` (lines 53, 61)
