// test: execution
// stdlib: true

// Copy-drift #2-#4 convergence (2026-06-10): Array's conditional Copyable
// gating arg is a mixed tuple (Int64, String) — Copyable element + Cloneable
// element — which now folds to Cloneable in every layer (solver, MIR ty_query,
// mono refinement). Copying the array must clone-elaborate; both copies stay
// independently valid with correct heap-string elements. Previously the mono
// refinement classified the tuple move-only and the solver classified it
// plain Copyable, so the layers disagreed about this exact program.

module Test

@main
func main() -> lang.i64 {
    var a = Array[(Int64, String)]();
    a.append((1, "one"));
    a.append((2, "two"));

    let b = a;
    if a.count != 2 { return 1 }
    if b.count != 2 { return 2 }

    let first = a(unchecked: 0);
    if first.0 != 1 { return 3 }
    if first.1.isEqual(to: "one") == false { return 4 }

    let second = b(unchecked: 1);
    if second.0 != 2 { return 5 }
    if second.1.isEqual(to: "two") == false { return 6 }
    0
}
