// test: execution
// stdlib: true
// expect-exit: 0

// Cloning a closure-holding view shares (retains) the escaping environment
// rather than bit-copying it, and the captures inside an acyclic environment
// are dropped exactly once at the last release (docs/design/closures.md,
// "escaping: a shared, stateful object" + "Copy and Drop").
module Test

public var drops: Int64 = 0;

struct Marker: not Copyable {
    var ch: Char

    // Method call widens the capture to the whole `Marker`, so the escaping
    // environment owns a droppable value.
    func isSeparator(c: Char) -> Bool { c == self.ch }

    deinit { drops = drops + 1; }
}

func run() -> Int64 {
    let marker = Marker(ch: ' ');
    let text: String = "one two three";

    // `split(where:)` stores the predicate: `marker` moves into the shared env.
    let view = text.split(where: { (c) in marker.isSeparator(c) });

    // Clone = share the same environment (reference semantics), not a deep copy.
    let alias = view.clone();

    // Both handles iterate correctly and independently.
    if view.count != 3 { return 1 }
    if alias.count != 3 { return 2 }

    let a = view.collect();
    let b = alias.collect();
    if a.count != 3 { return 3 }
    if b.count != 3 { return 4 }
    if a(unchecked: 0).toOwned().isEqual(to: "one") == false { return 5 }
    if a(unchecked: 2).toOwned().isEqual(to: "three") == false { return 6 }
    if b(unchecked: 0).toOwned().isEqual(to: "one") == false { return 7 }
    if b(unchecked: 2).toOwned().isEqual(to: "three") == false { return 8 }

    // Nothing has been released yet: both handles are still live.
    if drops != 0 { return 9 }

    0
}

@main
func main() -> lang.i64 {
    let rc = run();
    if rc != 0 { return 1 }

    // Both handles released -> environment destroyed once -> one `deinit`.
    if drops != 1 { return 2 }

    0
}
