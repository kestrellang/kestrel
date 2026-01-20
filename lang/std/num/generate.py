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


def generate_integer_format_method(type_name: str, bits: int, signed: bool) -> str:
    """Generate the format() method for integer types."""

    # For converting digit to Int64 for UInt8 conversion
    if bits == 64:
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
            result.appendByte(UInt8(from: {digit_as_i64} + 48));
            n = n / ten
        }}

        if isNegative {{
            result.appendByte(45)  // '-'
        }}

        // Reverse the string
        var reversed = String();
        var i = result.byteCount() - 1;
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
            result.appendByte(UInt8(from: {digit_as_i64} + 48));
            n = n / ten
        }}

        // Reverse the string
        var reversed = String();
        var i = result.byteCount() - 1;
        while i >= 0 {{
            reversed.appendByte(result.byteAtUnchecked(i));
            i = i - 1
        }}
        reversed
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

    # Int literal init - need to cast from i64 for smaller types
    if bits == 64:
        int_literal_init = "self.raw = value"
    else:
        int_literal_init = f"self.raw = lang.cast_i64_i{bits}(value)"

    # Shift cast - need to cast count from i64 for smaller types
    if bits == 64:
        shift_cast = "count"
    else:
        shift_cast = f"lang.cast_i64_i{bits}(count)"

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

    result = template
    result = result.replace("{{TYPE_NAME}}", type_name)
    result = result.replace("{{BITS}}", str(bits))
    result = result.replace("{{SIGNEDNESS}}", signedness)
    result = result.replace("{{SIGNEDNESS_PROTOCOL}}", signedness_protocol)
    result = result.replace("{{LANG_TYPE}}", lang_type)
    result = result.replace("{{MIN_VALUE}}", str(min_val))
    result = result.replace("{{MIN_VALUE_ABS}}", str(min_val_abs))
    result = result.replace("{{MAX_VALUE}}", str(max_val))
    result = result.replace("{{SIGNED_PREFIX}}", signed_prefix)
    result = result.replace("{{NEGATABLE}}", negatable)
    result = result.replace("{{NEGATABLE_OUTPUT}}", negatable_output)
    result = result.replace("{{NEGATE_METHOD}}", negate_method)
    result = result.replace("{{ABS_METHOD}}", abs_method)
    result = result.replace("{{INT_LITERAL_INIT}}", int_literal_init)
    result = result.replace("{{SHIFT_CAST}}", shift_cast)
    result = result.replace("{{TYPE_ALIAS}}", type_alias)
    result = result.replace("{{CONVERTIBLE_CONFORMANCES}}", convertible_conformances)
    result = result.replace("{{CONVERTIBLE_INITS}}", convertible_inits)
    result = result.replace("{{FORMAT_METHOD}}", format_method)

    return result


def generate_float_format_method(type_name: str, bits: int) -> str:
    """Generate the format() method for float types."""
    lang_type = f"f{bits}"

    return f'''    // Formattable
    public func format() -> String {{
        // Handle special cases
        if self.isNaN() {{
            return "NaN"
        }}
        if self.isInfinite() {{
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
                digits.appendByte(UInt8(from: digit + 48));
                intVal = intVal / 10
            }}
            // Reverse digits
            var i = digits.byteCount() - 1;
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
            result.appendByte(UInt8(from: digit + 48));
            fracPart = fracPart - {type_name}(raw: lang.cast_i64_{lang_type}(digit.raw));
            digitCount = digitCount + 1
        }}

        result
    }}'''


def generate_float(type_name: str, bits: int, is_default: bool) -> str:
    template = (SCRIPT_DIR / "float.ks.template").read_text()

    lang_type = f"f{bits}"

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

    result = template
    result = result.replace("{{TYPE_NAME}}", type_name)
    result = result.replace("{{BITS}}", str(bits))
    result = result.replace("{{LANG_TYPE}}", lang_type)
    result = result.replace("{{FLOAT_LITERAL_INIT}}", float_literal_init)
    result = result.replace("{{ZERO_LITERAL}}", zero_literal)
    result = result.replace("{{TYPE_ALIAS}}", type_alias)
    result = result.replace("{{FORMAT_METHOD}}", format_method)

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
