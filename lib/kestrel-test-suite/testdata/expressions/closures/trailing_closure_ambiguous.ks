// test: diagnostics
// stdlib: true

// A trailing closure that could fill more than one closure-typed parameter
// is ambiguous (E113): pass it inside the parentheses with its label.
module Test

import std.numeric.(Int64)

func pick(onA a: () -> Int64 = { 1 }, onB b: () -> Int64 = { 2 }) -> Int64 { a() + b() }

struct S {
    func pick(onA a: () -> Int64 = { 1 }, onB b: () -> Int64 = { 2 }) -> Int64 { a() + b() }
}

func f() {
    let a = pick { 5 }; // ERROR(E113): ambiguous trailing closure: it could fill `onA:` or `onB:`
    let b = S().pick { 5 }; // ERROR(E113): ambiguous trailing closure: it could fill `onA:` or `onB:`
    let c = pick(onB: { 5 });
    let d = pick(onA: { 5 }) { 6 };
}
