# `std.memory` — patterns

## Boxes that can hold an existential

`any[B] P` is `B[rawany P]`, and `rawany P` is unsized, not `Copyable`, and
only `Static` when `P` is. A box type meant to hold one (`RcBox`, `CowBox`,
`OwnedBox`, and anything new):

- declares its payload parameter `T: not Copyable, T: not Sized,
  T: not Static` (`CowBox` keeps `T: Cloneable` in place of `not Copyable`);
- holds `T` only behind a pointer, or as the last field of a heap storage
  struct (an unsized tail). Never as a field of the box itself;
- puts every member that takes or returns `T` by value behind
  `where T: Sized`. A generic method can't add a bound on the outer `T`
  (E437), so those go in an `extend Box[T] where T: Sized { … }`;
- measures and frees its block from the payload (`Layout.ofValue(at:)`,
  `lang.alignof_val`), not `Layout.of[T]()`, wherever `T` may be unsized;
- conforms to `ExistentialBox` (`init(consuming value:)` may stay
  `where T: Sized`).

E402 checks the parameter, conformance and field rules at every `any[B]`;
E403 checks by-value uses inside the box.

Generic helpers the boxes call (`Layout.of`, `Pointer.cast`, …) must relax
`Static` (and `Sized`, where they don't need a size) too, or a box over a
non-`Static` payload fails to type-check.

## `UniqueBox` is not a general box

`UniqueBox` is the compiler-internal handle for `consuming` closures. It has
no `deinit` on purpose: dropping the handle must not free the block. For a
user-facing single-owner box, use `OwnedBox`.
