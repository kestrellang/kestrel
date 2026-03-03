# Swoop

HTTP client library for Kestrel with a fluent, immutable API.

## Installation

```toml
[dependencies]
kestrel/swoop = "0.1.0"
```

Requires OpenSSL 3.x for HTTPS support. On macOS: `brew install openssl@3`

## Usage

```kestrel
// Simple GET request
let client = Swoop()
let response = try client.fetch("http://example.com/api/users")
let _ = println(response.body)

// Configure a reusable client
let api = Swoop()
    .baseUrl("https://api.example.com")
    .header("Authorization", "Bearer my-token")
    .header("Accept", "application/json")

let users = try api.fetch("/users")
let user = try api.fetch("/users/42")

// POST with a body
let result = try api.post("/users", Body.Text("{\"name\":\"Alice\"}"))
```

## Key Types

- **Swoop** - immutable HTTP client (each config method returns a new instance)
- **Response** - HTTP response with body, status code, and headers
- **Body** - request body (Text, Bytes, Form)

## HTTP Methods

- `fetch(url)` - GET
- `post(url, body)` - POST
- `put(url, body)` - PUT
- `patch(url, body)` - PATCH
- `delete(url)` - DELETE
- `head(url)` - HEAD

## Features

- Immutable, fluent configuration
- Automatic TLS for `https://` URLs
- Base URL with path resolution
- Custom headers and timeouts
