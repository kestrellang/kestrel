// Checks every match and capture against Python's re (fixtures.ks, made
// by make_fixtures.py), then the API, errors, Unicode case folding and
// inputs that take exponential time in backtracking engines.

module regex_tests

import regex.(Regex, RegexError, Match, Captures)

func check(name: String, ok: Bool) {
    if ok { print("PASS \(name)\n"); } else { print("FAIL \(name)\n"); }
}

/// All matches in the fixture format: "s-e,s-e ..." per match.
func describe(re: Regex, text: String) -> String {
    var parts: [String] = [];
    for c in re.allCaptures(in: text) {
        var spans: [String] = [];
        for g in 0..<c.count {
            match c.group(g) {
                .Some(m) => spans.append("\(m.start)-\(m.end)"),
                .None => spans.append("x")
            }
        }
        parts.append(join(spans, ","));
    }
    join(parts, " ")
}

func join(items: Array[String], separator: String) -> String {
    var out = "";
    var first = true;
    for item in items {
        if not first { out = out + separator; }
        out = out + item;
        first = false;
    }
    out
}

func compiled(pattern: String) -> Regex? {
    match Regex.compile(pattern) {
        .Ok(re) => .Some(re),
        .Err(e) => {
            check("compile \(pattern): \(e.description)", false);
            .None
        }
    }
}

func compileFails(pattern: String) -> Bool {
    match Regex.compile(pattern) {
        .Ok(_) => false,
        .Err(_) => true
    }
}

func testFixtures() {
    for c in cases() {
        guard let .Some(re) = compiled(c.pattern) else { continue; }
        let got = describe(re, c.text);
        if got == c.expected {
            check("/\(c.pattern)/ on \"\(c.text)\"", true);
        } else {
            check("/\(c.pattern)/ on \"\(c.text)\"\n  got  \(got)\n  want \(c.expected)", false);
        }
    }
}

func testApi() {
    guard let .Some(date) = compiled(#"(?<year>\d{4})-(?<month>\d{2})-(\d{2})"#) else { return; }
    check("captureCount", date.captureCount == 3);
    let names = date.captureNames;
    check("captureNames", names.count == 3 and names(0) == "year" and names(1) == "month" and names(2) == "");
    check("isMatch", date.isMatch(in: "due 2026-11-05!") and not date.isMatch(in: "due soon"));
    match date.captures(in: "due 2026-11-05!") {
        .Some(c) => {
            check("named group", (c.named("year")?.text ?? "") == "2026" and (c.named("month")?.text ?? "") == "11");
            check("numbered group", (c.group(3)?.text ?? "") == "05");
            check("whole match", c.whole.text == "2026-11-05" and c.whole.start == 4 and c.whole.end == 14);
            check("unknown name", c.named("day").isNone);
            check("group out of range", c.group(9).isNone);
        },
        .None => check("captures", false)
    }
    check("replace numbered and named", date.replacingAll(in: "2026-11-05 and 1999-01-02", with: "$3/${month}/$year") == "05/11/2026 and 02/01/1999");
    check("replace first only", date.replacingFirst(in: "2026-11-05 and 1999-01-02", with: "X") == "X and 1999-01-02");
    check("dollar escape", date.replacingAll(in: "2026-11-05", with: "$$1") == "$1");
    check("unknown group expands to nothing", date.replacingAll(in: "2026-11-05", with: "[$9${nope}]") == "[]");

    guard let .Some(comma) = compiled(#",\s*"#) else { return; }
    let parts = comma.split("a, b,c,,  d");
    check("split", parts.count == 5 and parts(0) == "a" and parts(1) == "b" and parts(3) == "" and parts(4) == "d");
    check("split without match", comma.split("abc").count == 1);

    guard let .Some(word) = compiled(#"\w+"#) else { return; }
    let shouted = word.replacingAll(in: "hi there", using: { (c) in c.whole.text.uppercased() });
    check("replace with closure", shouted == "HI THERE");
    let ms = word.matches(in: "one two  three");
    check("matches", ms.count == 3 and ms(2).text == "three" and ms(2).start == 9);
    check("firstMatch", (word.firstMatch(in: "  x1 y")?.text ?? "") == "x1");
    check("no match", word.firstMatch(in: "  !").isNone and word.matches(in: "").count == 0);

    let special = "a.b*c(d)[e]{f}|g^h$i+j?k\\l";
    match Regex.compile(Regex.escape(special)) {
        .Ok(literal) => check("escape", literal.isMatch(in: "x" + special + "y") and not literal.isMatch(in: "a.b*c")),
        .Err(_) => check("escape compiles", false)
    }
    guard let .Some(empty) = compiled("") else { return; }
    check("empty pattern matches everywhere", describe(empty, "ab") == "0-0 1-1 2-2");
    check("byte offsets with UTF-8", describe(word, "é ab") == "3-5");
}

func testUnicodeCase() {
    guard let .Some(greek) = compiled("(?i)σοφία") else { return; }
    check("Greek case-insensitive", greek.isMatch(in: "ΣΟΦΊΑ"));
    guard let .Some(cafe) = compiled("(?i)CAFÉ") else { return; }
    check("Latin-1 case-insensitive", cafe.isMatch(in: "café"));
    guard let .Some(cls) = compiled("(?i)[à-ä]+") else { return; }
    check("class folds", (cls.firstMatch(in: "xÀÄáy")?.text ?? "") == "ÀÄá");
    guard let .Some(posix) = compiled("[[:alpha:]]+[[:digit:]]+[[:^space:]]") else { return; }
    check("POSIX classes", (posix.firstMatch(in: "  abc123! ")?.text ?? "") == "abc123!");
    guard let .Some(hex) = compiled(#"\x{1F600}"#) else { return; }
    check("hex escape beyond BMP", hex.isMatch(in: "hi 😀"));
    guard let .Some(dot) = compiled("^.$") else { return; }
    check("dot is one code point", dot.isMatch(in: "😀") and not dot.isMatch(in: "ab"));
}

func testErrors() {
    let bad = ["a(b", "a)b", "[ab", "*a", "a**", "a{3,2}", "a{1001}", "(?<>a)", "(?<n>a)(?<n>b)", "\\", "\\q", "(?z)", "\\x{110000}", "[z-a]", "[[:nope:]]", "(a{1000}){1000}", "^*"];
    for pattern in bad {
        check("rejects \(pattern)", compileFails(pattern));
    }
    match Regex.compile("ab(cd") {
        .Ok(_) => check("error position", false),
        .Err(e) => check("error position", e.position == 2 and e.message == "unclosed group")
    }
}

/// Inputs that take exponential time in backtracking engines finish
/// immediately here.
func testLinearTime() {
    var a = "";
    for _ in 0..<5000 { a = a + "a"; }
    guard let .Some(nested) = compiled("(a*)*b") else { return; }
    check("(a*)*b on 5000 a's", not nested.isMatch(in: a));
    guard let .Some(alt) = compiled("(a|aa)+c") else { return; }
    check("(a|aa)+c on 5000 a's", alt.firstMatch(in: a).isNone);
    guard let .Some(xs) = compiled("(x+x+)+y") else { return; }
    var x = "";
    for _ in 0..<2000 { x = x + "x"; }
    check("(x+x+)+y on 2000 x's", not xs.isMatch(in: x));
    guard let .Some(counted) = compiled("a{0,30}a{30}") else { return; }
    var thirty = "";
    for _ in 0..<30 { thirty = thirty + "a"; }
    check("a{0,30}a{30} on 30 a's", (counted.firstMatch(in: thirty)?.end ?? 0) == 30);
}

@main
func main() {
    testFixtures();
    testApi();
    testUnicodeCase();
    testErrors();
    testLinearTime();
}
