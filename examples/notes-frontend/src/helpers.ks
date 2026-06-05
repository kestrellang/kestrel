module notes.helpers

import http.cookie.(parseCookieHeader)
import http.url.(percentDecode)
import perch.request.(Request)

public func getToken(req: Request) -> String {
    guard let .Some(cookieHeader) = req.header("Cookie") else { return "" }
    let pairs = parseCookieHeader(cookieHeader);
    var i: Int64 = 0;
    while i < pairs.count {
        let (name, value) = pairs(unchecked: i);
        if name == "token" {
            return value
        };
        i = i + 1
    };
    ""
}

public func parseForm(body: String) -> Dictionary[String, String] {
    var result = Dictionary[String, String]();
    for part in body.split("&") {
        let kv = part.trimmed();
        match kv.firstIndex(of: "=") {
            .Some(eqIdx) => {
                let key = percentDecode(kv.subslice(from: kv.start, to: eqIdx.value).toOwned());
                let value = percentDecode(kv.subslice(from: eqIdx.value + 1, to: kv.end).toOwned());
                result.insert(key, value);
            },
            .None => {}
        }
    }
    result
}

public func formField(fields: Dictionary[String, String], key: String) -> String {
    match fields(key) {
        .Some(v) => v,
        .None => ""
    }
}
