# UUID

UUID (RFC 9562) generation, formatting, and parsing for Kestrel. Stored as two `UInt64` halves of the 128-bit value.

## Installation

```toml
[dependencies]
uuid = { path = "../../lang/uuid" }
```

Depends on `crypto` (for the OS-backed secure random source).

## Usage

```kestrel
// Random v4 UUID using the OS cryptographic random source
let id = UUID.v4();
let s = id.formatted();   // "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"

// Parse the canonical form (failable — null on invalid input)
let parsed = UUID(from: "550e8400-e29b-41d4-a716-446655440000");
```

## API

`UUID` conforms to `Equatable`, `Hashable`, `Formattable`, `Matchable`.

- `static func v4() -> UUID` - random version-4/variant-1 UUID via `crypto.random.SecureRandom`
- `static func v4[R](mutating using rng: R) -> UUID where R: RandomNumberGenerator` - same, with your own RNG (deterministic tests)
- `init(from string: String)?` - parses `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`; `null` if invalid
- `init(high: UInt64, low: UInt64)` - raw 128-bit halves
- `static var nil: UUID` - the all-zeros UUID
- `var isNil: Bool`
- Formatting renders the canonical lowercase hyphenated form (works in string interpolation via `Formattable`)
