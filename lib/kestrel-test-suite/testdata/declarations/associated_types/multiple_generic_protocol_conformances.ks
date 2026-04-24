// test: diagnostics
// stdlib: false
module Test

protocol Add[Right] {
    type Output;
}
struct Int: Add[lang.i64], Add[lang.f64] {
    type Add[lang.i64].Output = lang.i64;
    type Add[lang.f64].Output = lang.f64;
}
