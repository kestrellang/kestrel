// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `lazy` property's cache is hidden state: derived `==` ignores it (and
// never reads the lazy property), and a struct pattern neither names it
// nor needs `..` for it. An observed property is plain storage, so it is
// compared.
module Test

import std.numeric.(Int64)

@derive(Equatable)
struct Doc {
    var words: Int64
    var hits: Int64 {
        didSet { }
    }
    lazy var doubled: Int64 = self.words * 2
}

@main
func main() -> lang.i64 {
    var a = Doc(words: 3, hits: 0);
    let b = Doc(words: 3, hits: 0);
    if a.doubled != 6 { return 1; }
    // `a` has filled its cache and `b` hasn't; they are still equal.
    if a != b { return 2; }
    let c = Doc(words: 3, hits: 1);
    if a == c { return 3; }

    let n = match b { Doc { words, hits } => words + hits };
    if n != 3 { return 4; }
    0
}
