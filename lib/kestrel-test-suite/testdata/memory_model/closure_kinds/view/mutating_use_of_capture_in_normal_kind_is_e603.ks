// test: diagnostics
// stdlib: true

// A normal closure's captures are read-only views, so every way of writing
// one is E603, not only `=`: compound assignment, a `mutating` method, a
// `mutating` argument, and a write inside a nested `mutating` closure (it
// writes through the outer view). These used to compile and write straight
// through to the caller's variables. A nested `escaping` closure writes its
// own copy, so it is fine.
module Test

struct Counter { var n: Int64 }

func bump(mutating n: Int64) { n = n + 1; }
func runNormal(f: () -> ()) { f(); }
func runMutating(mutating f: mutating () -> ()) { f(); }
func keep(f: escaping () -> ()) { f(); }

func normalClosureWrites() {
    var a: Int64 = 0;
    var items = [1, 2];
    var b: Int64 = 0;
    var c = Counter(n: 0);
    var d: Int64 = 0;
    var e: Int64 = 0;
    var g: Int64 = 0;
    runNormal({
        a += 1; // ERROR(E603): cannot mutate captured variable 'a'
        items.append(3); // ERROR(E603): cannot mutate captured variable 'items'
        bump(b); // ERROR(E603): cannot mutate captured variable 'b'
        c.n += 1; // ERROR(E603): cannot mutate captured variable 'c'
        d = 1; // ERROR(E603): cannot assign to captured variable 'd'
        runMutating({ e += 1; }); // ERROR(E603): cannot mutate captured variable 'e'
        keep({ g += 1; });
    });
}

// The writable kinds stay legal.
func writableKinds() {
    var a: Int64 = 0;
    var items = [1, 2];
    var b: Int64 = 0;
    runMutating({ a += 1; items.append(3); bump(b); });
    keep({ a += 1; items.append(4); });
}
