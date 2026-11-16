// test: diagnostics

// A statement keyword where a `let` initializer belongs is one mistake: the
// parser reports the missing expression and nothing else. It used to add a
// missing-`;` error and an inference error on the binding. A keyword in an
// error message is spelled out (`unsafe` used to read as 'token').

module Test

struct Handle: not Copyable { let id: Int64 }

func f() -> Int64 {
    let h = Handle(id: 1);
    let x = deinit h; // ERROR(E803): expected expression, found 'deinit'
    2
}

func g(a: Int64 unsafe) {} // ERROR(E800): expected `,` or `)`, found 'unsafe'
