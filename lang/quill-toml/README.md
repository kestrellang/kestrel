# Quill-TOML

TOML support for the quill serialization framework.

## Installation

```toml
[dependencies]
kestrel/quill = "0.1.0"
kestrel/quill-toml = "0.1.0"
```

## Usage

```kestrel
// Encode a Value to TOML
let value = Value.Obj([
    ("name", Value.Str("my-package")),
    ("version", Value.Str("0.1.0"))
])
let tomlStr = try Toml().encode(value: value)

// Decode TOML to a Value
let parsed = try Toml().decode(source: "name = \"my-package\"\nversion = \"0.1.0\"")

// Convenience function for Serialize types
let output = try toToml(value: mySerializableValue)
```

## Key Types

- **Toml** - implements the quill `Format` protocol for TOML
- Content type: `application/toml`
