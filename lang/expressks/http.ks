// HTTP types for ExpressKS
//
// This module provides HTTP method, status, request, and response types.

module expressks.http;

import std.collections.dictionary;
import std.text.string;
import std.serde.Serialize;
import std.json.Json;

// HTTP Method enum
public enum HttpMethod: Equatable {
    case Get
    case Post
    case Put
    case Delete
    case Patch
    case Head
    case Options

    // Parse method from string
    public static func parse(str: String) -> Optional[HttpMethod] {
        let upper = str.uppercase();
        if upper == "GET" {
            .Some(.Get)
        } else if upper == "POST" {
            .Some(.Post)
        } else if upper == "PUT" {
            .Some(.Put)
        } else if upper == "DELETE" {
            .Some(.Delete)
        } else if upper == "PATCH" {
            .Some(.Patch)
        } else if upper == "HEAD" {
            .Some(.Head)
        } else if upper == "OPTIONS" {
            .Some(.Options)
        } else {
            .None
        }
    }

    // Convert to string
    public func toString() -> String {
        match self {
            .Get => "GET",
            .Post => "POST",
            .Put => "PUT",
            .Delete => "DELETE",
            .Patch => "PATCH",
            .Head => "HEAD",
            .Options => "OPTIONS"
        }
    }

    public func equals(other: HttpMethod) -> Bool {
        match (self, other) {
            (.Get, .Get) => true,
            (.Post, .Post) => true,
            (.Put, .Put) => true,
            (.Delete, .Delete) => true,
            (.Patch, .Patch) => true,
            (.Head, .Head) => true,
            (.Options, .Options) => true,
            _ => false
        }
    }
}

// HTTP Status code
public struct HttpStatus: Equatable {
    public let code: Int;
    public let message: String;

    public init(code: Int, message: String) {
        self.code = code;
        self.message = message;
    }

    // Common status codes
    public static let ok = HttpStatus(code: 200, message: "OK");
    public static let created = HttpStatus(code: 201, message: "Created");
    public static let accepted = HttpStatus(code: 202, message: "Accepted");
    public static let noContent = HttpStatus(code: 204, message: "No Content");

    public static let movedPermanently = HttpStatus(code: 301, message: "Moved Permanently");
    public static let found = HttpStatus(code: 302, message: "Found");
    public static let seeOther = HttpStatus(code: 303, message: "See Other");
    public static let notModified = HttpStatus(code: 304, message: "Not Modified");
    public static let temporaryRedirect = HttpStatus(code: 307, message: "Temporary Redirect");

    public static let badRequest = HttpStatus(code: 400, message: "Bad Request");
    public static let unauthorized = HttpStatus(code: 401, message: "Unauthorized");
    public static let forbidden = HttpStatus(code: 403, message: "Forbidden");
    public static let notFound = HttpStatus(code: 404, message: "Not Found");
    public static let methodNotAllowed = HttpStatus(code: 405, message: "Method Not Allowed");
    public static let conflict = HttpStatus(code: 409, message: "Conflict");
    public static let gone = HttpStatus(code: 410, message: "Gone");
    public static let unprocessableEntity = HttpStatus(code: 422, message: "Unprocessable Entity");
    public static let tooManyRequests = HttpStatus(code: 429, message: "Too Many Requests");

    public static let internalServerError = HttpStatus(code: 500, message: "Internal Server Error");
    public static let notImplemented = HttpStatus(code: 501, message: "Not Implemented");
    public static let badGateway = HttpStatus(code: 502, message: "Bad Gateway");
    public static let serviceUnavailable = HttpStatus(code: 503, message: "Service Unavailable");

    public func equals(other: HttpStatus) -> Bool {
        self.code == other.code
    }
}

// HTTP Request
public struct Request {
    public let method: HttpMethod;
    public let path: String;
    public let headersMap: Dictionary[String, String];
    public var paramsMap: Dictionary[String, String];   // Path parameters (:id)
    public let queryMap: Dictionary[String, String];    // Query string (?key=value)
    public let body: String;
    public var contextMap: Dictionary[String, String];  // Middleware context

    public init(
        method: HttpMethod,
        path: String,
        headers: Dictionary[String, String],
        params: Dictionary[String, String],
        query: Dictionary[String, String],
        body: String
    ) {
        self.method = method;
        self.path = path;
        self.headersMap = headers;
        self.paramsMap = params;
        self.queryMap = query;
        self.body = body;
        self.contextMap = Dictionary[String, String]();
    }

    // Get a path parameter by name
    public func params(key: String) -> Optional[String] {
        self.paramsMap.get(key)
    }

    // Get a query parameter by name
    public func query(key: String) -> Optional[String] {
        self.queryMap.get(key)
    }

    // Get a header by name (case-insensitive)
    public func header(name: String) -> Optional[String] {
        self.headersMap.get(name.lowercase())
    }

    // Get context value by key
    public func context(key: String) -> Optional[String] {
        self.contextMap.get(key)
    }

    // Set context value
    public func setContext(key: String, value: String) {
        self.contextMap.insert(value: value, for: key);
    }

    // Get content type
    public func contentType() -> Optional[String] {
        self.header(name: "content-type")
    }

    // Check if request accepts JSON
    public func acceptsJson() -> Bool {
        match self.header(name: "accept") {
            .Some(let accept) => accept.contains(substring: "application/json"),
            .None => false
        }
    }
}

// HTTP Response
public enum Response {
    case Html(status: HttpStatus, body: String)
    case Json(status: HttpStatus, body: String)
    case Text(status: HttpStatus, body: String)
    case Empty(status: HttpStatus)
    case Redirect(to: String, status: HttpStatus)

    // Get the status code
    public func status() -> HttpStatus {
        match self {
            .Html(let s, _) => s,
            .Json(let s, _) => s,
            .Text(let s, _) => s,
            .Empty(let s) => s,
            .Redirect(_, let s) => s
        }
    }

    // Get content type
    public func contentType() -> String {
        match self {
            .Html(_, _) => "text/html; charset=utf-8",
            .Json(_, _) => "application/json",
            .Text(_, _) => "text/plain; charset=utf-8",
            .Empty(_) => "",
            .Redirect(_, _) => ""
        }
    }

    // Get body
    public func body() -> String {
        match self {
            .Html(_, let b) => b,
            .Json(_, let b) => b,
            .Text(_, let b) => b,
            .Empty(_) => "",
            .Redirect(_, _) => ""
        }
    }
}

// Response builder functions

// Create an HTML response with 200 OK
public func Html(body: String) -> Response {
    .Html(status: HttpStatus.ok, body: body)
}

// Create an HTML response with custom status
public func Html(status: HttpStatus, body: String) -> Response {
    .Html(status: status, body: body)
}

// Create a JSON response from any Serialize type with 200 OK
public func Json[T: Serialize](value: T) -> Response {
    match std.json.Json.serialize(value: value) {
        .Ok(let body) => .Json(status: HttpStatus.ok, body: body),
        .Err(_) => .Json(status: HttpStatus.internalServerError, body: "{\"error\":\"serialization failed\"}")
    }
}

// Create a JSON response with custom status
public func Json[T: Serialize](status: HttpStatus, value: T) -> Response {
    match std.json.Json.serialize(value: value) {
        .Ok(let body) => .Json(status: status, body: body),
        .Err(_) => .Json(status: HttpStatus.internalServerError, body: "{\"error\":\"serialization failed\"}")
    }
}

// Create a JSON response from raw string
public func JsonRaw(body: String) -> Response {
    .Json(status: HttpStatus.ok, body: body)
}

// Create a JSON response from raw string with custom status
public func JsonRaw(status: HttpStatus, body: String) -> Response {
    .Json(status: status, body: body)
}

// Create a text response with 200 OK
public func Text(body: String) -> Response {
    .Text(status: HttpStatus.ok, body: body)
}

// Create a text response with custom status
public func Text(status: HttpStatus, body: String) -> Response {
    .Text(status: status, body: body)
}

// Create a redirect response (302 Found)
public func Redirect(to url: String) -> Response {
    .Redirect(to: url, status: HttpStatus.found)
}

// Create a redirect response with custom status
public func Redirect(to url: String, permanent: Bool) -> Response {
    let status = if permanent {
        HttpStatus.movedPermanently
    } else {
        HttpStatus.found
    };
    .Redirect(to: url, status: status)
}

// Create a 404 Not Found response
public func NotFound() -> Response {
    .Text(status: HttpStatus.notFound, body: "Not Found")
}

// Create a 404 Not Found response with custom message
public func NotFound(message: String) -> Response {
    .Text(status: HttpStatus.notFound, body: message)
}

// Create a 400 Bad Request response
public func BadRequest(message: String) -> Response {
    .Text(status: HttpStatus.badRequest, body: message)
}

// Create a 401 Unauthorized response
public func Unauthorized(message: String) -> Response {
    .Text(status: HttpStatus.unauthorized, body: message)
}

// Create a 403 Forbidden response
public func Forbidden(message: String) -> Response {
    .Text(status: HttpStatus.forbidden, body: message)
}

// Create a 500 Internal Server Error response
public func InternalError(message: String) -> Response {
    .Text(status: HttpStatus.internalServerError, body: message)
}

// Create an empty response with custom status
public func Empty(status: HttpStatus) -> Response {
    .Empty(status: status)
}

// Create an empty 204 No Content response
public func NoContent() -> Response {
    .Empty(status: HttpStatus.noContent)
}
