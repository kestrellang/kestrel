# Swoop

HTTP client library for Kestrel with a fluent, immutable API.

## Installation

```toml
[dependencies]
kestrel/swoop = "0.2.1"
```

Requires OpenSSL 3.x for HTTPS support. On macOS: `brew install openssl@3`

## Usage

```kestrel
// Simple GET request
let response = try Swoop().fetch("http://example.com/api/users")
let _ = println(response.body)

// Configure a reusable client
let api = Swoop(baseUrl: "https://api.example.com")
    .header("Authorization", "Bearer my-token")
    .header("Accept", "application/json")

let users = try api.fetch("/users")
let user = try api.fetch("/users/42")

// POST with JSON content
let result = try api.post("/users", JsonBody("{\"name\":\"Alice\"}"))

// POST with form data
let result = try api.post("/login", Form([("username", "alice"), ("password", "secret")]))
```

## Key Types

- **Swoop** - immutable HTTP client (each config method returns a new instance)
- **Response** - HTTP response with body, status code, and headers
- **Content** - protocol for request body types
  - **Text** - plain text content
  - **Bytes** - raw binary content
  - **Form** - URL-encoded form data (auto-sets Content-Type)
  - **JsonBody** - JSON content (auto-sets Content-Type)

## HTTP Methods

Instance methods on a configured client:
- `fetch(url)` - GET
- `post(url, content)` - POST
- `put(url, content)` - PUT
- `patch(url, content)` - PATCH
- `delete(url)` - DELETE
- `head(url)` - HEAD

## Features

- Immutable, fluent configuration
- Automatic TLS for `https://` URLs
- Base URL with path resolution
- Custom headers and timeouts
- Extensible content types via `Content` protocol
