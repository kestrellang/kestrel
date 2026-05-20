module notes.helpers

import quill.value.(Value)
import quill.deserialize.(Deserialize)
import quill.error.(DeserializeError)
import quill.json.parser.(parseJson)
import perch.json_body.(JsonBody)
import perch.response.(Response)

import notes.time.(getCurrentTimestamp)

// Gets the current UTC timestamp in ISO8601 format using C FFI.
public func currentTimestamp() -> String {
    getCurrentTimestamp()
}

// Wraps an error message in a JSON object: {"error": "..."}
public func errorJson(message: String) -> Value {
    var obj = Dictionary[String, Value]();
    obj.insert("error", Value.Str(message));
    Value.Obj(obj)
}

// Parses a JSON request body into a Deserialize-conforming type.
public func parseBody[T](body: String) -> Result[T, Response] where T: Deserialize {
    let value = match parseJson(body) {
        .Ok(v) => v,
        .Err(_) => return .Err(Response.badRequest(JsonBody(fromRaw: errorJson("Invalid JSON"))))
    };
    match T.fromValue(value) {
        .Ok(parsed) => .Ok(parsed),
        .Err(e) => .Err(Response.badRequest(JsonBody(fromRaw: errorJson(e.description()))))
    }
}

// Extracts the authenticated userId from the request store.
public func requireUserId(store: Dictionary[String, String]) -> Int64? {
    guard let .Some(id) = store("userId") else { return .None }
    Int64(parsing: id)
}

// Parses a path parameter as Int64.
public func requireIdParam(value: String?) -> Int64? {
    guard let .Some(id) = value else { return .None }
    Int64(parsing: id)
}

// Builds a paginated JSON response envelope.
public func paginatedJson(data: Value, page: Int64, perPage: Int64, total: Int64) -> Value {
    var obj = Dictionary[String, Value]();
    obj.insert("data", data);
    obj.insert("page", Value.Int(page));
    obj.insert("perPage", Value.Int(perPage));
    obj.insert("total", Value.Int(total));
    Value.Obj(obj)
}

// Parses pagination query params. Defaults: page=1, perPage=25, max 100.
public func parsePagination(pageParam: String?, perPageParam: String?) -> (Int64, Int64) {
    let page = match pageParam {
        .Some(p) => match Int64(parsing: p) {
            .Some(n) => if n > 0 { n } else { 1 },
            .None => 1
        },
        .None => 1
    };
    let perPage = match perPageParam {
        .Some(p) => match Int64(parsing: p) {
            .Some(n) => if n > 0 and n <= 100 { n } else { 25 },
            .None => 25
        },
        .None => 25
    };
    (page, perPage)
}

