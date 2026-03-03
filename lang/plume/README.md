# Plume

Lightweight string templating for Kestrel.

## Installation

```toml
[dependencies]
kestrel/plume = "0.1.0"
```

## Usage

```kestrel
var t = Template()
t.put("name", "Alice")
t.setInt("count", 42)

let html = t.render("<p>Hello {name}, you have {count} items</p>")
// <p>Hello Alice, you have 42 items</p>
```

## Key Types

- **Template** - template engine with variable substitution

## API

- `put(k, v)` - set a variable (HTML-escaped)
- `setRaw(k, v)` - set a variable without escaping
- `setInt(k, v)` - set an integer variable
- `unset(k)` - remove a variable
- `clear()` - remove all variables
- `render(pattern)` - render a template string

## Template Syntax

- `{key}` - replaced with the variable value
- `{{` - literal `{`
- `}}` - literal `}`
- Missing keys produce an empty string
- `put()` automatically escapes `<`, `>`, `&`, `"` for HTML safety
