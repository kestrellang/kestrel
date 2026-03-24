// test: diagnostics
// stdlib: false
module Test

protocol Add[Right] {
    type Output;
    func add(right: Right) -> Output
}
struct Int: Add[lang.i64] {
    type Add[lang.i64].Output = lang.i64;
    func add(right: lang.i64) -> lang.i64 { 0 }
}
