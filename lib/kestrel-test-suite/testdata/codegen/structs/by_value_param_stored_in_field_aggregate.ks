// test: execution
// stdlib: true

// Regression: a by-value (hence *borrowed*) parameter stored into a struct
// field aggregate was packed into the @owned aggregate WITHOUT cloning, so the
// aggregate aliased the borrow. Once the caller dropped the original the stored
// element dangled (use-after-free / double-free on read).
//
// `add(name:, value:)` takes its params by value — which lowers to a BORROW —
// and does `self.entries.append((name, value))`. `emit_tuple` consumed the
// @guaranteed params as if @owned, building a tuple of aliases; `parse` then
// dropped the locals it had built, freeing the heap String buffers the tuple
// pointed at. Reading them back (here via `value(forName:)` iterating
// `self.entries`) crashed. Fix: aggregate construction (tuple/struct/enum)
// clones @guaranteed elements to @owned (`own_aggregate_element`).
//
// This is the exact shape of talon-sqlite's `Headers.parse` + `Headers.value`,
// which crashed kestrel-wall while parsing the first HTTP request. The strings
// must be heap-backed (built/derived at runtime) — string literals live in
// immortal static storage and mask the over-release.
module Test

func heap(a: std.text.String, b: std.text.String) -> std.text.String {
    var s = std.text.String();
    s.append(a);
    s.append(b);
    s
}

struct Bag: Cloneable {
    var entries: std.collections.Array[(std.text.String, std.text.String)]
    init() { self.entries = std.collections.Array[(std.text.String, std.text.String)](); }
    // by-value (borrowed) params escaping into a struct-field aggregate
    mutating func add(name: std.text.String, value: std.text.String) {
        self.entries.append((name, value));
    }
    func value(forName name: std.text.String) -> std.text.String? {
        for (key, v) in self.entries {
            if key.equalsCaseInsensitive(name) { return .Some(v); }
        };
        .None
    }
    func clone() -> Bag { var b = Bag(); b.entries = self.entries.clone(); b }
    // Built and RETURNED from a function (so the locals it built are dropped).
    static func build() -> Bag {
        var bag = Bag();
        bag.add(heap("Content", "-Length"), heap("4", "2"));
        bag.add(heap("Conn", "ection"), heap("keep", "-alive"));
        bag
    }
}

func main() -> lang.i64 {
    let bag = Bag.build();
    match bag.value(forName: "content-length") {
        some v => { if not v.isEqual(to: "42") { return 1; } },
        null => return 2
    };
    match bag.value(forName: "connection") {
        some v => { if not v.isEqual(to: "keep-alive") { return 3; } },
        null => return 4
    };
    0
}
