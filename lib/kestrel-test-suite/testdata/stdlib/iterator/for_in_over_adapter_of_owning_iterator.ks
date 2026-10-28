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
// An early or double release of the shared storage frees the elements'
// heap buffers; the second pass over the same array then reads freed memory,
// so it runs every loop twice and checks the contents after each.
//
// (A deinit counter cannot measure this: the element type would have to be
// Copyable to live in an Array, and a Copyable type's deinit runs once per
// copy — drop-semantics.md.)

module Test

func sumEnumerated(items: Array[String]) -> Int64 {
    var total = 0;
    for (index, item) in items.iter().enumerate() {
        total += index * 100 + item.bytes.count;
    }
    total
}

// The array lives in its own scope, so it is released when this returns.
func checkTwice() -> Int64 {
    var items = Array[String]();
    items.append("one");
    items.append("three");
    items.append("seventeen");
    // 0*100+3 + 1*100+5 + 2*100+9
    if sumEnumerated(items) != 317 { return 3 }
    if sumEnumerated(items) != 317 { return 4 }
    if items.count != 3 { return 5 }
    if items(unchecked: 2) != "seventeen" { return 6 }
    0
}

@main
func main() -> Int64 {
    // Array of a heap-owning type (String).
    var words = Array[String]();
    words.append("ab");
    words.append("cde");
    var wordTotal = 0;
    for (index, word) in words.iter().enumerate() {
        wordTotal += index * 10 + word.bytes.count;
    }
    if wordTotal != 15 { return 1 }

    // Bytes view of a string.
    let text = "hi!";
    var byteTotal = 0;
    for (offset, b) in text.bytes.iter().enumerate() {
        byteTotal += offset + Int64(from: b);
    }
    // 'h' 104 + 'i' 105 + '!' 33 + offsets 0+1+2
    if byteTotal != 245 { return 2 }

    // The array must stay intact across repeated loops.
    let intact = checkTwice();
    if intact != 0 { return intact }

    0
}
