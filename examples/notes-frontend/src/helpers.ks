module notes.helpers

import http.url.(percentDecode)
import perch.(Request, HttpError, FromRequest)

/// Parses an `application/x-www-form-urlencoded` body into its fields.
public func parseForm(body: String) -> Dictionary[String, String] {
    var result = Dictionary[String, String]();
    for part in body.split(by: "&") {
        let kv = part.trimmed();
        match kv.firstIndex(of: "=") {
            .Some(eqIdx) => {
                let key = percentDecode(kv.subslice(from: 0, to: eqIdx.value).toOwned());
                let value = percentDecode(kv.subslice(from: eqIdx.value + 1, to: kv.bytes.count).toOwned());
                result.insert(key, value);
            },
            .None => {}
        }
    }
    result
}

/// A form-encoded request body, as a handler input: perch has no form
/// extractor, so the app supplies one through `FromRequest`.
public struct Form {
    var fields: Dictionary[String, String]

    /// The field named `key`, or "" when the form doesn't have it.
    public func field(key: String) -> String = self.fields(key) ?? ""

    /// The field named `key` as a positive id; `.None` when absent, not a
    /// number, or 0 (the forms' "no folder" choice).
    public func positiveId(key: String) -> Int64? {
        guard let some n = Int64(parsing: self.field(key)) else { return .None }
        if n > 0 { .Some(n) } else { .None }
    }
}

extend Form: FromRequest {
    public static func fromRequest(request: Request) -> Form throws HttpError =
        Form(fields: parseForm(request.body))
}
