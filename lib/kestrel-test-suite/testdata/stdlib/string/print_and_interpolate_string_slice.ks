// test: execution
// stdlib: true
// expect-stdout: hello world\n[hello world]\nkey=value\n

module Test

// Formatting only reads its value, so a borrowed `StringSlice` prints and
// interpolates directly, without `toOwned()`.
@main
func main() {
    let s = "  hello world  ";
    let t = s.trimmed();
    println(t);
    print("[\(t)]\n");
    for line in "key=value\n".lines {
        println(line);
    }
}
