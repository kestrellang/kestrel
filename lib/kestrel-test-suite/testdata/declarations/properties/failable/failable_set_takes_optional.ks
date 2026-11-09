// test: execution
// stdlib: true
// expect-stdout: Some(5)\n[7]\n[]\n[set 5, set 7, clear]\nSome(8)\n

// A failable member may declare `set`; its `newValue` is `Optional[T]`, so
// the setter decides what assigning `null` means. Plain `=` calls it, even
// when a `mutating ref` is also declared (that one serves `&mutating`
// binders, which need the place to exist).

module Test

struct Box {
    var items: Array[Int64]
    var log: Array[String]

    var first?: Int64 {
        get {
            if self.items.isEmpty { return null; }
            .Some(self.items(0))
        }
        set {
            match newValue {
                .Some(v) => {
                    if self.items.isEmpty { self.items.append(v) } else { self.items(0) = v }
                    self.log.append("set \(v)");
                },
                .None => {
                    if not self.items.isEmpty { let _ = self.items.remove(at: 0); }
                    self.log.append("clear");
                },
            }
        }
    }

    subscript(slot i: Int64)? -> Int64 {
        ref {
            if i < 0 or i >= self.items.count { return null; }
            let r = &self.items(i);
            .Some(r)
        }
        mutating ref {
            if i < 0 or i >= self.items.count { return null; }
            let r = &mutating self.items(i);
            .Some(r)
        }
        set {
            if let .Some(v) = newValue {
                if i >= 0 and i < self.items.count { self.items(i) = v; }
            }
        }
    }
}

@main
func main() {
    var b = Box(items: [], log: []);
    b.first = 5;
    println(b.first);
    b.first = 7;
    println(b.items);
    b.first = null;
    println(b.items);
    println(b.log);
    b.items.append(1);
    b(slot: 0) = 3;
    if let .Some(&mutating v) = b(slot: 0) { v += 5; }
    println(b(slot: 0));
}
