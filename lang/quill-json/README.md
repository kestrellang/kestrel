# Quill-JSON

JSON support for the quill serialization framework.

## Installation

```toml
[dependencies]
kestrel/quill = "0.1.0"
kestrel/quill-json = "0.1.0"
```

## Usage

```kestrel
// Encode a Value to JSON
let value = Value.Obj([
    ("name", Value.Str("Alice")),
    ("age", Value.Int(30))
])
let json = try Json().encode(value: value)
// {"name":"Alice","age":30}

// Decode JSON to a Value
let parsed = try Json().decode(source: "{\"x\": 1}")

// Convenience functions for Serialize types
let jsonStr = try toJson(value: mySerializableValue)
let pretty = try toJsonPretty(value: mySerializableValue)
```

## Key Types

- **Json** - implements the quill `Format` protocol for JSON
- Content type: `application/json`

## Functions

- `toJson(value:)` - serialize any `Serialize` type to compact JSON
- `toJsonPretty(value:)` - serialize to pretty-printed JSON
