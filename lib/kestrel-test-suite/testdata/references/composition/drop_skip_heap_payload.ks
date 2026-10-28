// test: execution
// backends: cranelift,llvm
// stdlib: true

// References 2b drop elaboration: a ref payload slot is NEVER dropped,
// while its owned sibling IS — heap strings (not literals) so a
// double-free corrupts malloc and a leak is at least observable.
module Test

enum Pair {
    case P(&String, String)
    case Empty
}

@main
func main() {
    var src = "source-\(1)";
    let r = &src;
    let owned = "owned-\(2)";
    let e: Pair = .P(r, owned);
    match e {
        .P(a, b) => {
            if a != "source-1" {
                fatalError("ref payload read broke: \(a)");
            }
            if b != "owned-2" {
                fatalError("owned payload read broke: \(b)");
            }
        },
        .Empty => fatalError("P matched as Empty")
    }
    // src must still be intact after `e` (and its owned payload) drop:
    // a double-free of the REF slot would have freed src's storage.
    if src != "source-1" {
        fatalError("ref slot was dropped with the aggregate");
    }
    print("ok");
}

// CHECK: ok
