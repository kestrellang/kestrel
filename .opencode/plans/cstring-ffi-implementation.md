# Implementation Plan: CString Type for FFI

## Goal
Add a `CString` type that provides C-compatible null-terminated strings for FFI use, along with a `toCString()` method on `String`.

## Design Decisions

### 1. CString Type Location
- **File**: `/Users/dino/Documents/Projects/kestrel/lang/std/ffi/cstring.ks`
- **Module**: `std.ffi`
- **Conformance**: `FFISafe` - allows direct use in `@extern(.C)` function signatures

### 2. CString API

```kestrel
public struct CString: FFISafe {
    /// Pointer to null-terminated bytes (FFI Safe typed pointer)
    public var ptr: Pointer[UInt8]
    
    /// Length in bytes (excluding null terminator)
    public var length: Int64
    
    /// Creates from String (allocates + copies + null terminates)
    public init(from string: String)
    
    /// Creates from existing C pointer (unsafe, caller manages memory)
    public init(fromPointer pointer: Pointer[UInt8], length: Int64)
    
    /// Frees allocated memory (only for init(from:) created instances)
    deinit
}
```

**Design Rationale**:
- Uses `Pointer[UInt8]` instead of `lang.ptr[lang.i8]` for consistency with the rest of std
- `UInt8` is the standard library type for byte-level operations
- Matches how `String` stores its bytes internally (`Pointer[UInt8]`)
- Pointer[T] is already FFI Safe when T is FFI Safe (via existing extension in pointer.ks:137)
- When calling C functions expecting `char*`, use `cstring.ptr.asRaw().raw` to get the underlying `lang.ptr[lang.i8]`

### 3. String Extension

**Convertible Protocol Implementation** (in cstring.ks):
```kestrel
extend String: Convertible[CString] {
    public init(from cstring: CString)
}
```

**Convenience Method** (in cstring.ks):
```kestrel
extend String {
    public func toCString() -> CString
}
```

## Implementation Details

### Memory Management
- `CString(from: String)` allocates with `malloc`, copies bytes via `memcpy`, adds null terminator via `Pointer.write()`
- `deinit` frees with `free` only if `ptr.isNull` is false
- `CString(fromPointer:)` does NOT free the pointer (assumes external ownership)

**Implementation Note**: malloc/free work with `lang.ptr[lang.i8]`, so internally we convert between `Pointer[UInt8]` and the raw pointer as needed:
- To allocate: `let raw = malloc(size)` then `Pointer(raw: raw).cast[UInt8]()`
- To free: `free(cstring.ptr.asRaw().raw)`

### String Construction from CString
- Uses `String.fromBytesUnchecked()` to create String from CString bytes
- Excludes the null terminator from the resulting String

### Pointer Operations
Uses Pointer methods and standard library types:
- `Pointer[UInt8]` for the internal storage (consistent with String)
- `Pointer.offset(by:)` - offset pointer by n elements  
- `Pointer.write(value:)` - write byte at pointer
- `Pointer.isNull` - check if null
- `Pointer.asRaw().raw` - access underlying lang.ptr when needed for C functions
- `malloc`, `free`, `memcpy` from `std.ffi` (working with `lang.ptr[lang.i8]`)

**FFI Safety**: Since `Pointer[UInt8]` is already FFI Safe (conforms via extension when UInt8 is FFI Safe), CString automatically becomes FFI Safe by containing it as a field.

## Usage Examples

### Basic FFI Usage
```kestrel
@extern(.C, mangleName: "puts")
func puts(s: CString) -> Int32

let message = "Hello, C!"
puts(message.toCString())

// For functions expecting raw lang.ptr[lang.i8]:
@extern(.C, mangleName: "strlen")
func strlen(s: lang.ptr[lang.i8]) -> lang.i64

let cstr = message.toCString()
let len = strlen(cstr.ptr.asRaw().raw)  // Pointer[UInt8] -> RawPointer -> lang.ptr[lang.i8]
```

### Round-trip Conversion
```kestrel
let original = "Hello"
let cstr = original.toCString()
let back = String(from: cstr)
// back == "Hello"
```

### Wrapping C Pointer
```kestrel
let cPtr: lang.ptr[lang.i8] = getStringFromC()
let cstr = CString(fromPointer: Pointer(raw: cPtr).cast[UInt8](), length: lengthFromC)
// Use cstr.ptr or cstr.length
// Note: cstr will try to free on deinit - problematic!
```

**Issue Identified**: The current design has a problem - `fromPointer` creates a CString that will be freed by `deinit`, but external C pointers shouldn't be freed by us.

### Fix Needed
Add a flag to track ownership:
```kestrel
private var ownsMemory: Bool

// init(from:) sets ownsMemory = true
// init(fromPointer:) sets ownsMemory = false
// deinit only frees if ownsMemory == true
```

## Files to Create/Modify

1. **NEW**: `/Users/dino/Documents/Projects/kestrel/lang/std/ffi/cstring.ks`
   - CString struct definition
   - String extension for Convertible[CString]
   - String extension for toCString()

2. **NO CHANGES NEEDED TO**: `/Users/dino/Documents/Projects/kestrel/lang/std/text/string.ks`
   - Extensions can be defined in cstring.ks

## Testing Strategy

After implementation, verify:
1. CString can be used as parameter in @extern(.C) functions
2. String.toCString() compiles and works
3. String(from: cstring) compiles and works
4. Memory is properly managed (no leaks or double-frees)

## Questions to Resolve

1. Should CString also provide `fromCString()` static method for clarity?
2. Should we expose `ownsMemory` property for debugging?
3. Should CString conform to any other protocols (Equatable, Cloneable)?

## Next Steps

Ready to implement. Await user confirmation to switch to Act Mode and execute changes.
