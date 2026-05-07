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


def generate_byte_swap(type_name: str, bits: int, lang_type: str) -> str:
    """Generate byteSwapped implementation using bswap intrinsic."""
    if bits == 8:
        # Byte swap on 8-bit is a no-op
        return "self"
    else:
        return f"{type_name}(raw: lang.{lang_type}_bswap(self.raw))"


def generate_checked_arithmetic(type_name: str, bits: int, signed: bool, lang_type: str) -> str:
    """Generate checked arithmetic methods that return Optional."""
    if signed:
        return f'''    // TODO: requires overflow-detecting intrinsics for proper implementation
    /// Wrapping addition that returns `None` instead of overflowing.
    public func addChecked(other: {type_name}) -> {type_name}? {{
        // Simplified check - detect if signs are same and result sign differs
        let result = self.add(other);
        if self.isPositive and other.isPositive and result.isNegative {{
            return .None
        }};
        if self.isNegative and other.isNegative and result.isPositive {{
            return .None
        }};
        .Some(result)
    }}

    /// Wrapping subtraction that returns `None` instead of overflowing.
    public func subtractChecked(other: {type_name}) -> {type_name}? {{
        // Simplified check
        let result = self.subtract(other);
        if self.isPositive and other.isNegative and result.isNegative {{
            return .None
        }};
        if self.isNegative and other.isPositive and result.isPositive {{
            return .None
        }};
        .Some(result)
    }}

    /// Wrapping multiplication that returns `None` instead of overflowing.
    /// Implemented by multiplying then dividing back; replace with an
    /// overflow-detecting intrinsic when one is available.
    public func multiplyChecked(other: {type_name}) -> {type_name}? {{
        if other == {type_name}.zero {{
            return .Some({type_name}.zero)
        }};
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {{
            return .None
        }};
        .Some(result)
    }}

    /// Division that returns `None` for divide-by-zero or for the
    /// `minValue / -1` overflow case.
    public func divideChecked(other: {type_name}) -> {type_name}? {{
        if other == {type_name}.zero {{
            return .None
        }};
        // Check for minValue / -1 overflow
        if self == {type_name}.minValue and other == {type_name}(intLiteral: lang.i64_neg(1)) {{
            return .None
        }};
        .Some(self.divide(other))
    }}

    /// Negation that returns `None` for `minValue` (whose negation overflows).
    public func negateChecked() -> {type_name}? {{
        if self == {type_name}.minValue {{
            return .None
        }};
        .Some(self.negate())
    }}

    /// Absolute value that returns `None` for `minValue` (whose absolute
    /// value overflows).
    public func absChecked() -> {type_name}? {{
        if self == {type_name}.minValue {{
            return .None
        }};
        .Some(self.abs())
    }}

'''
    else:
        return f'''    // TODO: requires overflow-detecting intrinsics for proper implementation
    /// Wrapping addition that returns `None` on overflow. For unsigned types
    /// overflow is detected via `result < self`.
    public func addChecked(other: {type_name}) -> {type_name}? {{
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {{
            return .None
        }};
        .Some(result)
    }}

    /// Subtraction that returns `None` on underflow (`other > self`).
    public func subtractChecked(other: {type_name}) -> {type_name}? {{
        // For unsigned, underflow if other > self
        if other > self {{
            return .None
        }};
        .Some(self.subtract(other))
    }}

    /// Wrapping multiplication that returns `None` on overflow. Implemented
    /// by multiplying then dividing back.
    public func multiplyChecked(other: {type_name}) -> {type_name}? {{
        if other == {type_name}.zero {{
            return .Some({type_name}.zero)
        }};
        let result = self.multiply(other);
        // Check by dividing back
        if result.divide(other) != self {{
            return .None
        }};
        .Some(result)
    }}

    /// Division that returns `None` for divide-by-zero.
    public func divideChecked(other: {type_name}) -> {type_name}? {{
        if other == {type_name}.zero {{
            return .None
        }};
        .Some(self.divide(other))
    }}

'''


def generate_saturating_arithmetic(type_name: str, bits: int, signed: bool, lang_type: str) -> str:
    """Generate saturating arithmetic methods."""
    if signed:
        return f'''    /// Addition that clamps to `maxValue`/`minValue` instead of wrapping.
    public func addSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.addChecked(other);
        match checked {{
            .Some(result) => result,
            .None => if other.isPositive {{ {type_name}.maxValue }} else {{ {type_name}.minValue }}
        }}
    }}

    /// Subtraction that clamps to `maxValue`/`minValue` instead of wrapping.
    public func subtractSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.subtractChecked(other);
        match checked {{
            .Some(result) => result,
            .None => if other.isNegative {{ {type_name}.maxValue }} else {{ {type_name}.minValue }}
        }}
    }}

    /// Multiplication that clamps to `maxValue`/`minValue` instead of wrapping.
    /// The clamp direction follows the algebraic sign of the would-be result.
    public func multiplySaturating(other: {type_name}) -> {type_name} {{
        let checked = self.multiplyChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {{
                // Determine sign of result
                let sameSign = (self.isNegative == other.isNegative);
                if sameSign {{ {type_name}.maxValue }} else {{ {type_name}.minValue }}
            }}
        }}
    }}

    /// Negation that returns `maxValue` instead of wrapping `minValue`.
    public func negateSaturating() -> {type_name} {{
        if self == {type_name}.minValue {{
            {type_name}.maxValue
        }} else {{
            self.negate()
        }}
    }}

    /// Absolute value that returns `maxValue` instead of wrapping `minValue`.
    public func absSaturating() -> {type_name} {{
        if self == {type_name}.minValue {{
            {type_name}.maxValue
        }} else {{
            self.abs()
        }}
    }}

'''
    else:
        return f'''    /// Addition that clamps to `maxValue` on overflow.
    public func addSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.addChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {type_name}.maxValue
        }}
    }}

    /// Subtraction that clamps to `0` on underflow (unsigned types cannot
    /// represent negative results).
    public func subtractSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.subtractChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {type_name}.zero
        }}
    }}

    /// Multiplication that clamps to `maxValue` on overflow.
    public func multiplySaturating(other: {type_name}) -> {type_name} {{
        let checked = self.multiplyChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {type_name}.maxValue
        }}
    }}

'''


def generate_integer_format_method(type_name: str, bits: int, signed: bool) -> str:
    """Generate the format(into:) method for integer types."""

    # For converting values between types
    if bits == 64 and signed:
        digit_as_i64 = "digit"
        radix_as_type = "radix"
    elif bits == 64:
        digit_as_i64 = "Int64(from: digit)"
        radix_as_type = "UInt64(from: radix)"
    else:
        digit_as_i64 = f"Int64(from: digit)"
        radix_as_type = f"{type_name}(from: radix)"

    if signed:
        sign_handling = f'''
        let isNegative = n < 0;
        if isNegative {{
            n = n.negate()
        }}'''
        sign_prefix = '''
        if isNegative {
            result.appendChar('-')
        } else if options.sign == .Always {
            result.appendChar('+')
        } else if options.sign == .Space {
            result.appendChar(' ')
        }'''
    else:
        sign_handling = '''
        let isNegative = false;'''
        sign_prefix = '''
        if options.sign == .Always {
            result.appendChar('+')
        } else if options.sign == .Space {
            result.appendChar(' ')
        }'''

    return f'''    // Formattable
    /// Formats the integer directly into `writer`, honouring the supplied
    /// `FormatOptions`. Implements the `Formattable` protocol.
    ///
    /// # Examples
    ///
    /// ```
    /// (42).format();                                           // "42"
    /// (255).format(.{{radix: 16}});                     // "ff"
    /// (255).format(.{{radix: 16, uppercase: true}});    // "FF"
    /// (255).format(.{{radix: 16, alternate: true}});    // "0xff"
    /// (42).format(.{{radix: 2, alternate: true}});      // "0b101010"
    /// (42).format(.{{width: .Some(5), fill: '0'}});     // "00042"
    /// (-42).format(.{{sign: .Always}});                 // "-42"
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {{
        var n = self;{sign_handling}

        var radix: Int64 = options.radix;
        if radix < 2 or radix > 36 {{
            radix = 10
        }}

        // Build digits in reverse order (need random access to reverse)
        var digits = String();
        if n == {type_name}.zero {{
            digits.appendByte(48)
        }} else {{
            let radixVal: {type_name} = {radix_as_type};
            while n != {type_name}.zero {{
                let digit: {type_name} = n % radixVal;
                let digitVal: Int64 = {digit_as_i64};
                let charCode: Int64 = if digitVal < 10 {{
                    digitVal + 48
                }} else if options.uppercase {{
                    digitVal - 10 + 65
                }} else {{
                    digitVal - 10 + 97
                }};
                digits.appendByte(UInt8(from: charCode));
                n = n / radixVal
            }}
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

        var i = digits.byteCount - 1;
        while i >= 0 {{
            result.appendByte(digits.bytes(unchecked: i));
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
        return_expr = f"{type_name}(from: result)"
        if signed:
            # Convert smaller type's bounds to Int64 for comparison
            min_val_expr = f"Int64(from: {type_name}.minValue)"
            max_val_expr = f"Int64(from: {type_name}.maxValue)"
        else:
            min_val_expr = "0"
            max_val_expr = f"UInt64(from: {type_name}.maxValue)"

    # For signed types, handle negative numbers
    if signed:
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
    public init(parsing string: String)? {{
        let len = string.byteCount;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = string.bytes(unchecked: 0);
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

        var result: Int64 = 0;
        let maxBeforeMultiply: Int64 = 922337203685477580;

        while index < len {{
            let byte: UInt8 = string.bytes(unchecked: index);
            let byteVal = Int64(from: byte);

            if byteVal < 48 or byteVal > 57 {{
                return null
            }}

            let digit = byteVal - 48;

            if result > maxBeforeMultiply {{
                return null
            }}
            result = result * 10;

            if result > 9223372036854775807 - digit {{
                return null
            }}
            result = result + digit;

            index = index + 1
        }}

        if isNegative {{
            result = result.negate();
            if result < {min_val_expr} {{
                return null
            }}
        }} else {{
            if result > {max_val_expr} {{
                return null
            }}
        }}

        self.raw = {return_expr}.raw;
    }}'''
        # Per-type magnitude bounds for the UInt64 accumulator.
        if type_name == "Int64":
            pos_max_expr = "UInt64(from: Int64.maxValue)"
            neg_max_expr = "UInt64(from: Int64.maxValue) + 1"
        else:
            pos_max_expr = f"UInt64(from: {type_name}.maxValue)"
            neg_max_expr = f"UInt64(from: {type_name}.maxValue) + 1"
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
    public init(parsing string: String, radix radix: Int64)? {{
        if radix < 2 or radix > 36 {{
            return null
        }}

        let len = string.byteCount;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;
        var isNegative = false;

        let firstByte: UInt8 = string.bytes(unchecked: 0);
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

        let radixU: UInt64 = UInt64(from: radix);
        let maxMagnitude: UInt64 = if isNegative {{
            {neg_max_expr}
        }} else {{
            {pos_max_expr}
        }};

        var result: UInt64 = 0;

        while index < len {{
            let byte: UInt8 = string.bytes(unchecked: index);
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

            let digitU: UInt64 = UInt64(from: digit);
            if result > (maxMagnitude - digitU) / radixU {{
                return null
            }}
            result = result * radixU + digitU;
            index = index + 1
        }}

        let typedResult = {type_name}(from: result);
        if isNegative {{
            self.raw = typedResult.negate().raw
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
    public init(parsing string: String)? {{
        let len = string.byteCount;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;

        let firstByte: UInt8 = string.bytes(unchecked: 0);
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
            let byte: UInt8 = string.bytes(unchecked: index);
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
    public init(parsing string: String, radix radix: Int64)? {{
        if radix < 2 or radix > 36 {{
            return null
        }}

        let len = string.byteCount;
        if len == 0 {{
            return null
        }}

        var index: Int64 = 0;

        let firstByte: UInt8 = string.bytes(unchecked: 0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 43 {{
            index = 1
        }} else if firstByteVal == 45 {{
            return null
        }}

        if index >= len {{
            return null
        }}

        let radixU: UInt64 = UInt64(from: radix);
        let maxVal: UInt64 = {max_expr};

        var result: UInt64 = 0;

        while index < len {{
            let byte: UInt8 = string.bytes(unchecked: index);
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

            let digitU: UInt64 = UInt64(from: digit);
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
    widen_self = "self" if type_name == "UInt64" else "UInt64(from: self)"
    narrow_result = "result" if type_name == "UInt64" else f"{type_name}(from: result)"

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
            result.append(ptr.offset(by: i).read());
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
            result.append(UInt8(from: byteVal));
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
            result.append(UInt8(from: byteVal));
            i = i + 1
        }}
        result
    }}

    /// @name From Bytes
    /// Reassembles a `{type_name}` from {byte_count} bytes in native byte order.
    /// Returns `null` if the input is not exactly {byte_count} bytes long.
    public init[S](fromBytes fromBytes: S)? where S: Slice[UInt8] {{
        let bytes = fromBytes.asSlice();
        if bytes.count != {bc} {{
            return null
        }}
        var value = {type_name}.zero;
        let ptr = Pointer(to: value).asRaw().cast[UInt8]();
        var i: Int64 = 0;
        while i < {bc} {{
            ptr.offset(by: i).write(bytes(unchecked: i));
            i = i + 1
        }}
        self.raw = value.raw;
    }}

    /// @name From Bytes Big Endian
    /// Reassembles a `{type_name}` from {byte_count} bytes in big-endian order.
    /// Returns `null` if the input is not exactly {byte_count} bytes long.
    public init[S](fromBytesBigEndian fromBytesBigEndian: S)? where S: Slice[UInt8] {{
        let bytes = fromBytesBigEndian.asSlice();
        if bytes.count != {bc} {{
            return null
        }}
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < {bc} {{
            let byteVal = UInt64(from: bytes(unchecked: i));
            result = (result << 8) | byteVal;
            i = i + 1
        }}
        self.raw = {narrow_result}.raw;
    }}

    /// @name From Bytes Little Endian
    /// Reassembles a `{type_name}` from {byte_count} bytes in little-endian order.
    /// Returns `null` if the input is not exactly {byte_count} bytes long.
    public init[S](fromBytesLittleEndian fromBytesLittleEndian: S)? where S: Slice[UInt8] {{
        let bytes = fromBytesLittleEndian.asSlice();
        if bytes.count != {bc} {{
            return null
        }}
        var result: UInt64 = 0;
        var i: Int64 = 0;
        while i < {bc} {{
            let shift = i * 8;
            let byteVal = UInt64(from: bytes(unchecked: i));
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

    conformances = []
    inits = []
    for other_name, other_bits, other_signed in other_types:
        conformances.append(f"    Convertible[{other_name}]")
        cast_expr = get_cast(other_bits, bits, other_signed)
        inits.append(
            f"    /// @name From Integer\n"
            f"    /// Converts from `{other_name}`. Narrowing conversions truncate the high\n"
            f"    /// bits; signed→unsigned reinterprets the bit pattern.\n"
            f"    public init(from other: {other_name}) {{ self.raw = {cast_expr} }}"
        )

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
        negate_method = f"""/// Two's-complement negation. Wraps at the minimum value:
    /// `{type_name}.minValue.negate() == {type_name}.minValue`. Use
    /// `negateChecked` to surface the overflow.
    public func negate() -> {type_name} {{ {type_name}(raw: lang.{lang_type}_neg(self.raw)) }}"""
        abs_method = f"""/// Absolute value. Wraps at the minimum value
    /// (`{type_name}.minValue.abs() == {type_name}.minValue`); use
    /// `absChecked` if that's a problem.
    public func abs() -> {type_name} {{ if Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 0)) {{ self.negate() }} else {{ self }} }}"""
        # For signed, compute minValue via shift left: 1 << (bits - 1)
        # This avoids literal overflow issues with large values like 9223372036854775808
        min_value_expr = f"{type_name}(raw: lang.{lang_type}_shl(1, {bits - 1}))"
        gcd_abs_self = "self.abs()"
        gcd_abs_other = "other.abs()"
        lcm_abs_self = "self.abs()"
        lcm_abs_other = "other.abs()"
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
        negate_method = ""
        abs_method = ""
        min_value_expr = f"{type_name}(intLiteral: 0)"
        gcd_abs_self = "self"
        gcd_abs_other = "other"
        lcm_abs_self = "self"
        lcm_abs_other = "other"
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

    # Generate bit counting operations using intrinsics
    count_ones_impl = generate_count_ones(type_name, bits, lang_type)
    leading_zeros_impl = generate_leading_zeros(type_name, bits, lang_type)
    trailing_zeros_impl = generate_trailing_zeros(type_name, bits, lang_type)

    # Generate byte swap using intrinsic
    byte_swap_impl = generate_byte_swap(type_name, bits, lang_type)

    # Generate checked arithmetic
    checked_arithmetic = generate_checked_arithmetic(type_name, bits, signed, lang_type)

    # Generate saturating arithmetic
    saturating_arithmetic = generate_saturating_arithmetic(type_name, bits, signed, lang_type)

    # Generate parse method
    parse_method = generate_integer_parse_method(type_name, bits, signed)

    result = template
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
    result = result.replace("{{NEGATE_METHOD}}", negate_method)
    result = result.replace("{{ABS_METHOD}}", abs_method)
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
    result = result.replace("{{COUNT_ONES_IMPL}}", count_ones_impl)
    result = result.replace("{{LEADING_ZEROS_IMPL}}", leading_zeros_impl)
    result = result.replace("{{TRAILING_ZEROS_IMPL}}", trailing_zeros_impl)
    result = result.replace("{{BYTE_SWAP_IMPL}}", byte_swap_impl)
    result = result.replace("{{CHECKED_ARITHMETIC}}", checked_arithmetic)
    result = result.replace("{{SATURATING_ARITHMETIC}}", saturating_arithmetic)
    result = result.replace("{{GCD_ABS_SELF}}", gcd_abs_self)
    result = result.replace("{{GCD_ABS_OTHER}}", gcd_abs_other)
    result = result.replace("{{LCM_ABS_SELF}}", lcm_abs_self)
    result = result.replace("{{LCM_ABS_OTHER}}", lcm_abs_other)
    result = result.replace("{{PARSE_METHOD}}", parse_method)

    interface_path = DOCS_DIR / f"{type_name.lower()}.ks.interface"
    if interface_path.exists():
        docs_map = extract_interface_docs(interface_path)
        result = apply_interface_docs(result, docs_map)

    return result


def generate_float_parse_method(type_name: str, bits: int) -> str:
    """Generate the parse() method for float types."""
    lang_type = f"f{bits}"

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
    public init(parsing string: String)? {
        let len = string.byteCount;
        if len == 0 {
            return null
        }

        if len == 3 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let isN0 = Int64(from: b0) == 110 or Int64(from: b0) == 78;
            let isA1 = Int64(from: b1) == 97 or Int64(from: b1) == 65;
            let isN2 = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            if isN0 and isA1 and isN2 {
                self.raw = lang.__LANG_TYPE___nan();
                return
            }
        }

        if len == 3 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let isI = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            if isI and isN and isF {
                self.raw = lang.__LANG_TYPE___infinity();
                return
            }
        }

        if len == 4 {
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let b3: UInt8 = string.bytes(unchecked: 3);
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
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let b3: UInt8 = string.bytes(unchecked: 3);
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
            let b0: UInt8 = string.bytes(unchecked: 0);
            let b1: UInt8 = string.bytes(unchecked: 1);
            let b2: UInt8 = string.bytes(unchecked: 2);
            let b3: UInt8 = string.bytes(unchecked: 3);
            let b4: UInt8 = string.bytes(unchecked: 4);
            let b5: UInt8 = string.bytes(unchecked: 5);
            let b6: UInt8 = string.bytes(unchecked: 6);
            let b7: UInt8 = string.bytes(unchecked: 7);
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

        let firstByte: UInt8 = string.bytes(unchecked: 0);
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

        var integerPart: __TYPE_NAME__ = 0.0;
        var hasIntegerPart = false;
        var currentByte: Int64 = Int64(from: string.bytes(unchecked: index));

        while index < len and currentByte >= 48 and currentByte <= 57 {
            let digit = __TYPE_NAME__(from: currentByte - 48);
            integerPart = integerPart * 10.0 + digit;
            hasIntegerPart = true;
            index = index + 1;
            if index < len {
                currentByte = Int64(from: string.bytes(unchecked: index))
            }
        }

        var fractionalPart: __TYPE_NAME__ = 0.0;
        var hasFractionalPart = false;

        if index < len and currentByte == 46 {
            index = index + 1;
            var divisor: __TYPE_NAME__ = 10.0;

            if index < len {
                currentByte = Int64(from: string.bytes(unchecked: index));
                while index < len and currentByte >= 48 and currentByte <= 57 {
                    let digit = __TYPE_NAME__(from: currentByte - 48);
                    fractionalPart = fractionalPart + digit / divisor;
                    divisor = divisor * 10.0;
                    hasFractionalPart = true;
                    index = index + 1;
                    if index < len {
                        currentByte = Int64(from: string.bytes(unchecked: index))
                    }
                }
            }
        }

        if not hasIntegerPart and not hasFractionalPart {
            return null
        }

        var result = integerPart + fractionalPart;

        if index < len and (currentByte == 101 or currentByte == 69) {
            index = index + 1;

            if index >= len {
                return null
            }

            var expNegative = false;
            currentByte = Int64(from: string.bytes(unchecked: index));

            if currentByte == 45 {
                expNegative = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.bytes(unchecked: index))
                }
            } else if currentByte == 43 {
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.bytes(unchecked: index))
                }
            }

            if index >= len {
                return null
            }

            var exponent: Int64 = 0;
            var hasExpDigit = false;

            while index < len and currentByte >= 48 and currentByte <= 57 {
                exponent = exponent * 10 + (currentByte - 48);
                hasExpDigit = true;
                index = index + 1;
                if index < len {
                    currentByte = Int64(from: string.bytes(unchecked: index))
                }
            }

            if not hasExpDigit {
                return null
            }

            let expFloat = __TYPE_NAME__(from: exponent);
            let ten: __TYPE_NAME__ = 10.0;
            if expNegative {
                result = result / ten.pow(expFloat)
            } else {
                result = result * ten.pow(expFloat)
            }
        }

        if index != len {
            return null
        }

        if isNegative {
            result = result.negate()
        }

        self.raw = result.raw;
    }'''

    return method.replace("__TYPE_NAME__", type_name).replace("__LANG_TYPE__", lang_type)


def generate_float_format_method(type_name: str, bits: int) -> str:
    """Generate the format() method for float types."""
    lang_type = f"f{bits}"

    method = '''    /// Formats the float directly into `writer`, honouring the supplied
    /// `FormatOptions`. Implements `Formattable`.
    ///
    /// # Examples
    ///
    /// ```
    /// (3.14159).format();                                          // "3.14159"
    /// (3.14159).format(.{precision: 2});                  // "3.14"
    /// (1234.5).format(.{floatStyle: .Scientific});        // "1.2345e3"
    /// (0.756).format(.{floatStyle: .Percent});            // "75.6%"
    /// (3.14).format(.{width: 8, fill: '0'});              // "00003.14"
    /// (3.14).format(.{sign: .Always});                    // "+3.14"
    /// ```
    public func format(mutating into writer: StringBuilder, options: FormatOptions = FormatOptions.default()) {
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

            if style == .Auto {
                if precisionProvided == false {
                    trimTrailingZeros = true
                }
                if value.isZero {
                    style = .Fixed
                } else {
                    let expVal = value.log10().floor();
                    let expInt: Int64 = Int64(raw: lang.cast___LANG_TYPE___i64(expVal.raw));
                    if expInt < -4 or expInt >= precision {
                        style = .Scientific
                    } else {
                        style = .Fixed
                    }
                }
            }

            if style == .Scientific or style == .ScientificUpper {
                var exponent: Int64 = 0;
                var mantissa = value;
                if value.isZero == false {
                    let expVal = value.log10().floor();
                    exponent = Int64(raw: lang.cast___LANG_TYPE___i64(expVal.raw));
                    let pow10 = __TYPE_NAME__(floatLiteral: 10.0).powi(exponent);
                    mantissa = value.divide(pow10);
                }

                let scale = __TYPE_NAME__(floatLiteral: 10.0).powi(precision);
                mantissa = mantissa.multiply(scale).round().divide(scale);
                if mantissa >= 10.0 {
                    mantissa = mantissa.divide(10.0);
                    exponent = exponent + 1
                }

                let intPart = mantissa.trunc();
                var intVal: Int64 = Int64(raw: lang.cast___LANG_TYPE___i64(intPart.raw));

                if intVal == 0 {
                    number.appendByte(48)
                } else {
                    var digits = String();
                    while intVal > 0 {
                        let digit: Int64 = intVal % 10;
                        let charCode: Int64 = digit + 48;
                        digits.appendByte(UInt8(from: charCode));
                        intVal = intVal / 10
                    }
                    var i = digits.byteCount - 1;
                    while i >= 0 {
                        number.appendByte(digits.bytes(unchecked: i));
                        i = i - 1
                    }
                }

                if precision > 0 {
                    number.appendByte(46);
                    var fracPart = mantissa - intPart;
                    var digitCount: Int64 = 0;
                    let ten: __TYPE_NAME__ = 10.0;
                    while digitCount < precision {
                        fracPart = fracPart * ten;
                        let digit: Int64 = Int64(raw: lang.cast___LANG_TYPE___i64(fracPart.trunc().raw));
                        let charCode: Int64 = digit + 48;
                        number.appendByte(UInt8(from: charCode));
                        fracPart = fracPart - __TYPE_NAME__(raw: lang.cast_i64___LANG_TYPE__(digit.raw));
                        digitCount = digitCount + 1
                    }
                }

                if style == .ScientificUpper {
                    number.appendByte(69)  // 'E'
                } else {
                    number.appendByte(101)  // 'e'
                }

                var expVal: Int64 = exponent;
                if expVal < 0 {
                    number.appendByte(45);  // '-'
                    expVal = expVal.negate()
                }
                if expVal == 0 {
                    number.appendByte(48)  // '0'
                } else {
                    var digits = String();
                    while expVal > 0 {
                        let digit: Int64 = expVal % 10;
                        let charCode: Int64 = digit + 48;
                        digits.appendByte(UInt8(from: charCode));
                        expVal = expVal / 10
                    }
                    var i = digits.byteCount - 1;
                    while i >= 0 {
                        number.appendByte(digits.bytes(unchecked: i));
                        i = i - 1
                    }
                }
            } else {
                let scale = if precision > 0 {
                    __TYPE_NAME__(floatLiteral: 10.0).powi(precision)
                } else {
                    __TYPE_NAME__(floatLiteral: 1.0)
                };

                var rounded = value;
                if precision >= 0 {
                    rounded = rounded.multiply(scale).round().divide(scale)
                }

                let intPart = rounded.trunc();
                var intVal: Int64 = Int64(raw: lang.cast___LANG_TYPE___i64(intPart.raw));

                if intVal == 0 {
                    number.appendByte(48)
                } else {
                    var digits = String();
                    while intVal > 0 {
                        let digit: Int64 = intVal % 10;
                        let charCode: Int64 = digit + 48;
                        digits.appendByte(UInt8(from: charCode));
                        intVal = intVal / 10
                    }
                    var i = digits.byteCount - 1;
                    while i >= 0 {
                        number.appendByte(digits.bytes(unchecked: i));
                        i = i - 1
                    }
                }

                if precision > 0 {
                    number.appendByte(46);
                    var fracPart = rounded - intPart;
                    var digitCount: Int64 = 0;
                    let ten: __TYPE_NAME__ = 10.0;
                    while digitCount < precision {
                        fracPart = fracPart * ten;
                        let digit: Int64 = Int64(raw: lang.cast___LANG_TYPE___i64(fracPart.trunc().raw));
                        let charCode: Int64 = digit + 48;
                        number.appendByte(UInt8(from: charCode));
                        fracPart = fracPart - __TYPE_NAME__(raw: lang.cast_i64___LANG_TYPE__(digit.raw));
                        digitCount = digitCount + 1
                    }
                }
            }

            if suffixPercent and precisionProvided == false {
                trimTrailingZeros = true
            }
        }

        var result = String();
        if allowSign {
            if isNegative {
                result.appendChar('-')
            } else if options.sign == .Always {
                result.appendChar('+')
            } else if options.sign == .Space {
                result.appendChar(' ')
            }
        }
        if trimTrailingZeros {
            let len = number.byteCount;
            var dotIndex: Int64 = -1;
            var expIndex: Int64 = -1;
            var i: Int64 = 0;
            while i < len {
                let b = number.bytes(unchecked: i);
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
                    let b = number.bytes(unchecked: trimEnd - 1);
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
            result.appendChar('%')
        }

        _writePadded(into: writer, result, options)
    }'''

    return method.replace("__TYPE_NAME__", type_name).replace("__LANG_TYPE__", lang_type)


def generate_float(type_name: str, bits: int, is_default: bool) -> str:
    template = (SCRIPT_DIR / "float.ks.template").read_text()

    lang_type = f"f{bits}"
    other_float = "Float32" if bits == 64 else "Float64"
    other_lang_type = "f32" if bits == 64 else "f64"

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
