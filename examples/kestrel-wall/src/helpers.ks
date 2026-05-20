module wall.helpers

import http.url.(percentDecode)
import perch.request.(Request)

public func escapeHtml(s: String) -> String {
    var out = String();
    for c in s.chars.iter() {
        match c {
            '&' => out.append("&amp;"),
            '<' => out.append("&lt;"),
            '>' => out.append("&gt;"),
            '"' => out.append("&quot;"),
            '\'' => out.append("&#39;"),
            _ => out.append(char: c),
        }
    };
    out
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
    };
    result
}

public func formField(fields: Dictionary[String, String], key: String) -> String {
    match fields(key) {
        .Some(v) => v,
        .None => ""
    }
}

public func getClientIp(req: Request) -> String {
    if let .Some(cfIp) = req.header("CF-Connecting-IP") {
        return cfIp
    };
    if let .Some(xff) = req.header("X-Forwarded-For") {
        for part in xff.split(",") {
            return part.trimmed().toOwned()
        }
    };
    "unknown"
}
