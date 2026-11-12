#!/usr/bin/env python3
"""
Generate integer and float type files from templates.
Run from this directory: python generate.py
"""

import os
import re
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
REPO_ROOT = SCRIPT_DIR.parents[2]
DOCS_DIR = REPO_ROOT / "docs" / "std" / "num"

# Integer type configurations
INTEGERS = [
    # (type_name, bits, signed, is_platform_default)
    ("Int8", 8, True, False),
    ("Int16", 16, True, False),
    ("Int32", 32, True, False),
    ("Int64", 64, True, True),  # Int alias
    ("UInt8", 8, False, False),
    ("UInt16", 16, False, False),
    ("UInt32", 32, False, False),
    ("UInt64", 64, False, True),  # UInt alias
]

# Float type configurations
FLOATS = [
    # (type_name, bits, is_platform_default)
    ("Float32", 32, False),
    ("Float64", 64, True),  # Float alias
]

# Min/max values for each bit width (signed)
SIGNED_RANGES = {
    8: (-128, 127),
    16: (-32768, 32767),
    32: (-2147483648, 2147483647),
    64: (-9223372036854775808, 9223372036854775807),
}

# Max values for unsigned
UNSIGNED_MAX = {
    8: 255,
    16: 65535,
    32: 4294967295,
    64: 18446744073709551615,
}


def get_cast(from_bits: int, to_bits: int, from_signed: bool) -> str:
    """Get the cast expression from one bit width to another."""
    if from_bits == to_bits:
        return "other.raw"
    else:
        # Use 'i' prefix for signed source, 'u' prefix for unsigned source
        from_prefix = "i" if from_signed else "u"
        return f"lang.cast_{from_prefix}{from_bits}_i{to_bits}(other.raw)"


def is_lossless(from_bits: int, from_signed: bool, to_bits: int, to_signed: bool) -> bool:
    """Whether every value of the source integer type fits in the target."""
    if from_signed == to_signed:
        return to_bits >= from_bits
    # Unsigned fits in a strictly wider signed type; signed never fits unsigned.
    return not from_signed and to_bits > from_bits


def generate_integer_conversion(
    type_name: str, bits: int, signed: bool, other_name: str, other_bits: int, other_signed: bool
) -> str:
    """`init(from:)` from another integer type, plus `truncating:`,
    `clamping:` and (same width) `bitPattern:` when the conversion is lossy.

    `init(from:)` exists only when lossless (`Convertible`); a lossy source
    gets `exactly:` (failable) and the spellings that say what they do
    (`NarrowsFrom`).
    """
    cast = f"self.raw = {get_cast(other_bits, bits, other_signed)}"
    if is_lossless(other_bits, other_signed, bits, signed):
        return (
            f"    /// @name From Integer\n"
            f"    /// Converts from `{other_name}`. Lossless: every `{other_name}` fits in `{type_name}`.\n"
            f"    public init(from other: {other_name}) {{ {cast} }}"
        )

    # Out-of-range tests, written in the source type so they can't overflow.
    below = above = None
    if other_signed and signed:
        below = f"other < {other_name}(from: {type_name}.minValue)"
        above = f"other > {other_name}(from: {type_name}.maxValue)"
    elif other_signed:
        below = "other < 0"
        if bits < other_bits:
            above = f"other > {other_name}(from: {type_name}.maxValue)"
    elif signed:
        above = f"other > {other_name}(truncating: {type_name}.maxValue)"
    else:
        above = f"other > {other_name}(from: {type_name}.maxValue)"
    fits = " or ".join(c for c in (below, above) if c)
    chain = []
    if below:
        chain.append(f"if {below} {{\n            self.raw = {type_name}.minValue.raw\n        }}")
    if above:
        chain.append(f"if {above} {{\n            self.raw = {type_name}.maxValue.raw\n        }}")
    chain.append(f"{{\n            {cast}\n        }}")
    clamp = " else ".join(chain)
    out = f"""    /// @name Exactly
    /// Converts from `{other_name}` when the value fits in `{type_name}`, and
    /// is `nil` when it doesn't. `{other_name}` doesn't always fit, so there is
    /// no `init(from:)`: use `init(truncating:)` or `init(clamping:)` to say
    /// what an out-of-range value should become.
    public init(exactly other: {other_name})? {{
        if {fits} {{
            return null
        }}
        {cast}
    }}
    /// @name Truncating
    /// Converts from `{other_name}` keeping only the low bits, reinterpreted as
    /// `{type_name}`. Never traps.
    public init(truncating other: {other_name}) {{ {cast} }}
    /// @name Clamping
    /// Converts from `{other_name}`, saturating to `{type_name}.minValue` / `{type_name}.maxValue`
    /// when the value doesn't fit.
    public init(clamping other: {other_name}) {{
        {clamp}
    }}"""
    if other_bits == bits:
        out += f"""
    /// @name Bit Pattern
    /// Reinterprets the bits of a same-width `{other_name}` as `{type_name}`. Never traps.
    public init(bitPattern other: {other_name}) {{ {cast} }}"""
    return out



def generate_float_to_int_conversion(type_name: str, bits: int, signed: bool, float_name: str) -> str:
    """`NarrowsFrom[Float*]` for an integer: `exactly:` (nil for a fraction,
    NaN or out of range), and `truncating:` / `clamping:`, which both drop
    the fraction toward zero, saturate out of range, and send NaN to 0 —
    the same answer as Rust's `as`.
    """
    f = "f64" if float_name == "Float64" else "f32"
    # Bounds are powers of two, so they're exact in either float type.
    lo = f"-{2 ** (bits - 1)}.0" if signed else "0.0"
    hi = f"{2 ** (bits - 1) if signed else 2 ** bits}.0"
    if signed:
        # `cast_f*_i*` is LLVM's `fptosi.sat`: already saturating, NaN -> 0.
        clamp = f"self.raw = lang.cast_{f}_i{bits}(other.raw)"
    elif bits < 64:
        # Saturate to Int64 first, then clamp the integer into range.
        clamp = f"self.raw = {type_name}(clamping: Int64(raw: lang.cast_{f}_i64(other.raw))).raw"
    else:
        # Int64 can't hold the upper half: shift it down by 2^63, convert,
        # and set the top bit back. Past 2^64 the shifted cast saturates to
        # 2^63 - 1, so the result saturates to `maxValue`.
        top = "9223372036854775808.0"
        clamp = (
            f"if other >= {top} {{\n"
            f"            let low = Int64(raw: lang.cast_{f}_i64(other.subtract({top}).raw));\n"
            f"            self.raw = UInt64(bitPattern: low).bitwiseOr(UInt64(intLiteral: 1).shiftLeft(by: 63)).raw\n"
            f"        }} else {{\n"
            f"            self.raw = UInt64(clamping: Int64(raw: lang.cast_{f}_i64(other.raw))).raw\n"
            f"        }}"
        )
    return f"""    /// @name Exactly
    /// Converts from `{float_name}` when the value is a whole number in
    /// `{type_name}`'s range, and is `nil` otherwise (a fraction, NaN, an
    /// infinity, or out of range).
    public init(exactly other: {float_name})? {{
        // Written so a NaN fails the range test too.
        if not (other >= {lo} and other < {hi}) {{
            return null
        }}
        if other.trunc() != other {{
            return null
        }}
        {clamp}
    }}
    /// @name Truncating
    /// Converts from `{float_name}`, dropping any fraction toward zero.
    /// Out-of-range values saturate to `{type_name}.minValue` /
    /// `{type_name}.maxValue` and NaN becomes 0. Never traps.
    public init(truncating other: {float_name}) {{
        {clamp}
    }}
    /// @name Clamping
    /// Same as `init(truncating:)`: drops the fraction, saturates out of
    /// range, and NaN becomes 0. Never traps.
    public init(clamping other: {float_name}) {{
        {clamp}
    }}"""

# `{{IF_SIGNED}}` ... [`{{ELSE}}` ...] `{{END_IF}}`, each marker on its own
# line: keeps the signed or the unsigned text. Not nestable.
SIGNED_BLOCK = re.compile(r"\{\{IF_SIGNED\}\}\n(.*?)(?:\{\{ELSE\}\}\n(.*?))?\{\{END_IF\}\}\n", re.S)


def apply_signed_blocks(template: str, signed: bool) -> str:
    return SIGNED_BLOCK.sub(lambda m: m.group(1) if signed else (m.group(2) or ""), template)


def normalize_signature(line: str) -> str:
    line = line.strip()
    if "{" in line:
        line = line.split("{", 1)[0].rstrip()
    if line.endswith(";"):
        line = line[:-1].rstrip()
    # Remove default argument values to match implementation signatures.
    line = re.sub(r"=\s*[^,)]+", "", line)
    # Normalize module qualifiers that differ between interface and impl.
    line = line.replace("std.collections.", "")
    # Cleanup whitespace and stray parens introduced by default removal.
    line = re.sub(r"\)\)", ")", line)
    line = re.sub(r"\s+", " ", line)
    return line


def extract_interface_docs(interface_path: Path) -> dict[str, list[str]]:
    lines = interface_path.read_text().splitlines()
    docs: dict[str, list[str]] = {}
    doc_buffer: list[str] = []

    for line in lines:
        stripped = line.lstrip()
        if stripped.startswith("///"):
            doc_buffer.append(stripped)
            continue
        if stripped.startswith("@"):
            # Keep doc_buffer for the following declaration.
            continue
        if stripped.startswith("public "):
            sig = normalize_signature(stripped)
            if doc_buffer:
                docs[sig] = doc_buffer[:]
            doc_buffer = []
            continue
        if stripped != "":
            doc_buffer = []

    return docs


def apply_interface_docs(content: str, docs_map: dict[str, list[str]]) -> str:
    lines = content.splitlines()
    i = 0
    while i < len(lines):
        stripped = lines[i].lstrip()
        if stripped.startswith("public "):
            sig = normalize_signature(stripped)
            if sig in docs_map:
                # Find existing doc comment block directly above.
                j = i - 1
                while j >= 0 and lines[j].lstrip().startswith("///"):
                    j -= 1
                indent = lines[i][: len(lines[i]) - len(stripped)]
                new_docs = [indent + doc for doc in docs_map[sig]]
                if j + 1 <= i - 1:
                    lines[j + 1 : i] = new_docs
                    current_index = j + 1 + len(new_docs)
                else:
                    lines[i:i] = new_docs
                    current_index = i + len(new_docs)
                i = current_index + 1
                continue
        i += 1
    return "\n".join(lines) + "\n"


def generate_sign_properties(type_name: str, bits: int, signed: bool, lang_type: str, signed_prefix: str) -> str:
    """Generate sign inspection properties for integer types."""
    if signed:
        return f'''    /// Sign as a `{type_name}`: `-1`, `0`, or `1`.
    public var sign: {type_name} {{ get {{
        if Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 0)) {{ {type_name}(intLiteral: lang.i64_neg(1)) }}
        else if Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0)) {{ {type_name}.zero }}
        else {{ {type_name}.one }}
    }}}}

    /// True when `self > 0`.
    public var isPositive: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_signed_gt(self.raw, 0))
    }}}}

    /// True when `self < 0`.
    public var isNegative: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 0))
    }}}}

    /// True when `self == 0`.
    public var isZero: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0))
    }}}}
'''
    else:
        return f'''    /// Sign as a `{type_name}`: `0` for zero, `1` otherwise (unsigned types
    /// have no negative values).
    public var sign: {type_name} {{ get {{
        if Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0)) {{ {type_name}.zero }}
        else {{ {type_name}.one }}
    }}}}

    /// True when `self > 0`.
    public var isPositive: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_unsigned_gt(self.raw, 0))
    }}}}

    /// Always `false` — unsigned types cannot be negative.
    public var isNegative: Bool {{ get {{
        // Unsigned types are never negative
        false
    }}}}

    /// True when `self == 0`.
    public var isZero: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0))
    }}}}
'''


def generate_is_power_of_two(type_name: str, signed: bool, lang_type: str) -> str:
    """Generate isPowerOfTwo check."""
    if signed:
        return f'''if Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 1)) {{ false }}
        else {{ Bool(boolLiteral: lang.{lang_type}_eq(lang.{lang_type}_and(self.raw, lang.{lang_type}_sub(self.raw, 1)), 0)) }}'''
    else:
        return f'''if Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0)) {{ false }}
        else {{ Bool(boolLiteral: lang.{lang_type}_eq(lang.{lang_type}_and(self.raw, lang.{lang_type}_sub(self.raw, 1)), 0)) }}'''


def generate_count_ones(type_name: str, bits: int, lang_type: str) -> str:
    """Generate countOnes implementation using popcount intrinsic."""
    if bits == 64:
        # For 64-bit, popcount returns i64 which is already Int64
        return f"Int64(raw: lang.{lang_type}_popcount(self.raw))"
    else:
        # For smaller types, popcount returns the same type, need to widen to Int64
        return f"Int64(raw: lang.cast_i{bits}_i64(lang.{lang_type}_popcount(self.raw)))"


def generate_step_distance(type_name: str, bits: int, signed: bool, lang_type: str) -> str:
    """Generate the Steppable.distance(to:) body: `other - self` as `Int64`.

    Narrower types widen both operands to i64 first (sign-extending signed,
    zero-extending unsigned), so the subtraction can't overflow. `Int64` uses
    its own checked `subtract`. `UInt64` spans up to 2^64 - 1, so it computes
    the magnitude unsigned and converts, letting exactly -2^63 through.
    """
    if type_name == "Int64":
        return "other.subtract(self)"
    if type_name == "UInt64":
        return """if other >= self {
            return Int64(exactly: other.subtract(self))!
        }
        let back = self.subtract(other);
        // 2^63 back is exactly Int64.minValue; anything larger traps.
        if back == UInt64(bitPattern: Int64.minValue) { Int64.minValue } else { Int64(exactly: back)!.negate() }"""
    prefix = "i" if signed else "u"
    return (
        f"Int64(raw: lang.i64_sub("
        f"lang.cast_{prefix}{bits}_i64(other.raw), "
        f"lang.cast_{prefix}{bits}_i64(self.raw)))"
    )


def generate_leading_zeros(type_name: str, bits: int, lang_type: str) -> str:
    """Generate leadingZeros implementation using clz intrinsic."""
    if bits == 64:
        return f"Int64(raw: lang.{lang_type}_clz(self.raw))"
    else:
        return f"Int64(raw: lang.cast_i{bits}_i64(lang.{lang_type}_clz(self.raw)))"


def generate_trailing_zeros(type_name: str, bits: int, lang_type: str) -> str:
    """Generate trailingZeros implementation using ctz intrinsic."""
    if bits == 64:
        return f"Int64(raw: lang.{lang_type}_ctz(self.raw))"
    else:
        return f"Int64(raw: lang.cast_i{bits}_i64(lang.{lang_type}_ctz(self.raw)))"


def generate_byte_swap_doc(bits: int) -> str:
    """Doc comment for byteSwapped; a single byte has nothing to swap."""
    if bits == 8:
        return (
            "/// Value with its byte order reversed. A single byte has no byte order,\n"
            "    /// so this is the identity — there is no `bswap` intrinsic at this width."
        )
    return (
        "/// Value with its byte order reversed. Use to convert between big- and\n"
        "    /// little-endian; lowered to a `bswap` intrinsic."
    )


def generate_byte_swap(type_name: str, bits: int, lang_type: str) -> str:
    """Generate byteSwapped implementation using bswap intrinsic."""
    if bits == 8:
        # Byte swap on 8-bit is a no-op
        return "self"
    else:
        return f"{type_name}(raw: lang.{lang_type}_bswap(self.raw))"


def generate_integer_format_method(type_name: str, bits: int, signed: bool) -> str:
    """Generate the format(into:) method for integer types."""

    if signed:
        # For signed types, extract digits via the same-width unsigned type.
        # This correctly handles minValue: negate() overflows back to minValue,
        # but `UIntN.zero.subtractWrapping(UIntN(truncating: n))` gives the correct magnitude for all
        # values including minValue (two's-complement unsigned wraparound).
        uint_name = f"UInt{bits}"
        digit_to_i64 = "Int64(truncating: digit)" if bits == 64 else "Int64(from: digit)"
        sign_handling = f'''
        let isNegative = n < 0;'''
        sign_prefix = '''
        if isNegative {
            result.append(char: '-')
        } else if options.sign == .Always {
            result.append(char: '+')
        } else if options.sign == .Space {
            result.append(char: ' ')
        }'''
        digit_body = f'''
        // Convert to unsigned magnitude so minValue formats correctly.
        // negate() overflows on minValue; unsigned subtraction from zero does not.
        let mag: {uint_name} = if isNegative {{
            {uint_name}.zero.subtractWrapping({uint_name}(truncating: n))
        }} else {{
            {uint_name}(truncating: n)
        }};
        let radixVal: {uint_name} = {uint_name}(truncating: radix);
        var m = mag;
        while m != {uint_name}.zero {{
            let digit: {uint_name} = m % radixVal;
            let digitVal: Int64 = {digit_to_i64};
            let charCode: Int64 = if digitVal < 10 {{
                digitVal + 48
            }} else if options.uppercase {{
                digitVal - 10 + 65
            }} else {{
                digitVal - 10 + 97
            }};
            digits.appendByte(UInt8(truncating: charCode));
            m = m / radixVal
        }}'''
    else:
        # For unsigned types no sign handling is needed; loop directly over self.
        uint_name = type_name
        sign_handling = '''
        let isNegative = false;'''
        sign_prefix = '''
        if options.sign == .Always {
            result.append(char: '+')
        } else if options.sign == .Space {
            result.append(char: ' ')
        }'''
        if bits == 64:
            radix_as_type = "UInt64(truncating: radix)"
            digit_as_i64 = "Int64(truncating: digit)"
        else:
            radix_as_type = f"{type_name}(truncating: radix)"
            digit_as_i64 = "Int64(from: digit)"
        digit_body = f'''
        let radixVal: {type_name} = {radix_as_type};
        var m = n;
        while m != {type_name}.zero {{
            let digit: {type_name} = m % radixVal;
            let digitVal: Int64 = {digit_as_i64};
            let charCode: Int64 = if digitVal < 10 {{
                digitVal + 48
            }} else if options.uppercase {{
                digitVal - 10 + 65
            }} else {{
                digitVal - 10 + 97
            }};
            digits.appendByte(UInt8(truncating: charCode));
            m = m / radixVal
        }}'''

    return f'''    // Formattable
    /// Formats the integer directly into `writer`, honouring the supplied
    /// `FormatOptions`. Implements the `Formattable` protocol.
    ///
    /// # Examples
    ///
    /// ```
    /// (42).formatted();                                           // "42"
    /// (255).formatted(.{{radix: 16}});                     // "ff"
    /// (255).formatted(.{{radix: 16, uppercase: true}});    // "FF"
    /// (255).formatted(.{{radix: 16, alternate: true}});    // "0xff"
    /// (42).formatted(.{{radix: 2, alternate: true}});      // "0b101010"
    /// (42).formatted(.{{width: .Some(5), fill: '0'}});     // "00042"
    /// (-42).formatted(.{{sign: .Always}});                 // "-42"
    /// ```
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {{
        var n = self;{sign_handling}

        var radix: Int64 = options.radix;
        if radix < 2 or radix > 36 {{
            radix = 10
        }}

        // Build digits in reverse order (need random access to reverse)
        var digits = String();
        if n == {type_name}.zero {{
            digits.appendByte(48)
        }} else {{{digit_body}
        }}

        // Build content: sign + prefix + reversed digits
        var result = String();
{sign_prefix}

        if options.alternate {{
            if radix == 2 {{
                result.append("0b")
            }} else if radix == 8 {{
                result.append("0o")
            }} else if radix == 16 {{
                result.append("0x")
            }}
        }}

        var i = digits.bytes.count - 1;
        while i >= 0 {{
            result.appendByte(unsafe digits.bytes(unchecked: i));
            i = i - 1
        }}

        _writePadded(into: writer, result, options)
    }}'''


def generate_integer_parse_method(type_name: str, bits: int, signed: bool) -> str:
    """Generate the parse() method for integer types."""
    if signed:
        min_val, max_val = SIGNED_RANGES[bits]
    else:
        min_val = 0
        max_val = UNSIGNED_MAX[bits]

    # Determine how to return the result and bounds check expressions
    # For 64-bit types, we accumulate in the same type, so just return result
    # For smaller types, we need to convert from Int64/UInt64
    if bits == 64:
        if signed:
            return_expr = "result"
            # Use type constants to avoid literal overflow issues
            min_val_expr = "Int64.minValue"
            max_val_expr = "Int64.maxValue"
        else:
            return_expr = "result"
            min_val_expr = "0"
            max_val_expr = "UInt64.maxValue"
    else:
        return_expr = f"{type_name}(truncating: result)"
        if signed:
            # Convert smaller type's bounds to Int64 for comparison
            min_val_expr = f"Int64(truncating: {type_name}.minValue)"
            max_val_expr = f"Int64(truncating: {type_name}.maxValue)"
        else:
            min_val_expr = "0"
            max_val_expr = f"UInt64(from: {type_name}.maxValue)"

    # For signed types, handle negative numbers
    if signed:
        # Per-type magnitude bounds for the UInt64 accumulator. The magnitude of
        # minValue is maxValue+1, which a signed accumulator cannot hold, so both
        # parse inits accumulate the positive magnitude in UInt64 and convert+negate.
        pos_max_expr = f"UInt64(truncating: {type_name}.maxValue)"
        neg_max_expr = f"UInt64(truncating: {type_name}.maxValue) + 1"
        base_parse = f'''    /// @name Parsing
    /// Parses a base-10 integer literal, optionally prefixed with `+` or `-`.
    /// Returns `null` for an empty string, a non-digit character,
    /// or a value that does not fit in `{type_name}`.
    ///
    /// # Examples
    ///
    /// ```
    /// {type_name}(parsing: "42");    // Some(42)
    /// {type_name}(parsing: "-7");    // Some(-7)
    /// {type_name}(parsing: "abc");   // None
    /// ```
    public init(parsing text: &StringSlice)? {{
        // One view for the whole parse; a String argument lends its slice.
        let bytes = text.bytes;
        let len = bytes.count;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {{
            isNegative = true;
            index = 1
        }} else if firstByteVal == 43 {{
            index = 1
        }}

        if index >= len {{
            return null
        }}

        // Accumulate the positive magnitude in UInt64 so that minValue's
        // magnitude (maxValue+1) is representable; convert + negate at the end.
        let maxMagnitude: UInt64 = if isNegative {{
            {neg_max_expr}
        }} else {{
            {pos_max_expr}
        }};

        var result: UInt64 = 0;

        while index < len {{
            let byte: UInt8 = unsafe bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            if byteVal < 48 or byteVal > 57 {{
                return null
            }}

            let digit = byteVal - 48;
            let digitU: UInt64 = UInt64(truncating: digit);

            if result > (maxMagnitude - digitU) / 10 {{
                return null
            }}
            result = result * 10 + digitU;

            index = index + 1
        }}

        let typedResult = {type_name}(truncating: result);
        if isNegative {{
            self.raw = typedResult.negateWrapping().raw
        }} else {{
            self.raw = typedResult.raw
        }}
    }}'''
        radix_parse = f'''
    /// @name Parsing with Radix
    /// Parses an integer in `radix` (base 2-36 inclusive). Letters a-z are
    /// case-insensitive and represent digit values 10-35.
    ///
    /// # Examples
    ///
    /// ```
    /// {type_name}(parsing: "ff", radix: 16);     // Some(255 if it fits, else None)
    /// {type_name}(parsing: "101010", radix: 2);  // Some(42)
    /// {type_name}(parsing: "z", radix: 36);      // Some(35)
    /// ```
    public init(parsing text: &StringSlice, radix radix: Int64)? {{
        // One view for the whole parse; a String argument lends its slice.
        let bytes = text.bytes;
        if radix < 2 or radix > 36 {{
            return null
        }}

        let len = bytes.count;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {{
            isNegative = true;
            index = 1
        }} else if firstByteVal == 43 {{
            index = 1
        }}

        if index >= len {{
            return null
        }}

        let radixU: UInt64 = UInt64(truncating: radix);
        let maxMagnitude: UInt64 = if isNegative {{
            {neg_max_expr}
        }} else {{
            {pos_max_expr}
        }};

        var result: UInt64 = 0;

        while index < len {{
            let byte: UInt8 = unsafe bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            let digit: Int64 = if byteVal >= 48 and byteVal <= 57 {{
                byteVal - 48
            }} else if byteVal >= 65 and byteVal <= 90 {{
                byteVal - 55
            }} else if byteVal >= 97 and byteVal <= 122 {{
                byteVal - 87
            }} else {{
                return null
            }};

            if digit >= radix {{
                return null
            }}

            let digitU: UInt64 = UInt64(truncating: digit);
            if result > (maxMagnitude - digitU) / radixU {{
                return null
            }}
            result = result * radixU + digitU;
            index = index + 1
        }}

        let typedResult = {type_name}(truncating: result);
        if isNegative {{
            self.raw = typedResult.negateWrapping().raw
        }} else {{
            self.raw = typedResult.raw
        }}
    }}'''
        return base_parse + radix_parse
    else:
        # Unsigned - no negative numbers allowed
        max_before_multiply = "1844674407370955161"  # UInt64.maxValue / 10

        base_parse = f'''    /// @name Parse
    /// Parses a base-10 unsigned integer literal, optionally prefixed
    /// with `+`. A leading `-` is rejected. Returns `null` for an empty
    /// string, a non-digit character, or a value that does not fit in
    /// `{type_name}`.
    ///
    /// # Examples
    ///
    /// ```
    /// let n = {type_name}(parsing: "42");   // Some(42)
    /// let bad = {type_name}(parsing: "-1"); // null (no sign for unsigned)
    /// ```
    public init(parsing text: &StringSlice)? {{
        // One view for the whole parse; a String argument lends its slice.
        let bytes = text.bytes;
        let len = bytes.count;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 43 {{
            index = 1
        }} else if firstByteVal == 45 {{
            return null
        }}

        if index >= len {{
            return null
        }}

        var result: UInt64 = 0;
        let maxBeforeMultiply: UInt64 = {max_before_multiply};
        let maxVal: UInt64 = {max_val_expr};

        while index < len {{
            let byte: UInt8 = unsafe bytes(unchecked: index);
            let byteVal = UInt64(from: byte);

            if byteVal < 48 or byteVal > 57 {{
                return null
            }}

            let digit = byteVal - 48;

            if result > maxBeforeMultiply {{
                return null
            }}
            result = result * 10;

            if result > UInt64.maxValue - digit {{
                return null
            }}
            result = result + digit;

            index = index + 1
        }}

        if result > maxVal {{
            return null
        }}

        self.raw = {return_expr}.raw;
    }}'''
        # Per-type max for the UInt64 accumulator (no sign bookkeeping).
        if type_name == "UInt64":
            max_expr = "UInt64.maxValue"
        else:
            max_expr = f"UInt64(from: {type_name}.maxValue)"
        radix_parse = f'''
    /// @name Parse Radix
    /// Parses an unsigned integer in `radix` (base 2-36 inclusive). Letters
    /// a-z are case-insensitive and represent digit values 10-35. A
    /// leading `+` is allowed but a leading `-` is rejected. Returns
    /// `null` for an out-of-range radix, an empty string, an
    /// unrecognised digit, or a value that overflows `{type_name}`.
    ///
    /// # Examples
    ///
    /// ```
    /// let n = {type_name}(parsing: "ff", radix: 16);      // Some(255 if it fits, else None)
    /// let m = {type_name}(parsing: "101010", radix: 2);   // Some(42)
    /// ```
    public init(parsing text: &StringSlice, radix radix: Int64)? {{
        // One view for the whole parse; a String argument lends its slice.
        let bytes = text.bytes;
        if radix < 2 or radix > 36 {{
            return null
        }}

        let len = bytes.count;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 43 {{
            index = 1
        }} else if firstByteVal == 45 {{
            return null
        }}

        if index >= len {{
            return null
        }}

        let radixU: UInt64 = UInt64(truncating: radix);
        let maxVal: UInt64 = {max_expr};

        var result: UInt64 = 0;

        while index < len {{
            let byte: UInt8 = unsafe bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            let digit: Int64 = if byteVal >= 48 and byteVal <= 57 {{
                byteVal - 48
            }} else if byteVal >= 65 and byteVal <= 90 {{
                byteVal - 55
            }} else if byteVal >= 97 and byteVal <= 122 {{
                byteVal - 87
            }} else {{
                return null
            }};

            if digit >= radix {{
                return null
            }}

            let digitU: UInt64 = UInt64(truncating: digit);
            if result > (maxVal - digitU) / radixU {{
                return null
            }}
            result = result * radixU + digitU;
            index = index + 1
        }}

        self.raw = {return_expr}.raw;
    }}'''
        return base_parse + radix_parse


def generate_integer_byte_conversion_method(type_name: str, bits: int, signed: bool) -> str:
    """Generate byte-conversion methods for any integer width.

    `toBytes()` / `fromBytes()` use a raw-pointer cast and run for `bits / 8`
    bytes, so they work for any width without per-width tweaks. The
    big/little-endian forms widen `self` to `UInt64` (sign-extended for
    signed types — high bits never appear in the output because the byte
    extraction loop is bounded by the type's byte count), then mask-shift
    out each byte.
    """
    byte_count = bits // 8
    bc = f"{byte_count}"
    # Same-width same-type means no Convertible[UInt64] conformance exists for
    # UInt64 itself (no self-conversion); use plain identifiers in that case.
    # Signed → UInt64 reinterprets the bits; narrower unsigned widens exactly.
    if type_name == "UInt64":
        widen_self = "self"
    elif signed:
        widen_self = "UInt64(truncating: self)"
    else:
        widen_self = "UInt64(from: self)"
    narrow_result = "result" if type_name == "UInt64" else f"{type_name}(truncating: result)"

    return f'''    /// Splits this integer into {byte_count} bytes in *native* (host) byte order.
    /// Use `toBytesBigEndian` / `toBytesLittleEndian` when serialising for
    /// a fixed wire format.
    ///
    /// # Examples
    ///
    /// ```
    /// let bytes = {type_name}.maxValue.toBytes();   // {byte_count} bytes, host order
    /// ```
    public func toBytes() -> std.collections.Array[UInt8] {{
        var result = std.collections.Array[UInt8](capacity: {bc});
        let value = self;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < {bc} {{
            result.append(unsafe ptr.offset(by: i).read());
            i = i + 1
        }}
        result
    }}

    /// Splits this integer into {byte_count} bytes in big-endian order (most
    /// significant byte first — i.e. network byte order).
    public func toBytesBigEndian() -> std.collections.Array[UInt8] {{
        var result = std.collections.Array[UInt8](capacity: {bc});
        let value = {widen_self};
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < {bc} {{
            let shift = ({bc} - 1 - i) * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(truncating: byteVal));
            i = i + 1
        }}
        result
    }}

    /// Splits this integer into {byte_count} bytes in little-endian order (least
    /// significant byte first).
    public func toBytesLittleEndian() -> std.collections.Array[UInt8] {{
        var result = std.collections.Array[UInt8](capacity: {bc});
        let value = {widen_self};
        let mask: UInt64 = 255;
        var i: Int64 = 0;
        while i < {bc} {{
            let shift = i * 8;
            let byteVal = value.shiftRight(by: shift).bitwiseAnd(mask);
            result.append(UInt8(truncating: byteVal));
            i = i + 1
        }}
        result
    }}

    /// @name From Bytes
    /// Reassembles a `{type_name}` from {byte_count} bytes in native byte order.
    /// Returns `null` if the input is not exactly {byte_count} bytes long.
    public init(fromBytes fromBytes: &Slice[UInt8])? {{
        let bytes = fromBytes.asUnsafeSlice();
        if bytes.count != {bc} {{
            return null
        }}
        var value = {type_name}.zero;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < {bc} {{
            unsafe ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }}
        self.raw = value.raw;
    }}

    /// @name From Bytes Big Endian
    /// Reassembles a `{type_name}` from {byte_count} bytes in big-endian order.
    /// Returns `null` if the input is not exactly {byte_count} bytes long.
    public init(fromBytesBigEndian fromBytesBigEndian: &Slice[UInt8])? {{
        let bytes = fromBytesBigEndian.asUnsafeSlice();
        if bytes.count != {bc} {{
            return null
        }}
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < {bc} {{
            let byteVal = UInt64(from: unsafe bytes(unchecked: i));
            result = (result << 8) | byteVal;
            i = i + 1
        }}
        self.raw = {narrow_result}.raw;
    }}

    /// @name From Bytes Little Endian
    /// Reassembles a `{type_name}` from {byte_count} bytes in little-endian order.
    /// Returns `null` if the input is not exactly {byte_count} bytes long.
    public init(fromBytesLittleEndian fromBytesLittleEndian: &Slice[UInt8])? {{
        let bytes = fromBytesLittleEndian.asUnsafeSlice();
        if bytes.count != {bc} {{
            return null
        }}
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < {bc} {{
            let shift = i * 8;
            let byteVal = UInt64(from: unsafe bytes(unchecked: i));
            result = result | (byteVal << shift);
            i = i + 1
        }}
        self.raw = {narrow_result}.raw;
    }}'''


def generate_integer(type_name: str, bits: int, signed: bool, is_default: bool) -> str:
    template = (SCRIPT_DIR / "integer.ks.template").read_text()

    lang_type = f"i{bits}"

    # Generate Convertible conformances and inits for all other integer types
    other_types = [(name, b, s) for name, b, s, _ in INTEGERS if name != type_name]

    # `Convertible` is lossless: the identity and every source that always
    # fits. Every other source is `NarrowsFrom` only.
    lossless = [n for n, b, sg in other_types if is_lossless(b, sg, bits, signed)]
    lossy = [n for n, b, sg in other_types if not is_lossless(b, sg, bits, signed)]
    conformances = [f"    Convertible[{name}]" for name in [type_name] + lossless]
    conformances += [f"    NarrowsFrom[{name}]" for name in lossy]
    conformances += [f"    NarrowsFrom[{name}]" for name, _, _ in FLOATS]
    identity = (
        f"    /// @name Identity\n"
        f"    /// `{type_name}` from itself, so a `Convertible[T]` bound accepts `{type_name}`.\n"
        f"    public init(from other: {type_name}) {{ self.raw = other.raw }}"
    )
    if type_name == "Int64":
        # Every other number gets `Numeric.init(exactly: Int64)?` from
        # `NarrowsFrom[Int64]` (or, for floats, its own); Int64 always fits.
        identity += (
            "\n    /// @name Exactly\n"
            "    /// Always succeeds: satisfies `Numeric.init(exactly:)`, which other\n"
            "    /// numbers fail when the value doesn't fit.\n"
            "    public init(exactly other: Int64)? { self.raw = other.raw }"
        )
    inits = [identity] + [
        generate_integer_conversion(type_name, bits, signed, other_name, other_bits, other_signed)
        for other_name, other_bits, other_signed in other_types
    ] + [
        generate_float_to_int_conversion(type_name, bits, signed, float_name)
        for float_name, _, _ in FLOATS
    ]

    # Join with comma+newline, no trailing comma
    convertible_conformances = ",\n".join(conformances) + "\n" if conformances else ""
    convertible_inits = "\n".join(inits) + "\n" if inits else ""

    if signed:
        min_val, max_val = SIGNED_RANGES[bits]
        min_val_abs = abs(min_val)
        signedness = "signed"
        signedness_protocol = "SignedInteger"
        signed_prefix = "signed_"
        negatable = "Negatable,"
        negatable_output = f"type Negatable.Output = {type_name}"
        # For signed, compute minValue via shift left: 1 << (bits - 1)
        # This avoids literal overflow issues with large values like 9223372036854775808
        min_value_expr = f"{type_name}(raw: lang.{lang_type}_shl(1, {bits - 1}))"
        # Format min/max with commas for readability
        min_val_formatted = f"{min_val:,}".replace(",", "_")
        max_val_formatted = f"{max_val:,}".replace(",", "_")
        min_value_doc = f"/// This is -2^{bits-1} ({min_val_formatted})."
        max_value_doc = f"/// This is 2^{bits-1} - 1 ({max_val_formatted})."
    else:
        min_val = 0
        min_val_abs = 0
        max_val = UNSIGNED_MAX[bits]
        signedness = "unsigned"
        signedness_protocol = "UnsignedInteger"
        signed_prefix = "unsigned_"
        negatable = ""
        negatable_output = ""
        min_value_expr = f"{type_name}(intLiteral: 0)"
        max_val_formatted = f"{max_val:,}".replace(",", "_")
        min_value_doc = "/// This is always 0 for unsigned types."
        max_value_doc = f"/// This is 2^{bits} - 1 ({max_val_formatted})."

    # Int literal init - need to cast from i64 for smaller types
    if bits == 64:
        int_literal_init = "self.raw = value"
    else:
        int_literal_init = f"self.raw = lang.cast_i64_i{bits}(value)"

    # Shift cast - need to cast count from i64 for smaller types
    if bits == 64:
        shift_cast = "count.raw"
        shift_cast_i = "i.raw"
    else:
        shift_cast = f"lang.cast_i64_i{bits}(count.raw)"
        shift_cast_i = f"lang.cast_i64_i{bits}(i.raw)"

    # Type alias for platform defaults
    if is_default:
        if signed:
            type_alias = f"\n/// Platform-sized signed integer — currently always `Int64`.\npublic type Int = {type_name}"
        else:
            type_alias = f"\n/// Platform-sized unsigned integer — currently always `UInt64`.\npublic type UInt = {type_name}"
    else:
        type_alias = ""

    # Generate format method
    format_method = generate_integer_format_method(type_name, bits, signed)
    byte_conversion = generate_integer_byte_conversion_method(type_name, bits, signed)

    # Generate sign properties
    sign_properties = generate_sign_properties(type_name, bits, signed, lang_type, signed_prefix)

    # Generate isPowerOfTwo
    is_power_of_two = generate_is_power_of_two(type_name, signed, lang_type)

    # Generate Steppable.distance(to:)
    step_distance_impl = generate_step_distance(type_name, bits, signed, lang_type)

    # Generate bit counting operations using intrinsics
    count_ones_impl = generate_count_ones(type_name, bits, lang_type)
    leading_zeros_impl = generate_leading_zeros(type_name, bits, lang_type)
    trailing_zeros_impl = generate_trailing_zeros(type_name, bits, lang_type)

    # Generate byte swap using intrinsic
    byte_swap_impl = generate_byte_swap(type_name, bits, lang_type)



    # Generate parse method
    parse_method = generate_integer_parse_method(type_name, bits, signed)

    result = apply_signed_blocks(template, signed)
    # The hash word: the value's bits as a UInt64. Unsigned types widen
    # losslessly (from:); signed types reinterpret (truncating:).
    if type_name == "UInt64":
        hash_word = "self"
    elif signed:
        hash_word = "UInt64(truncating: self)"
    else:
        hash_word = "UInt64(from: self)"
    result = result.replace("{{HASH_WORD}}", hash_word)
    result = result.replace("{{TYPE_NAME}}", type_name)
    result = result.replace("{{BITS}}", str(bits))
    result = result.replace("{{SIGNEDNESS}}", signedness)
    result = result.replace("{{SIGNEDNESS_PROTOCOL}}", signedness_protocol)
    result = result.replace("{{LANG_TYPE}}", lang_type)
    result = result.replace("{{MIN_VALUE}}", str(min_val))
    result = result.replace("{{MIN_VALUE_ABS}}", str(min_val_abs))
    result = result.replace("{{MIN_VALUE_EXPR}}", min_value_expr)
    result = result.replace("{{MIN_VALUE_DOC}}", min_value_doc)
    result = result.replace("{{MAX_VALUE}}", str(max_val))
    result = result.replace("{{MAX_VALUE_DOC}}", max_value_doc)
    result = result.replace("{{SIGNED_PREFIX}}", signed_prefix)
    result = result.replace("{{NEGATABLE}}", negatable)
    result = result.replace("{{NEGATABLE_OUTPUT}}", negatable_output)
    result = result.replace("{{INT_LITERAL_INIT}}", int_literal_init)
    result = result.replace("{{SHIFT_CAST}}", shift_cast)
    result = result.replace("{{SHIFT_CAST_I}}", shift_cast_i)
    result = result.replace("{{TYPE_ALIAS}}", type_alias)
    result = result.replace("{{CONVERTIBLE_CONFORMANCES}}", convertible_conformances)
    result = result.replace("{{CONVERTIBLE_INITS}}", convertible_inits)
    result = result.replace("{{FORMAT_METHOD}}", format_method)
    result = result.replace("{{BYTE_CONVERSION}}", byte_conversion)
    result = result.replace("{{SIGN_PROPERTIES}}", sign_properties)
    result = result.replace("{{IS_POWER_OF_TWO}}", is_power_of_two)
    result = result.replace("{{STEP_DISTANCE_IMPL}}", step_distance_impl)
    result = result.replace("{{COUNT_ONES_IMPL}}", count_ones_impl)
    result = result.replace("{{LEADING_ZEROS_IMPL}}", leading_zeros_impl)
    result = result.replace("{{TRAILING_ZEROS_IMPL}}", trailing_zeros_impl)
    result = result.replace("{{BYTE_SWAP_IMPL}}", byte_swap_impl)
    result = result.replace("{{BYTE_SWAP_DOC}}", generate_byte_swap_doc(bits))
    result = result.replace("{{PARSE_METHOD}}", parse_method)

    interface_path = DOCS_DIR / f"{type_name.lower()}.ks.interface"
    if interface_path.exists():
        docs_map = extract_interface_docs(interface_path)
        result = apply_interface_docs(result, docs_map)

    return result


def generate_float_parse_method(type_name: str, bits: int) -> str:
    """Generate the parse() method for float types.

    Digits are accumulated into a big integer and converted to the correctly-
    rounded float by `floatRoundDecimal` (float_digits.ks). The old code did
    `result * 10.pow(exp)` in float arithmetic, which lost precision and
    overflowed `10^k` to infinity for large exponents -> 0 (issue #216).
    """
    lang_type = f"f{bits}"

    if bits == 64:
        sig_bits = 52
        min_e = -1074
        max_e = 971
        exp_bias_add = 1075     # rawExp = e + exp_bias_add
        exp_mask = 2047
        hidden_bit = 1 << 52
        mant_mask = (1 << 52) - 1
        sign_shift = 63
        from_bits = "lang.f64_from_bits(rawBits.raw)"
    else:
        sig_bits = 23
        min_e = -149
        max_e = 104
        exp_bias_add = 150
        exp_mask = 255
        hidden_bit = 1 << 23
        mant_mask = (1 << 23) - 1
        sign_shift = 31
        from_bits = "lang.f32_from_bits(UInt32(truncating: rawBits).raw)"

    method = '''    /// @name Parsing
    /// Parses a `__TYPE_NAME__` from a string. Recognises decimal
    /// (`"3.14"`), scientific (`"1.5e10"`, `"2.5E-3"`), and the special
    /// tokens `"inf"`, `"-inf"`, `"+inf"`, `"infinity"`, `"nan"`
    /// (case-insensitive). Returns `null` for any other input.
    ///
    /// # Examples
    ///
    /// ```
    /// __TYPE_NAME__(parsing: "3.14");      // Some(3.14)
    /// __TYPE_NAME__(parsing: "-2.5e10");   // Some(-2.5e10)
    /// __TYPE_NAME__(parsing: "inf");       // Some(infinity)
    /// __TYPE_NAME__(parsing: "nan");       // Some(nan)
    /// __TYPE_NAME__(parsing: "abc");       // None
    /// __TYPE_NAME__(parsing: "");          // None
    /// ```
    public init(parsing text: &StringSlice)? {
        // One view for the whole parse; a String argument lends its slice.
        let bytes = text.bytes;
        let len = bytes.count;
        if len == 0 {
            return null
        }

        if len == 3 {
            let b0: UInt8 = unsafe bytes(unchecked: 0);
            let b1: UInt8 = unsafe bytes(unchecked: 1);
            let b2: UInt8 = unsafe bytes(unchecked: 2);
            let isN0 = Int64(from: b0) == 110 or Int64(from: b0) == 78;
            let isA1 = Int64(from: b1) == 97 or Int64(from: b1) == 65;
            let isN2 = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            if isN0 and isA1 and isN2 {
                self.raw = lang.__LANG_TYPE___nan();
                return
            }
        }

        if len == 3 {
            let b0: UInt8 = unsafe bytes(unchecked: 0);
            let b1: UInt8 = unsafe bytes(unchecked: 1);
            let b2: UInt8 = unsafe bytes(unchecked: 2);
            let isI = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            if isI and isN and isF {
                self.raw = lang.__LANG_TYPE___infinity();
                return
            }
        }

        if len == 4 {
            let b0: UInt8 = unsafe bytes(unchecked: 0);
            let b1: UInt8 = unsafe bytes(unchecked: 1);
            let b2: UInt8 = unsafe bytes(unchecked: 2);
            let b3: UInt8 = unsafe bytes(unchecked: 3);
            let isMinus = Int64(from: b0) == 45;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isMinus and isI and isN and isF {
                self.raw = lang.__LANG_TYPE___neg(lang.__LANG_TYPE___infinity());
                return
            }
        }

        if len == 4 {
            let b0: UInt8 = unsafe bytes(unchecked: 0);
            let b1: UInt8 = unsafe bytes(unchecked: 1);
            let b2: UInt8 = unsafe bytes(unchecked: 2);
            let b3: UInt8 = unsafe bytes(unchecked: 3);
            let isPlus = Int64(from: b0) == 43;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isPlus and isI and isN and isF {
                self.raw = lang.__LANG_TYPE___infinity();
                return
            }
        }

        if len == 8 {
            let b0: UInt8 = unsafe bytes(unchecked: 0);
            let b1: UInt8 = unsafe bytes(unchecked: 1);
            let b2: UInt8 = unsafe bytes(unchecked: 2);
            let b3: UInt8 = unsafe bytes(unchecked: 3);
            let b4: UInt8 = unsafe bytes(unchecked: 4);
            let b5: UInt8 = unsafe bytes(unchecked: 5);
            let b6: UInt8 = unsafe bytes(unchecked: 6);
            let b7: UInt8 = unsafe bytes(unchecked: 7);
            let isI0 = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN1 = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF2 = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            let isI3 = Int64(from: b3) == 105 or Int64(from: b3) == 73;
            let isN4 = Int64(from: b4) == 110 or Int64(from: b4) == 78;
            let isI5 = Int64(from: b5) == 105 or Int64(from: b5) == 73;
            let isT6 = Int64(from: b6) == 116 or Int64(from: b6) == 84;
            let isY7 = Int64(from: b7) == 121 or Int64(from: b7) == 89;
            if isI0 and isN1 and isF2 and isI3 and isN4 and isI5 and isT6 and isY7 {
                self.raw = lang.__LANG_TYPE___infinity();
                return
            }
        }

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = unsafe bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {
            isNegative = true;
            index = 1
        } else if firstByteVal == 43 {
            index = 1
        }

        if index >= len {
            return null
        }

        // Accumulate every significant digit into the big integer `mantissa`
        // and count the fractional digits, so value = mantissa * 10^(exp - frac).
        var mantissa = Array[UInt32]();
        var hasDigits = false;
        var fracCount: Int64 = 0;
        var currentByte: Int64 = Int64(from: unsafe bytes(unchecked: index));

        while index < len and currentByte >= 48 and currentByte <= 57 {
            mantissa = bnMulSmall(mantissa, UInt32(truncating: 10));
            mantissa = bnAddSmall(mantissa, UInt64(truncating: currentByte - 48));
            hasDigits = true;
            index = index + 1;
            if index < len {
                currentByte = Int64(from: unsafe bytes(unchecked: index))
            }
        }

        if index < len and currentByte == 46 {
            index = index + 1;
            if index < len {
                currentByte = Int64(from: unsafe bytes(unchecked: index));
                while index < len and currentByte >= 48 and currentByte <= 57 {
                    mantissa = bnMulSmall(mantissa, UInt32(truncating: 10));
                    mantissa = bnAddSmall(mantissa, UInt64(truncating: currentByte - 48));
                    fracCount = fracCount + 1;
                    hasDigits = true;
                    index = index + 1;
                    if index < len {
                        currentByte = Int64(from: unsafe bytes(unchecked: index))
                    }
                }
            }
        }

        if not hasDigits {
            return null
        }

        var expValue: Int64 = 0;

        if index < len and (currentByte == 101 or currentByte == 69) {
            index = index + 1;

            if index >= len {
                return null
            }

            var expNegative = false;
            currentByte = Int64(from: unsafe bytes(unchecked: index));

            if currentByte == 45 {
                expNegative = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: unsafe bytes(unchecked: index))
                }
            } else if currentByte == 43 {
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: unsafe bytes(unchecked: index))
                }
            }

            if index >= len {
                return null
            }

            var hasExpDigit = false;

            while index < len and currentByte >= 48 and currentByte <= 57 {
                // Cap accumulation far beyond any representable exponent so an
                // absurdly long exponent can't overflow Int64 and wrap negative
                // (which would turn an overflow into a spurious 0). |k| > ~400
                // already saturates to inf / 0.
                if expValue < 1000000 {
                    expValue = expValue * 10 + (currentByte - 48)
                };
                hasExpDigit = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: unsafe bytes(unchecked: index))
                }
            }

            if not hasExpDigit {
                return null
            }

            if expNegative {
                expValue = expValue.negate()
            }
        }

        if index != len {
            return null
        }

        // value = mantissa * 10^k, rounded to the nearest float (round-even).
        let k = expValue - fracCount;
        let parts = floatRoundDecimal(mantissa, k, __SIG_BITS__, __MIN_E__, __MAX_E__);
        var rawBits = UInt64.zero;
        if parts.overflow {
            rawBits = UInt64(truncating: __EXP_MASK__).shiftLeft(by: __SIG_BITS__)
        } else if parts.m == UInt64.zero {
            rawBits = UInt64.zero
        } else if parts.m >= UInt64(truncating: __HIDDEN_BIT__) {
            let rawExp = parts.e + __EXP_BIAS_ADD__;
            rawBits = UInt64(truncating: rawExp).shiftLeft(by: __SIG_BITS__)
                .bitwiseOr(parts.m.bitwiseAnd(UInt64(truncating: __MANT_MASK__)))
        } else {
            rawBits = parts.m
        }
        if isNegative {
            rawBits = rawBits.bitwiseOr(UInt64.one.shiftLeft(by: __SIGN_SHIFT__))
        }

        self.raw = __FROM_BITS__;
    }'''

    return (
        method.replace("__SIG_BITS__", str(sig_bits))
        .replace("__MIN_E__", str(min_e))
        .replace("__MAX_E__", str(max_e))
        .replace("__EXP_BIAS_ADD__", str(exp_bias_add))
        .replace("__EXP_MASK__", str(exp_mask))
        .replace("__HIDDEN_BIT__", str(hidden_bit))
        .replace("__MANT_MASK__", str(mant_mask))
        .replace("__SIGN_SHIFT__", str(sign_shift))
        .replace("__FROM_BITS__", from_bits)
        .replace("__TYPE_NAME__", type_name)
        .replace("__LANG_TYPE__", lang_type)
    )


def generate_float_format_method(type_name: str, bits: int) -> str:
    """Generate the format() method for float types.

    Digit generation is delegated to the exact integer/big-integer engine in
    `float_digits.ks` (shared by Float32/Float64). The value is decomposed into
    `m * 2^e` and all decimal digits are produced by big-integer arithmetic, so
    rounding is round-to-nearest-even of the stored binary value — fixing the
    float-arithmetic double-rounding of the old pipeline (#161, #216).
    """
    lang_type = f"f{bits}"

    if bits == 64:
        p = 52
        exp_bits = 11
        bias = 1023
        bits_expr = "UInt64(raw: lang.f64_to_bits(value.raw))"
    else:
        p = 23
        exp_bits = 8
        bias = 127
        bits_expr = "UInt64(from: UInt32(raw: lang.f32_to_bits(value.raw)))"
    exp_mask = (1 << exp_bits) - 1
    mant_mask = (1 << p) - 1
    hidden_bit = 1 << p
    bias_p = bias + p           # normal:  e = rawExp - (bias + p)
    e_sub = 1 - bias - p        # subnormal exponent

    method = '''    /// Formats the float directly into `writer`, honouring the supplied
    /// `FormatOptions`. Implements `Formattable`.
    ///
    /// Digit generation uses the exact big-integer engine in `float_digits.ks`:
    /// the value is decomposed into `m * 2^e` and rounded with round-to-nearest-
    /// even on the stored binary value, so printed decimals are correct.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.14159).formatted();                                          // "3.14159"
    /// (3.0).formatted();                                              // "3.0" (a whole float keeps its ".0")
    /// (3.14159).formatted(.{precision: 2});                  // "3.14"
    /// (1234.5).formatted(.{floatStyle: .Scientific});        // "1.2345e3"
    /// (0.756).formatted(.{floatStyle: .Percent});            // "75.6%"
    /// (3.14).formatted(.{width: 8, fill: '0'});              // "00003.14"
    /// (3.14).formatted(.{sign: .Always});                    // "+3.14"
    /// ```
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        var precision: Int64 = 6;
        var precisionProvided = false;
        if let .Some(p) = options.precision {
            precisionProvided = true;
            if p < 0 {
                precision = 0
            } else {
                precision = p
            }
        }

        var number = String();
        var isNegative = false;
        var allowSign = true;
        var suffixPercent = false;
        var trimTrailingZeros = false;
        var value = self;

        if self.isNaN {
            number = "NaN";
            allowSign = false;
        } else if self.isInfinite {
            number = "Infinity";
            isNegative = self < 0.0;
        } else {
            isNegative = value < 0.0;
            if value.isZero {
                let one = __TYPE_NAME__.one;
                let inverse = one.divide(value);
                if inverse < 0.0 {
                    isNegative = true
                }
            }
            if isNegative {
                value = value.negate()
            }

            var style = options.floatStyle;
            if style == .Percent {
                value = value.multiply(100.0);
                suffixPercent = true;
                style = .Fixed
            }

            // Exact IEEE-754 decomposition of the non-negative `value` into
            // m * 2^e (m an integer significand, e a binary exponent).
            let bits = __BITS_EXPR__;
            let rawExp = Int64(truncating: bits.shiftRight(by: __PSHIFT__).bitwiseAnd(UInt64(truncating: __EXP_MASK__)));
            let rawMant = bits.bitwiseAnd(UInt64(truncating: __MANT_MASK__));
            var m = UInt64.zero;
            var e: Int64 = 0;
            if rawExp == 0 {
                m = rawMant;
                e = __E_SUB__
            } else {
                m = rawMant.bitwiseOr(UInt64(truncating: __HIDDEN_BIT__));
                e = rawExp - __BIAS_P__
            }

            if options.radix == 16 {
                // `:x` / `:X`: exact hexadecimal floating point (C's `%a`).
                number = floatHexString(rawExp, rawMant, __PSHIFT__, __BIAS__, options.uppercase)
            } else if precisionProvided == false and suffixPercent == false {
                // No explicit precision: print the SHORTEST decimal that round-
                // trips back to this exact value (Dragon4 boundary search). The
                // lower gap is a half-ulp only at a binade boundary (fraction
                // zero) above the smallest normal.
                let lowerGapIsHalf = rawMant == UInt64.zero and rawExp >= 2;
                let sr = floatShortestDigits(m, e, lowerGapIsHalf);
                let decExp = sr.decExp;
                var scientific = false;
                if style == .Scientific or style == .ScientificUpper {
                    scientific = true
                } else if style == .Auto and m != UInt64.zero and (decExp < -4 or decExp >= precision) {
                    scientific = true
                };
                if scientific {
                    number = floatShortestSciString(sr, style == .ScientificUpper)
                } else {
                    number = floatShortestFixedString(sr)
                }
            } else {
                // Explicit precision (or percent): exact rounding to that many
                // fractional / mantissa digits.
                // An explicit precision counts decimal places, so `Auto`
                // prints fixed notation (`12.34:.1` is "12.3", not "1.2e1").
                if style == .Auto {
                    style = .Fixed
                }

                if style == .Scientific or style == .ScientificUpper {
                    number = floatSciString(m, e, precision, style == .ScientificUpper)
                } else {
                    number = floatFixedString(m, e, precision)
                }
            }

            if suffixPercent and precisionProvided == false {
                trimTrailingZeros = true
            }
        }

        var result = String();
        if allowSign {
            if isNegative {
                result.append(char: '-')
            } else if options.sign == .Always {
                result.append(char: '+')
            } else if options.sign == .Space {
                result.append(char: ' ')
            }
        }
        if trimTrailingZeros {
            let len = number.bytes.count;
            var dotIndex: Int64 = -1;
            var expIndex: Int64 = -1;
            var i: Int64 = 0;
            while i < len {
                let b = unsafe number.bytes(unchecked: i);
                let v = Int64(from: b);
                if v == 46 {  // '.'
                    dotIndex = i
                } else if v == 101 or v == 69 {  // 'e' or 'E'
                    expIndex = i;
                    break
                }
                i = i + 1
            }

            if dotIndex >= 0 {
                let endIndex: Int64 = if expIndex >= 0 { expIndex } else { len };
                var trimEnd = endIndex;
                while trimEnd > dotIndex + 1 {
                    let b = unsafe number.bytes(unchecked: trimEnd - 1);
                    if Int64(from: b) == 48 {
                        trimEnd = trimEnd - 1
                    } else {
                        break
                    }
                }
                if trimEnd == dotIndex + 1 {
                    trimEnd = dotIndex
                }
                if trimEnd != endIndex {
                    var trimmed = String();
                    if trimEnd > 0 {
                        trimmed.append(number.substringBytes(from: 0, to: trimEnd))
                    }
                    if expIndex >= 0 {
                        trimmed.append(number.substringBytes(from: expIndex, to: len))
                    }
                    number = trimmed
                }
            }
        }

        result.append(number);
        if suffixPercent {
            result.append(char: '%')
        }

        _writePadded(into: writer, result, options)
    }'''

    return (
        method.replace("__BITS_EXPR__", bits_expr)
        .replace("__PSHIFT__", str(p))
        .replace("__EXP_MASK__", str(exp_mask))
        .replace("__MANT_MASK__", str(mant_mask))
        .replace("__HIDDEN_BIT__", str(hidden_bit))
        .replace("__BIAS_P__", str(bias_p))
        .replace("__BIAS__", str(bias))
        .replace("__E_SUB__", str(e_sub))
        .replace("__TYPE_NAME__", type_name)
        .replace("__LANG_TYPE__", lang_type)
    )


# Mantissa bits (including the implicit one): an integer of at most this
# many magnitude bits converts exactly.
FLOAT_MANTISSA = {64: 53, 32: 24}


def float_conversions(type_name: str, bits: int) -> tuple[str, str]:
    """`Convertible` conformances and inits for a float: the identity, the
    other float when it widens, and every integer whose values all fit the
    mantissa. Everything else converts with `init(rounding:)`, which says it
    may lose low-order bits."""
    lang_type = f"f{bits}"
    other_float, other_bits = ("Float32", 32) if bits == 64 else ("Float64", 64)
    mantissa = FLOAT_MANTISSA[bits]
    exact, rounded = [], []
    for name, ibits, signed, _ in INTEGERS:
        magnitude = ibits - 1 if signed else ibits
        if magnitude <= mantissa:
            exact.append((name, ibits, signed))
        elif signed or ibits < 64:
            # No `UInt64` source: there is no unsigned 64-bit cast to widen
            # through. (None existed before `Convertible` went lossless.)
            rounded.append((name, ibits, signed))

    def int_cast(name: str, ibits: int, signed: bool) -> str:
        # Signed sources up to 64 bits have a direct cast; unsigned ones
        # widen to `Int64` first (lossless below 64 bits).
        if signed:
            return f"lang.cast_i{ibits}_{lang_type}(value.raw)"
        return f"lang.cast_i64_{lang_type}(Int64(from: value).raw)"

    conformances = [type_name] + [n for n, _, _ in exact]
    out = [
        f"""    /// @name Identity
    /// `{type_name}` from itself, so a `Convertible[T]` bound accepts `{type_name}`.
    public init(from value: {type_name}) {{
        self.raw = value.raw
    }}"""
    ]
    for name, ibits, signed in exact:
        out.append(f"""    /// @name From Integer
    /// Converts from `{name}`. Exact: every `{name}` fits `{type_name}`'s mantissa.
    ///
    /// # Examples
    ///
    /// ```
    /// let n: {name} = 42;
    /// let f = {type_name}(from: n);  // 42.0
    /// ```
    public init(from value: {name}) {{
        self.raw = {int_cast(name, ibits, signed)}
    }}""")
    for name, ibits, signed in rounded:
        out.append(f"""    /// @name Rounding Integer
    /// Converts from `{name}`, rounding to the nearest `{type_name}`: values
    /// with magnitude above 2^{mantissa} lose low-order bits. Not `init(from:)`,
    /// which is reserved for exact conversions.
    public init(rounding value: {name}) {{
        self.raw = {int_cast(name, ibits, signed)}
    }}""")
    if other_bits < bits:
        conformances.append(other_float)
        out.append(f"""    /// @name From Float
    /// Converts from `{other_float}`. Exact: every `{other_float}` is a `{type_name}`.
    public init(from value: {other_float}) {{
        self.raw = lang.cast_f{other_bits}_{lang_type}(value.raw)
    }}""")
    else:
        out.append(f"""    /// @name Rounding Float
    /// Converts from `{other_float}`, rounding to the nearest `{type_name}`;
    /// values beyond `{type_name}.maxValue` become ±infinity.
    public init(rounding value: {other_float}) {{
        self.raw = lang.cast_f{other_bits}_{lang_type}(value.raw)
    }}""")
    # `Numeric.init(exactly: Int64)?`: generic code's count-to-`T`. Strict,
    # like the integers' `exactly:` — `nil` when the value doesn't round-trip.
    out.append(f"""    /// @name Exactly
    /// Converts from `Int64` when `{type_name}` holds the value exactly, and
    /// is `nil` when it would round (magnitudes above 2^{mantissa} that aren't
    /// representable). Satisfies `Numeric`, so generic code can write
    /// `T(exactly: xs.count)!`.
    public init(exactly value: Int64)? {{
        let rounded = {type_name}(rounding: value);
        if Int64(exactly: rounded) != value {{
            return null
        }}
        self.raw = rounded.raw
    }}""")
    conf = "".join(f"    Convertible[{n}],\n" for n in conformances)
    rounding = [n for n, _, _ in rounded] + ([] if other_bits < bits else [other_float])
    conf += "".join(f"    RoundsFrom[{n}],\n" for n in rounding)
    return conf, "\n\n".join(out) + "\n"


def generate_float(type_name: str, bits: int, is_default: bool) -> str:
    template = (SCRIPT_DIR / "float.ks.template").read_text()

    lang_type = f"f{bits}"
    other_float = "Float32" if bits == 64 else "Float64"
    other_lang_type = "f32" if bits == 64 else "f64"
    uint_type = "UInt64" if bits == 64 else "UInt32"

    # Float literal init - need to cast from f64 for f32
    if bits == 64:
        float_literal_init = "self.raw = value"
        zero_literal = "0.0"
    else:
        float_literal_init = f"self.raw = lang.cast_f64_f{bits}(value)"
        zero_literal = "0.0"  # Will be cast by the literal protocol

    # Type alias for platform default
    if is_default:
        type_alias = f"""

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Default floating-point type — alias for `{type_name}`. Reach for `Float`
/// when you want the recommended precision/performance trade-off; reach for
/// `Float32` only when you specifically need 32-bit storage.
public type Float = {type_name}"""
    else:
        type_alias = ""

    # Generate format method
    format_method = generate_float_format_method(type_name, bits)

    # Generate parse method
    parse_method = generate_float_parse_method(type_name, bits)

    # Float constants - use literal values since intrinsics don't exist
    # Note: negative constants need special handling to avoid -literal being parsed as negate()
    if bits == 64:
        # min_value is -(max_value) - we'll construct it differently in the template
        max_value = "1.7976931348623157e308"
        min_value = "-1.7976931348623157e308"
        min_positive = "2.2250738585072014e-308"
        epsilon = "2.220446049250313e-16"
        precision_kind = "double-precision"
        precision_kind_header = "double precision"
        range_approx = "1.8×10^308"
        sig_digits = "15-17"
    else:
        max_value = "3.4028235e38"
        min_value = "-3.4028235e38"
        min_positive = "1.17549435e-38"
        epsilon = "1.1920929e-7"
        precision_kind = "single-precision"
        precision_kind_header = "single precision"
        range_approx = "3.4×10^38"
        sig_digits = "6-9"

    default_float = "Float64"
    type_name_upper = type_name.upper()

    # Conversion to other float type
    if bits == 64:
        to_other_float = f"Float32(raw: lang.cast_f64_f32(self.raw))"
        libm_suffix = ""  # f64 functions: sin, cos, etc.
    else:
        to_other_float = f"Float64(raw: lang.cast_f32_f64(self.raw))"
        libm_suffix = "f"  # f32 functions: sinf, cosf, etc.

    result = template
    result = result.replace("{{TYPE_NAME}}", type_name)
    result = result.replace("{{TYPE_NAME_UPPER}}", type_name_upper)
    result = result.replace("{{BITS}}", str(bits))
    result = result.replace("{{LANG_TYPE}}", lang_type)
    result = result.replace("{{OTHER_FLOAT}}", other_float)
    result = result.replace("{{OTHER_LANG_TYPE}}", other_lang_type)
    result = result.replace("{{UINT_TYPE}}", uint_type)
    result = result.replace("{{FLOAT_LITERAL_INIT}}", float_literal_init)
    result = result.replace("{{ZERO_LITERAL}}", zero_literal)
    result = result.replace("{{TYPE_ALIAS}}", type_alias)
    result = result.replace("{{FORMAT_METHOD}}", format_method)
    result = result.replace("{{MIN_VALUE}}", min_value)
    result = result.replace("{{MAX_VALUE}}", max_value)
    result = result.replace("{{MIN_POSITIVE}}", min_positive)
    result = result.replace("{{EPSILON}}", epsilon)
    result = result.replace("{{PRECISION_KIND}}", precision_kind)
    result = result.replace("{{PRECISION_KIND_HEADER}}", precision_kind_header)
    result = result.replace("{{RANGE_APPROX}}", range_approx)
    result = result.replace("{{SIG_DIGITS}}", sig_digits)
    result = result.replace("{{DEFAULT_FLOAT}}", default_float)
    result = result.replace("{{TO_OTHER_FLOAT}}", to_other_float)
    result = result.replace("{{LIBM_SUFFIX}}", libm_suffix)
    result = result.replace("{{PARSE_METHOD}}", parse_method)
    float_conformances, float_inits = float_conversions(type_name, bits)
    result = result.replace("{{FLOAT_CONFORMANCES}}", float_conformances)
    result = result.replace("{{FLOAT_CONVERSIONS}}", float_inits)

    return result


def main():
    # Generate integer types
    for type_name, bits, signed, is_default in INTEGERS:
        filename = f"{type_name.lower()}.ks"
        content = generate_integer(type_name, bits, signed, is_default)
        output_path = SCRIPT_DIR / filename
        output_path.write_text(content)
        print(f"Generated {filename}")

    # Generate float types
    for type_name, bits, is_default in FLOATS:
        filename = f"{type_name.lower()}.ks"
        content = generate_float(type_name, bits, is_default)
        output_path = SCRIPT_DIR / filename
        output_path.write_text(content)
        print(f"Generated {filename}")

    print(f"\nGenerated {len(INTEGERS)} integer types and {len(FLOATS)} float types")


if __name__ == "__main__":
    main()
