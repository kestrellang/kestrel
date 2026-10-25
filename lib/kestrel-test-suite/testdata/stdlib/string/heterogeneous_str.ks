// test: execution
// stdlib: true

module Test

import std.text.(Str)

@main
func main() -> lang.i64 {
    let owned: String = "alpha,beta";
    let whole = owned.asSlice();
    let alpha = whole.subslice(from: 0, to: 5);
    let comma = whole.subslice(from: 5, to: 6);
    let beta = whole.subslice(from: 6, to: 10);

    if owned != whole { return 1 }
    if whole != owned { return 2 }
    if owned == alpha { return 3 }
    if alpha == owned { return 4 }

    if owned.asSlice().contains(beta) == false { return 5 }
    if owned.starts(with: alpha) == false { return 6 }
    if owned.ends(with: beta) == false { return 7 }

    let parts = owned.asSlice().split(comma).collect();
    if parts.count != 2 { return 8 }
    let alphaOwned: String = "alpha";
    let betaOwned: String = "beta";
    if parts(unchecked: 0) != alphaOwned { return 9 }
    if parts(unchecked: 1) != betaOwned { return 10 }

    let replaced: String = owned.asSlice().replaced(alpha, with: beta);
    if replaced.isEqual(to: "beta,beta") == false { return 11 }

    0
}
