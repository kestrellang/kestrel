// test: execution
// stdlib: true

module Test

func findValue(pairs: Array[(String, String)], name: String) -> String? {
    var i: Int64 = 0;
    while i < pairs.count {
        let pair = pairs(unchecked: i);
        if pair.0.isEqual(to: name) {
            return .Some(pair.1)
        }
        i = i + 1
    }
    .None
}

func main() -> lang.i64 {
    var headers = Array[(String, String)]();
    headers.append(("Content-Type", "text/html"));
    headers.append(("Host", "example.com"));
    headers.append(("Accept", "application/json"));

    let ct = findValue(headers, "Content-Type");
    match ct {
        .Some(v) => if v.isEqual(to: "text/html") == false { return 1 },
        .None => return 2
    }

    let host = findValue(headers, "Host");
    match host {
        .Some(v) => if v.isEqual(to: "example.com") == false { return 3 },
        .None => return 4
    }

    let missing = findValue(headers, "X-Missing");
    match missing {
        .Some(_) => return 5,
        .None => 0
    }

    0
}
