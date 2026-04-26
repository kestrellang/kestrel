// test: diagnostics
// stdlib: false

module Main

func grade(score: lang.i64) -> lang.str {
    match score {
        0..=59 => "F",
        60..=69 => "D",
        70..=79 => "C",
        80..=89 => "B",
        90..=100 => "A",
        _ => "invalid"
    }
}
