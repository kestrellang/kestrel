module expressks.http

import std.collections.dictionary
import std.text.string
import std.json.json

public struct Request {
    public let method: String
    public let path: String
    public let headers: Dictionary[String, String]
    // TODO: Body support
    // public let body: String
}

public enum Response {
    case Html(String)
    case Json(String)
    case Text(String)
    case Empty
}

public func Html(body: String) -> Response {
    .Html(body)
}

public func Json(body: Dictionary[String, String]) -> Response {
    // Basic JSON serialization for string map
    // TODO: use full Json serialization
    var json = "{"
    var first = true
    for (key, value) in body {
        if !first {
            json = json + ","
        }
        first = false
        json = json + "\"" + key + "\":\"" + value + "\""
    }
    json = json + "}"
    .Json(json)
}

public func Text(body: String) -> Response {
    .Text(body)
}
