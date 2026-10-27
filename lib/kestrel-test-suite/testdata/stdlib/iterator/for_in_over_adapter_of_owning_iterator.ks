// test: execution
// stdlib: true
// backends: cranelift,llvm
// expect-exit: 0

// Regression: `for x in arr.iter().enumerate()` — looping over an adapter
// whose inner iterator owns something (`ArrayIterator` holds a refcounted
// share of the array's storage) — failed to build with a post-mono ICE.
//
// The `for` loop calls `Iterable.iter()`, which every `Iterator` gets from the
// blanket `extend Iterator: Iterable`. That body was `func iter() -> Self
// { self }`: a BORROWING method returning `self` by value, i.e. a copy. For a
// `not Copyable` iterator the copy could only be an alias — two owners of one
// storage share, released twice. It is now `consuming` (moves, like Rust's
// `impl<I: Iterator> IntoIterator for I`).
//
// The deinit counter checks no element is released early or twice.

module Test

var drops: Int64 = 0

struct Tracked {
    var id: Int64
    deinit { drops += 1; }
}

func sumEnumerated(items: Array[Tracked]) -> Int64 {
    var total = 0;
    for (index, item) in items.iter().enumerate() {
        total += index * 100 + item.id;
    }
    total
}

@main
func main() -> lang.i64 {
    // Array of a heap-owning type (String).
    var words = Array[String]();
    words.append("ab");
    words.append("cde");
    var wordTotal = 0;
    for (index, word) in words.iter().enumerate() {
        wordTotal += index * 10 + word.count;
    }
    if wordTotal != 15 { return 1 }

    // Bytes view of a string.
    let text = "hi!";
    var byteTotal = 0;
    for (offset, b) in text.bytes.iter().enumerate() {
        byteTotal += offset + Int64(b);
    }
    // 'h' 104 + 'i' 105 + '!' 33 + offsets 0+1+2
    if byteTotal != 245 { return 2 }

    // Elements with a deinit: the array must stay intact after the loop.
    do {
        var items = Array[Tracked]();
        items.append(Tracked(id: 1));
        items.append(Tracked(id: 2));
        items.append(Tracked(id: 3));
        if sumEnumerated(items: items) != 306 { return 3 }
        if drops != 0 { return 4 }
        if items.count != 3 { return 5 }
        if items(unchecked: 2).id != 3 { return 6 }
    }
    if drops != 3 { return 7 }

    0
}
