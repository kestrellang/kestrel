// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0
//
// A stored var witnessing a `var { get set }` requirement gets a synthesized
// setter. Setter call sites pass the new value BORROWED; the synthesized
// setter used to declare it `consuming` and store it without a copy, so a
// heap value (String) was owned twice — the caller's drop and the field's
// drop freed the same buffer (crash). It now copies the borrowed value in.
// Covers instance, static, generic-conformer and hand-written setters.

module Test

protocol Named {
    var name: String { get set }
}

protocol Labeled {
    static var label: String { get set }
}

struct Plain: Named {
    var name: String;
}

struct Box[T]: Named {
    var name: String;
    var payload: T;
}

struct Computed: Named {
    var stored: String;
    var name: String {
        get { self.stored }
        set { self.stored = newValue; }
    }
}

struct Global: Labeled {
    static var label: String = "start";
}

func rename[T](t: T, to n: String) -> String where T: Named {
    var u = t;
    u.name = n;
    u.name
}

func relabel[T](kind: T, to n: String) -> String where T: Labeled {
    T.label = n;
    T.label
}

@main
func main() -> Int64 {
    var total = 0;
    for i in 0..<200 {
        let v = "new-name-\(i)";
        total += rename(Plain(name: "old-\(i)"), to: v).bytes.count;
        total += rename(Box(name: "box-\(i)", payload: i), to: v).bytes.count;
        total += rename(Computed(stored: "c-\(i)"), to: v).bytes.count;
        total += relabel(Global(), to: v).bytes.count;
        // The caller's value is intact after the setter stored a copy.
        if v != "new-name-\(i)" { return 1 }
    }
    // 4 setters × (10 × 10 + 90 × 11 + 100 × 12) bytes for "new-name-<i>".
    if total != 4 * (100 + 990 + 1200) { return 2 }
    if Global.label != "new-name-199" { return 3 }
    0
}
