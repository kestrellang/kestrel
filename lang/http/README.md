# HTTP

Shared HTTP types for Kestrel. Provides foundational types used by both client (swoop) and server (perch) libraries.

## Installation

```toml
[dependencies]
kestrel/http = "0.1.0"
```

## Key Types

### HttpMethod

Enum representing HTTP methods: Get, Post, Put, Delete, Patch, Head, Options.

```kestrel
let method = HttpMethod.Post
let _ = println(method.toString())  // "POST"
let hasBody = method.hasBody()      // true
```

### ParsedUrl

URL parsing with path segments and query string extraction.

```kestrel
let url = parseUrl("/users/42?page=1&limit=10")
// url.path == "/users/42"
// url.segments == ["users", "42"]
// url.queryString == "page=1&limit=10"

let params = parseQueryString(url.queryString)
// [("page", "1"), ("limit", "10")]
```

### Headers

Case-insensitive HTTP header collection.

```kestrel
var headers = Headers()
headers.add(name: "Content-Type", value: "application/json")
let ct = headers.value(forName: "content-type")  // "application/json"
```

### StatusCode

HTTP status codes with helper constructors and category checks.

```kestrel
let status = StatusCode.ok()          // 200
let notFound = StatusCode.notFound()  // 404
let _ = println(status.text())        // "OK"
let _ = println(status.isSuccess())   // true
```
