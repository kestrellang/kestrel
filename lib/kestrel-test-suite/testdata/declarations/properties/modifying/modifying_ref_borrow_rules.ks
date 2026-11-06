// test: execution
// stdlib: true
// backends: cranelift,llvm

// A `modifying ref` result is a shared borrow of the receiver with the
// language's ratified may-alias semantics (references/bindings/
// binding_aliases_var.ks): other reads and writes of the receiver stay
// legal while it is live, and the binding observes them.
module Test

import std.numeric.(Int64)
import std.collections.(Array)

struct Doc {
    var words: Array[Int64]
    var built: Bool

    var index: Array[Int64] {
        modifying ref {
            self.built = true;
            self.words
        }
    }
}

@main
func main() -> lang.i64 {
    var d = Doc(words: [1, 2], built: false);
    let r = &d.index;
    if not d.built { return 1; }
    let n = d.words.count;
    d.words.append(3);
    let m = r.count;
    if n != 2 { return 2; }
    if m != 3 { return 3; }
    0
}
