// test: execution
// stdlib: true
// expect-stdout: Some(10)\nNone\nSome(26)\n2\nSome(other)\nkey\nSome(15)\n

// `d(key)` is assignable: its setter takes `Optional[V]`. A value inserts
// or replaces, `null` removes (a missing key is a no-op), and reads and
// `&mutating` updates in place work as before.

module Test

@main
func main() {
    var d = ["a": 1, "b": 2];
    d("z") = 26;
    d("a") = 10;
    d("b") = null;
    d("missing") = null;
    println(d("a"));
    println(d("b"));
    println(d("z"));
    println(d.count);
    var names = Dictionary[String, String]();
    let k = "key".toOwned();
    names(k) = "value";
    names("key") = "other";
    println(names("key"));
    println(k);
    if let .Some(&mutating v) = d("a") { v += 5; }
    println(d("a"));
}
