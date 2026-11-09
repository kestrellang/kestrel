// test: execution
// stdlib: true
// expect-stdout: PT1.5S\n1.5s\n2m 3s\nPT6S\nPT2.5S\n10\n-1\n-500\ntrue\n1500\n

// std.time.Duration is visible without an import: units, arithmetic and
// both print forms. Accessors truncate toward zero (-1.5s = -1s, -500ms).

module Test

@main
func main() {
    let d = Duration.seconds(1) + Duration.milliseconds(500);
    println(d);
    println(d.humanString());
    println((Duration.minutes(2) + .seconds(3)).humanString());
    println(Duration.seconds(2) * 3);
    println(Duration.seconds(10) / 4);
    println(Duration.minutes(5) / Duration.seconds(30));
    let neg = Duration.milliseconds(-1500);
    println(neg.totalSeconds);
    println(neg.subsecondNanoseconds / 1_000_000);
    println(neg.isNegative);
    println(neg.abs().totalMilliseconds);
}
