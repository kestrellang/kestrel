#!/usr/bin/env python3
"""
Generate integer and float type files from templates.
Run from this directory: python generate.py
"""

import os
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent

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


def get_cast(from_bits: int, to_bits: int) -> str:
    """Get the cast expression from one bit width to another."""
    if from_bits == to_bits:
        return "other.raw"
    else:
        return f"lang.cast_i{from_bits}_i{to_bits}(other.raw)"


def generate_sign_properties(type_name: str, bits: int, signed: bool, lang_type: str, signed_prefix: str) -> str:
    """Generate sign inspection properties for integer types."""
    if signed:
        return f'''    public var sign: {type_name} {{ get {{
        if Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 0)) {{ {type_name}(intLiteral: lang.i64_neg(1)) }}
        else if Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0)) {{ {type_name}.zero }}
        else {{ {type_name}.one }}
    }}}}

    public var isPositive: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_signed_gt(self.raw, 0))
    }}}}

    public var isNegative: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 0))
    }}}}

    public var isZero: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0))
    }}}}
'''
    else:
        return f'''    public var sign: {type_name} {{ get {{
        if Bool(boolLiteral: lang.{lang_type}_eq(self.raw, 0)) {{ {type_name}.zero }}
        else {{ {type_name}.one }}
    }}}}

    public var isPositive: Bool {{ get {{
        Bool(boolLiteral: lang.{lang_type}_unsigned_gt(self.raw, 0))
    }}}}

    public var isNegative: Bool {{ get {{
        // Unsigned types are never negative
        false
    }}}}

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

    public func negateChecked() -> {type_name}? {{
        if self == {type_name}.minValue {{
            return .None
        }};
        .Some(self.negate())
    }}

    public func absChecked() -> {type_name}? {{
        if self == {type_name}.minValue {{
            return .None
        }};
        .Some(self.abs())
    }}

'''
    else:
        return f'''    // TODO: requires overflow-detecting intrinsics for proper implementation
    public func addChecked(other: {type_name}) -> {type_name}? {{
        let result = self.add(other);
        // For unsigned, overflow if result < either operand
        if result < self {{
            return .None
        }};
        .Some(result)
    }}

    public func subtractChecked(other: {type_name}) -> {type_name}? {{
        // For unsigned, underflow if other > self
        if other > self {{
            return .None
        }};
        .Some(self.subtract(other))
    }}

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
        return f'''    public func addSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.addChecked(other);
        match checked {{
            .Some(result) => result,
            .None => if other.isPositive {{ {type_name}.maxValue }} else {{ {type_name}.minValue }}
        }}
    }}

    public func subtractSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.subtractChecked(other);
        match checked {{
            .Some(result) => result,
            .None => if other.isNegative {{ {type_name}.maxValue }} else {{ {type_name}.minValue }}
        }}
    }}

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

    public func negateSaturating() -> {type_name} {{
        if self == {type_name}.minValue {{
            {type_name}.maxValue
        }} else {{
            self.negate()
        }}
    }}

    public func absSaturating() -> {type_name} {{
        if self == {type_name}.minValue {{
            {type_name}.maxValue
        }} else {{
            self.abs()
        }}
    }}

'''
    else:
        return f'''    public func addSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.addChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {type_name}.maxValue
        }}
    }}

    public func subtractSaturating(other: {type_name}) -> {type_name} {{
        let checked = self.subtractChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {type_name}.zero
        }}
    }}

    public func multiplySaturating(other: {type_name}) -> {type_name} {{
        let checked = self.multiplyChecked(other);
        match checked {{
            .Some(result) => result,
            .None => {type_name}.maxValue
        }}
    }}

'''


def generate_integer_format_method(type_name: str, bits: int, signed: bool) -> str:
    """Generate the format() method for integer types."""

    # For converting digit to Int64 for UInt8 conversion
    if bits == 64 and signed:
        digit_as_i64 = "digit"
    else:
        digit_as_i64 = f"Int64(from: digit)"

    # For signed types, we need to handle negative numbers
    if signed:
        return f'''    // Formattable
    public func format() -> String {{
        if self == {type_name}.zero {{
            return "0"
        }}

        var result = String();
        var n = self;
        let isNegative = n < 0;
        if isNegative {{
            n = n.negate()
        }}

        let ten: {type_name} = 10;
        while n != {type_name}.zero {{
            let digit: {type_name} = n % ten;
            let charCode: Int64 = {digit_as_i64} + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
        }}

        if isNegative {{
            result.appendByte(45)  // '-'
        }}

        // Reverse the string
        var reversed = String();
        var i = result.byteCount - 1;
        while i >= 0 {{
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }}
        reversed
    }}'''
    else:
        return f'''    // Formattable
    public func format() -> String {{
        if self == {type_name}.zero {{
            return "0"
        }}

        var result = String();
        var n = self;

        let ten: {type_name} = 10;
        while n != {type_name}.zero {{
            let digit: {type_name} = n % ten;
            let charCode: Int64 = {digit_as_i64} + 48;
            result.appendByte(UInt8(from: charCode));
            n = n / ten
        }}

        // Reverse the string
        var reversed = String();
        var i = result.byteCount - 1;
        while i >= 0 {{
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }}
        reversed
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
        return f'''    public static func parse(string: String) -> {type_name}? {{
        let len = string.byteCount;
        if len == 0 {{
            return .None
        }}

        var index: Int64 = 0;
        var isNegative = false;

        // Check for sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {{  // '-'
            isNegative = true;
            index = 1
        }} else if firstByteVal == 43 {{  // '+'
            index = 1
        }}

        // Must have at least one digit
        if index >= len {{
            return .None
        }}

        // Parse digits using Int64 for accumulation
        var result: Int64 = 0;
        let maxBeforeMultiply: Int64 = 922337203685477580;  // Int64.maxValue / 10

        while index < len {{
            let byte: UInt8 = string.byteAtUnchecked(index);
            let byteVal = Int64(from: byte);

            // Check if digit (0-9 = 48-57)
            if byteVal < 48 or byteVal > 57 {{
                return .None
            }}

            let digit = byteVal - 48;

            // Check for overflow before multiply
            if result > maxBeforeMultiply {{
                return .None
            }}
            result = result * 10;

            // Check for overflow before add
            if result > 9223372036854775807 - digit {{
                return .None
            }}
            result = result + digit;

            index = index + 1
        }}

        // Apply sign and check bounds for target type
        if isNegative {{
            result = result.negate();
            if result < {min_val_expr} {{
                return .None
            }}
        }} else {{
            if result > {max_val_expr} {{
                return .None
            }}
        }}

        .Some({return_expr})
    }}'''
    else:
        # Unsigned - no negative numbers allowed
        max_before_multiply = "1844674407370955161"  # UInt64.maxValue / 10

        return f'''    public static func parse(string: String) -> {type_name}? {{
        let len = string.byteCount;
        if len == 0 {{
            return .None
        }}

        var index: Int64 = 0;

        // Check for optional + sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 43 {{  // '+'
            index = 1
        }} else if firstByteVal == 45 {{  // '-' not allowed for unsigned
            return .None
        }}

        // Must have at least one digit
        if index >= len {{
            return .None
        }}

        // Parse digits using UInt64 for accumulation
        var result: UInt64 = 0;
        let maxBeforeMultiply: UInt64 = {max_before_multiply};
        let maxVal: UInt64 = {max_val_expr};

        while index < len {{
            let byte: UInt8 = string.byteAtUnchecked(index);
            let byteVal = UInt64(from: byte);

            // Check if digit (0-9 = 48-57)
            if byteVal < 48 or byteVal > 57 {{
                return .None
            }}

            let digit = byteVal - 48;

            // Check for overflow before multiply
            if result > maxBeforeMultiply {{
                return .None
            }}
            result = result * 10;

            // Check for overflow before add
            if result > UInt64.maxValue - digit {{
                return .None
            }}
            result = result + digit;

            index = index + 1
        }}

        // Check bounds for target type
        if result > maxVal {{
            return .None
        }}

        .Some({return_expr})
    }}'''


def generate_integer(type_name: str, bits: int, signed: bool, is_default: bool) -> str:
    template = (SCRIPT_DIR / "integer.ks.template").read_text()

    lang_type = f"i{bits}"

    # Generate Convertible conformances and inits for all other integer types
    other_types = [(name, b) for name, b, _, _ in INTEGERS if name != type_name]

    conformances = []
    inits = []
    for other_name, other_bits in other_types:
        conformances.append(f"    Convertible[{other_name}]")
        cast_expr = get_cast(other_bits, bits)
        inits.append(f"    public init(from other: {other_name}) {{ self.raw = {cast_expr} }}")

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
        negate_method = f"public func negate() -> {type_name} {{ {type_name}(raw: lang.{lang_type}_neg(self.raw)) }}"
        abs_method = f"public func abs() -> {type_name} {{ if Bool(boolLiteral: lang.{lang_type}_signed_lt(self.raw, 0)) {{ self.negate() }} else {{ self }} }}"
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
        shift_cast = "count"
        shift_cast_i = "i.raw"
    else:
        shift_cast = f"lang.cast_i64_i{bits}(count)"
        shift_cast_i = f"lang.cast_i64_i{bits}(i.raw)"

    # Type alias for platform defaults
    if is_default:
        if signed:
            type_alias = f"\n// Int - platform-sized signed integer (alias to Int64 on 64-bit platforms)\npublic type Int = {type_name}"
        else:
            type_alias = f"\n// UInt - platform-sized unsigned integer (alias to UInt64 on 64-bit platforms)\npublic type UInt = {type_name}"
    else:
        type_alias = ""

    # Generate format method
    format_method = generate_integer_format_method(type_name, bits, signed)

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

    return result


def generate_float_parse_method(type_name: str, bits: int) -> str:
    """Generate the parse() method for float types."""
    lang_type = f"f{bits}"

    return f'''    public static func parse(string: String) -> {type_name}? {{
        let len = string.byteCount;
        if len == 0 {{
            return .None
        }}

        // Check for special values
        // "nan"
        if len == 3 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            // 'n' or 'N' = 110 or 78
            // 'a' or 'A' = 97 or 65
            let isN0 = Int64(from: b0) == 110 or Int64(from: b0) == 78;
            let isA1 = Int64(from: b1) == 97 or Int64(from: b1) == 65;
            let isN2 = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            if isN0 and isA1 and isN2 {{
                return .Some({type_name}.nan)
            }}
        }}

        // "inf"
        if len == 3 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            // 'i' or 'I' = 105 or 73
            // 'n' or 'N' = 110 or 78
            // 'f' or 'F' = 102 or 70
            let isI = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            if isI and isN and isF {{
                return .Some({type_name}.infinity)
            }}
        }}

        // "-inf"
        if len == 4 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let isMinus = Int64(from: b0) == 45;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isMinus and isI and isN and isF {{
                return .Some({type_name}(raw: lang.{lang_type}_neg(lang.{lang_type}_infinity())))
            }}
        }}

        // "+inf"
        if len == 4 {{
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let isPlus = Int64(from: b0) == 43;
            let isI = Int64(from: b1) == 105 or Int64(from: b1) == 73;
            let isN = Int64(from: b2) == 110 or Int64(from: b2) == 78;
            let isF = Int64(from: b3) == 102 or Int64(from: b3) == 70;
            if isPlus and isI and isN and isF {{
                return .Some({type_name}.infinity)
            }}
        }}

        // "infinity"
        if len == 8 {{
            // Check for "infinity" (case insensitive)
            let b0: UInt8 = string.byteAtUnchecked(0);
            let b1: UInt8 = string.byteAtUnchecked(1);
            let b2: UInt8 = string.byteAtUnchecked(2);
            let b3: UInt8 = string.byteAtUnchecked(3);
            let b4: UInt8 = string.byteAtUnchecked(4);
            let b5: UInt8 = string.byteAtUnchecked(5);
            let b6: UInt8 = string.byteAtUnchecked(6);
            let b7: UInt8 = string.byteAtUnchecked(7);
            let isI0 = Int64(from: b0) == 105 or Int64(from: b0) == 73;
            let isN1 = Int64(from: b1) == 110 or Int64(from: b1) == 78;
            let isF2 = Int64(from: b2) == 102 or Int64(from: b2) == 70;
            let isI3 = Int64(from: b3) == 105 or Int64(from: b3) == 73;
            let isN4 = Int64(from: b4) == 110 or Int64(from: b4) == 78;
            let isI5 = Int64(from: b5) == 105 or Int64(from: b5) == 73;
            let isT6 = Int64(from: b6) == 116 or Int64(from: b6) == 84;
            let isY7 = Int64(from: b7) == 121 or Int64(from: b7) == 89;
            if isI0 and isN1 and isF2 and isI3 and isN4 and isI5 and isT6 and isY7 {{
                return .Some({type_name}.infinity)
            }}
        }}

        // Parse regular number: [+-]?[0-9]*[.]?[0-9]*([eE][+-]?[0-9]+)?
        var index: Int64 = 0;
        var isNegative = false;

        // Check for sign
        let firstByte: UInt8 = string.byteAtUnchecked(0);
        let firstByteVal = Int64(from: firstByte);
        if firstByteVal == 45 {{  // '-'
            isNegative = true;
            index = 1
        }} else if firstByteVal == 43 {{  // '+'
            index = 1
        }}

        // Must have something after sign
        if index >= len {{
            return .None
        }}

        // Parse integer part - inline digit check (48='0', 57='9')
        var integerPart: {type_name} = 0.0;
        var hasIntegerPart = false;
        var currentByte: Int64 = Int64(from: string.byteAtUnchecked(index));

        while index < len and currentByte >= 48 and currentByte <= 57 {{
            let digit = {type_name}(from: currentByte - 48);
            integerPart = integerPart * 10.0 + digit;
            hasIntegerPart = true;
            index = index + 1;
            if index < len {{
                currentByte = Int64(from: string.byteAtUnchecked(index))
            }}
        }}

        // Parse fractional part
        var fractionalPart: {type_name} = 0.0;
        var hasFractionalPart = false;

        if index < len and currentByte == 46 {{  // '.'
            index = index + 1;
            var divisor: {type_name} = 10.0;

            if index < len {{
                currentByte = Int64(from: string.byteAtUnchecked(index));
                while index < len and currentByte >= 48 and currentByte <= 57 {{
                    let digit = {type_name}(from: currentByte - 48);
                    fractionalPart = fractionalPart + digit / divisor;
                    divisor = divisor * 10.0;
                    hasFractionalPart = true;
                    index = index + 1;
                    if index < len {{
                        currentByte = Int64(from: string.byteAtUnchecked(index))
                    }}
                }}
            }}
        }}

        // Must have at least integer or fractional part
        if not hasIntegerPart and not hasFractionalPart {{
            return .None
        }}

        var result = integerPart + fractionalPart;

        // Parse exponent part
        if index < len and (currentByte == 101 or currentByte == 69) {{  // 'e' or 'E'
            index = index + 1;

            if index >= len {{
                return .None  // 'e' with no exponent
            }}

            var expNegative = false;
            currentByte = Int64(from: string.byteAtUnchecked(index));

            if currentByte == 45 {{  // '-'
                expNegative = true;
                index = index + 1;
                if index < len {{
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }}
            }} else if currentByte == 43 {{  // '+'
                index = index + 1;
                if index < len {{
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }}
            }}

            if index >= len {{
                return .None  // No exponent digits
            }}

            var exponent: Int64 = 0;
            var hasExpDigit = false;

            while index < len and currentByte >= 48 and currentByte <= 57 {{
                exponent = exponent * 10 + (currentByte - 48);
                hasExpDigit = true;
                index = index + 1;
                if index < len {{
                    currentByte = Int64(from: string.byteAtUnchecked(index))
                }}
            }}

            if not hasExpDigit {{
                return .None
            }}

            // Apply exponent using pow
            let expFloat = {type_name}(from: exponent);
            let ten: {type_name} = 10.0;
            if expNegative {{
                result = result / ten.pow(expFloat)
            }} else {{
                result = result * ten.pow(expFloat)
            }}
        }}

        // Check for trailing characters
        if index != len {{
            return .None
        }}

        // Apply sign
        if isNegative {{
            result = result.negate()
        }}

        .Some(result)
    }}'''


def generate_float_format_method(type_name: str, bits: int) -> str:
    """Generate the format() method for float types."""
    lang_type = f"f{bits}"

    return f'''    // Formattable
    public func format() -> String {{
        // Handle special cases
        if self.isNaN {{
            return "NaN"
        }}
        if self.isInfinite {{
            if self < 0.0 {{
                return "-Infinity"
            }} else {{
                return "Infinity"
            }}
        }}

        var result = String();
        var value = self;

        // Handle negative
        let isNegative = value < 0.0;
        if isNegative {{
            result.appendByte(45);  // '-'
            value = value.negate()
        }}

        // Get integer part
        let intPart = value.trunc();
        var intVal: Int64 = Int64(raw: lang.cast_{lang_type}_i64(intPart.raw));

        // Format integer part
        if intVal == 0 {{
            result.appendByte(48)  // '0'
        }} else {{
            var digits = String();
            while intVal > 0 {{
                let digit: Int64 = intVal % 10;
                let charCode: Int64 = digit + 48;
                digits.appendByte(UInt8(from: charCode));
                intVal = intVal / 10
            }}
            // Reverse digits
            var i = digits.byteCount - 1;
            while i >= 0 {{
                result.appendByte(digits.byteAtUnchecked(i));
                i = i - 1
            }}
        }}

        // Add decimal point
        result.appendByte(46);  // '.'

        // Get fractional part (6 digits of precision)
        var fracPart = value - intPart;
        var digitCount: Int64 = 0;
        let maxDigits: Int64 = 6;
        let ten: {type_name} = 10.0;

        while digitCount < maxDigits {{
            fracPart = fracPart * ten;
            let digit: Int64 = Int64(raw: lang.cast_{lang_type}_i64(fracPart.trunc().raw));
            let charCode: Int64 = digit + 48;
            result.appendByte(UInt8(from: charCode));
            fracPart = fracPart - {type_name}(raw: lang.cast_i64_{lang_type}(digit.raw));
            digitCount = digitCount + 1
        }}

        result
    }}'''


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
        type_alias = f"\n// Float - alias to Float64\npublic type Float = {type_name}"
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
        min_positive = "2.2250738585072014e-308"
        epsilon = "2.220446049250313e-16"
    else:
        max_value = "3.4028235e38"
        min_positive = "1.17549435e-38"
        epsilon = "1.1920929e-7"

    # Conversion to other float type
    if bits == 64:
        to_other_float = f"Float32(raw: lang.cast_f64_f32(self.raw))"
        libm_suffix = ""  # f64 functions: sin, cos, etc.
    else:
        to_other_float = f"Float64(raw: lang.cast_f32_f64(self.raw))"
        libm_suffix = "f"  # f32 functions: sinf, cosf, etc.

    result = template
    result = result.replace("{{TYPE_NAME}}", type_name)
    result = result.replace("{{BITS}}", str(bits))
    result = result.replace("{{LANG_TYPE}}", lang_type)
    result = result.replace("{{OTHER_FLOAT}}", other_float)
    result = result.replace("{{OTHER_LANG_TYPE}}", other_lang_type)
    result = result.replace("{{FLOAT_LITERAL_INIT}}", float_literal_init)
    result = result.replace("{{ZERO_LITERAL}}", zero_literal)
    result = result.replace("{{TYPE_ALIAS}}", type_alias)
    result = result.replace("{{FORMAT_METHOD}}", format_method)
    result = result.replace("{{MAX_VALUE}}", max_value)
    result = result.replace("{{MIN_POSITIVE}}", min_positive)
    result = result.replace("{{EPSILON}}", epsilon)
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
