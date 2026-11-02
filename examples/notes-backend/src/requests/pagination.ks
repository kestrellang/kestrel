module notes.requests

import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)

// `?page=&per_page=` query params. Missing or invalid values fall back to
// page 1, 25 per page; perPage is capped at 100.
public struct Pagination: Deserialize, Cloneable {
    public var page: Int64
    public var perPage: Int64

    public var offset: Int64 { (self.page - 1) * self.perPage }

    public static func fromValue(value: Value) -> Result[Pagination, DeserializeError] {
        let page = intParam(value, "page", default: 1, max: Int64.maxValue);
        let perPage = intParam(value, "per_page", default: 25, max: 100);
        .Ok(Pagination(page: page, perPage: perPage))
    }

    public func clone() -> Pagination {
        Pagination(page: self.page, perPage: self.perPage)
    }
}

// Query params arrive as strings; anything unparsable or out of range is the default.
fileprivate func intParam(value: Value, key: String, default fallback: Int64, max limit: Int64) -> Int64 {
    guard let some .Str(raw) = value.value(for: key) else { return fallback }
    guard let some n = Int64(parsing: raw) else { return fallback }
    if n > 0 and n <= limit { n } else { fallback }
}
