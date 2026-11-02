// test: execution
// stdlib: true
// expect-stdout: built\n3\n3\n
// backends: cranelift,llvm

// `modifying ref` fills a cache through a mutable receiver, then lends the
// cached value as a shared `&T` — no copy on later reads.
module Test

import std.numeric.(Int64)
import std.collections.(Array)
import std.io.stdio.(println)

struct Doc {
    var words: Array[Int64]
    var built: Bool

    var index: Array[Int64] {
        modifying ref {
            if not self.built {
                let _ = println("built");
                self.words.append(3);
                self.built = true;
            }
            self.words
        }
    }
}

@main
func main() -> lang.i64 {
    var d = Doc(words: [1, 2], built: false);
    let _ = println("\(d.index.count)");
    let _ = println("\(d.index.count)");
    0
}
