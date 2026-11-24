// test: execution
// stdlib: true
// expect-exit: 0

// Control flow that stays inside the right side of `and` / `or` is fine:
// a `break` / `continue` aimed at a loop written inside the right side, and a
// `return` inside a closure written there (the closure's own return). Only
// jumps that would leave the right side are E012
// (see rhs_jumps_out_rejected.ks).

module Test

func apply(f: () -> Bool) -> Bool { f() }

func innerLoop(n: Int64) -> Bool {
    n < 0 or (if n > 0 {
        var i = 0;
        while true {
            i = i + 1;
            if i < n { continue }
            break
        }
        i == n
    } else {
        false
    })
}

func closureReturn(x: Int64) -> Bool {
    x > 10 or apply({ if x > 0 { return true } ; false })
}

@main
func main() -> Int64 {
    if not innerLoop(4) { return 1 }
    if innerLoop(0) { return 2 }
    if not closureReturn(3) { return 3 }
    if closureReturn(-3) { return 4 }
    0
}
