// test: execution
// stdlib: true
// backends: cranelift,llvm
// skip: stage1 — needs ref returns end-to-end (S1+M6)

// The canonical safe ret_borrow: a borrowed-self accessor returning
// `&self.field` (Param root, fully checked). The caller reads via binding
// decay — `let a = p.age()` stores an owned copy.
module Test

struct Person {
    var age: Int64
    func ageRef() -> &Int64 { self.age }
}

@main
func main() -> lang.i64 {
    let p = Person(age: 42);
    let a = p.ageRef();
    if a != 42 { return 1; }
    0
}
