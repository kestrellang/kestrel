# Quill

Format-agnostic serialization for Kestrel. Quill provides an intermediate Value representation and protocols for converting types to and from any data format.

## Installation

```toml
[dependencies]
kestrel/quill = "0.1.0"
```

## Overview

Quill separates serialization into two concerns:

1. **Type conversion** - your types conform to `Serialize` and `Deserialize` to convert to/from `Value`
2. **Format encoding** - format libraries (quill-json, quill-toml) conform to `Format` to encode/decode `Value` to/from strings

This means you write serialization logic once and get support for every format.

## Value

The core intermediate representation:

```kestrel
// Value is an enum with these cases:
// .Null, .Boolean(Bool), .Int(Int64), .Float(Float64),
// .Str(String), .Arr(Array[Value]), .Obj(Array[(String, Value)])
```

## Serialize Protocol

```kestrel
protocol Serialize {
    func toValue() -> Result[Value, SerializeError]
}
```

Built-in conformances: Bool, Int64, Float64, String, Optional, Array, Value.

## Deserialize Protocol

```kestrel
protocol Deserialize {
    static func fromValue(value: Value) -> Result[Self, DeserializeError]
}
```

## Format Protocol

```kestrel
protocol Format {
    func encode(value: Value) -> Result[String, SerializeError]
    func decode(source: String) -> Result[Value, DeserializeError]
    func contentType() -> String
}
```

## Format Libraries

- **kestrel/quill-json** - JSON encoding/decoding
- **kestrel/quill-toml** - TOML encoding/decoding
