//! Format specification parser for string interpolation.
//!
//! Parses format specs like `:>8`, `:<10.2f`, `:08x` into structured options.
//!
//! Grammar: `[[fill]align][sign][#][0][width][.precision][type]`
//!
//! - fill: Any character (default: space)
//! - align: `<` (left), `>` (right), `^` (center)
//! - sign: `+` (always), `-` (negative only), ` ` (space for positive)
//! - `#`: Alternate form (0x for hex, 0b for binary, etc.)
//! - `0`: Zero-pad (sets fill to '0' and align to right)
//! - width: Integer minimum field width
//! - `.precision`: `.` followed by integer precision
//! - type: `s` (string), `d` (decimal), `b` (binary), `o` (octal),
//!         `x`/`X` (hex), `e`/`E` (scientific), `f`/`F` (fixed),
//!         `%` (percent), `?` (debug)

/// Alignment options for formatted output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Right,
    Center,
}

/// Sign display mode for numeric formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignMode {
    /// Only show sign for negative numbers (default).
    #[default]
    Negative,
    /// Always show sign (+ or -).
    Always,
    /// Show space for positive, minus for negative.
    Space,
}

/// Float display style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloatStyle {
    /// Default: use shortest representation.
    #[default]
    Auto,
    /// Fixed-point notation (e.g., "3.14").
    Fixed,
    /// Scientific notation with lowercase 'e' (e.g., "3.14e0").
    Scientific,
    /// Scientific notation with uppercase 'E' (e.g., "3.14E0").
    ScientificUpper,
    /// Percentage (multiplies by 100, adds '%').
    Percent,
}

/// Format type specifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FormatType {
    /// No explicit type - use default for the value type.
    #[default]
    Default,
    /// String: `s`
    String,
    /// Decimal integer: `d`
    Decimal,
    /// Binary integer: `b`
    Binary,
    /// Octal integer: `o`
    Octal,
    /// Lowercase hex integer: `x`
    Hex,
    /// Uppercase hex integer: `X`
    HexUpper,
    /// Lowercase scientific float: `e`
    Scientific,
    /// Uppercase scientific float: `E`
    ScientificUpper,
    /// Fixed-point float: `f` or `F`
    Fixed,
    /// Percentage: `%`
    Percent,
    /// Debug format: `?`
    Debug,
}

/// Parsed format specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatSpec {
    /// Fill character (default: space).
    pub fill: char,
    /// Text alignment.
    pub alignment: Alignment,
    /// Sign display mode.
    pub sign: SignMode,
    /// Alternate form (show 0x/0b/0o prefix).
    pub alternate: bool,
    /// Zero-pad flag.
    pub zero_pad: bool,
    /// Minimum field width.
    pub width: Option<u32>,
    /// Precision (decimal places for floats, max chars for strings).
    pub precision: Option<u32>,
    /// Format type specifier.
    pub format_type: FormatType,
}

impl Default for FormatSpec {
    fn default() -> Self {
        Self {
            fill: ' ',
            alignment: Alignment::default(),
            sign: SignMode::default(),
            alternate: false,
            zero_pad: false,
            width: None,
            precision: None,
            format_type: FormatType::default(),
        }
    }
}

/// Error when parsing a format specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatSpecError {
    /// Invalid alignment character.
    InvalidAlignment(char),
    /// Invalid sign character.
    InvalidSign(char),
    /// Invalid format type.
    InvalidType(char),
    /// Invalid width value.
    InvalidWidth(String),
    /// Invalid precision value.
    InvalidPrecision(String),
    /// Unexpected character.
    UnexpectedChar(char),
    /// Empty precision after dot.
    EmptyPrecision,
}

impl std::fmt::Display for FormatSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatSpecError::InvalidAlignment(c) => write!(f, "invalid alignment character: '{c}'"),
            FormatSpecError::InvalidSign(c) => write!(f, "invalid sign character: '{c}'"),
            FormatSpecError::InvalidType(c) => write!(f, "invalid format type: '{c}'"),
            FormatSpecError::InvalidWidth(s) => write!(f, "invalid width: '{s}'"),
            FormatSpecError::InvalidPrecision(s) => write!(f, "invalid precision: '{s}'"),
            FormatSpecError::UnexpectedChar(c) => write!(f, "unexpected character: '{c}'"),
            FormatSpecError::EmptyPrecision => write!(f, "empty precision after '.'"),
        }
    }
}

impl std::error::Error for FormatSpecError {}

/// Parse a format specification string.
///
/// The input should NOT include the leading colon.
///
/// # Examples
///
/// ```ignore
/// parse_format_spec(">8")       // right-align, width 8
/// parse_format_spec("<10.2f")   // left-align, width 10, precision 2, fixed
/// parse_format_spec("08x")      // zero-pad, width 8, hex
/// parse_format_spec("^20s")     // center, width 20, string
/// parse_format_spec("+")        // always show sign
/// parse_format_spec("#x")       // hex with 0x prefix
/// ```
pub fn parse_format_spec(spec: &str) -> Result<FormatSpec, FormatSpecError> {
    let mut result = FormatSpec::default();
    let chars: Vec<char> = spec.chars().collect();
    let mut pos = 0;

    if chars.is_empty() {
        return Ok(result);
    }

    // Try to parse [[fill]align]
    // If we have at least 2 chars and the second is an alignment char,
    // then the first is a fill char
    if chars.len() >= 2 && is_alignment_char(chars[1]) {
        result.fill = chars[0];
        result.alignment = parse_alignment(chars[1])?;
        pos = 2;
    } else if !chars.is_empty() && is_alignment_char(chars[0]) {
        result.alignment = parse_alignment(chars[0])?;
        pos = 1;
    }

    // Parse [sign]
    if pos < chars.len() && is_sign_char(chars[pos]) {
        result.sign = parse_sign(chars[pos])?;
        pos += 1;
    }

    // Parse [#] alternate form
    if pos < chars.len() && chars[pos] == '#' {
        result.alternate = true;
        pos += 1;
    }

    // Parse [0] zero-pad
    // Note: '0' before width means zero-pad, not a width of 0
    if pos < chars.len() && chars[pos] == '0' {
        // Look ahead to see if this is zero-pad or just a width
        if pos + 1 < chars.len() && chars[pos + 1].is_ascii_digit() {
            // This is a zero-padded width like "08"
            result.zero_pad = true;
            result.fill = '0';
            if result.alignment == Alignment::Left {
                result.alignment = Alignment::Right;
            }
            pos += 1;
        } else if pos + 1 >= chars.len() || !chars[pos + 1].is_ascii_digit() {
            // Just "0" or "0" followed by non-digit (type char)
            result.zero_pad = true;
            result.fill = '0';
            if result.alignment == Alignment::Left {
                result.alignment = Alignment::Right;
            }
            pos += 1;
        }
    }

    // Parse [width]
    let width_start = pos;
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        pos += 1;
    }
    if pos > width_start {
        let width_str: String = chars[width_start..pos].iter().collect();
        result.width = Some(width_str.parse().map_err(|_| {
            FormatSpecError::InvalidWidth(width_str)
        })?);
    }

    // Parse [.precision]
    if pos < chars.len() && chars[pos] == '.' {
        pos += 1;
        let precision_start = pos;
        while pos < chars.len() && chars[pos].is_ascii_digit() {
            pos += 1;
        }
        if pos == precision_start {
            return Err(FormatSpecError::EmptyPrecision);
        }
        let precision_str: String = chars[precision_start..pos].iter().collect();
        result.precision = Some(precision_str.parse().map_err(|_| {
            FormatSpecError::InvalidPrecision(precision_str)
        })?);
    }

    // Parse [type]
    if pos < chars.len() {
        result.format_type = parse_format_type(chars[pos])?;
        pos += 1;
    }

    // Check for unexpected trailing characters
    if pos < chars.len() {
        return Err(FormatSpecError::UnexpectedChar(chars[pos]));
    }

    Ok(result)
}

fn is_alignment_char(c: char) -> bool {
    matches!(c, '<' | '>' | '^')
}

fn parse_alignment(c: char) -> Result<Alignment, FormatSpecError> {
    match c {
        '<' => Ok(Alignment::Left),
        '>' => Ok(Alignment::Right),
        '^' => Ok(Alignment::Center),
        _ => Err(FormatSpecError::InvalidAlignment(c)),
    }
}

fn is_sign_char(c: char) -> bool {
    matches!(c, '+' | '-' | ' ')
}

fn parse_sign(c: char) -> Result<SignMode, FormatSpecError> {
    match c {
        '+' => Ok(SignMode::Always),
        '-' => Ok(SignMode::Negative),
        ' ' => Ok(SignMode::Space),
        _ => Err(FormatSpecError::InvalidSign(c)),
    }
}

fn parse_format_type(c: char) -> Result<FormatType, FormatSpecError> {
    match c {
        's' => Ok(FormatType::String),
        'd' => Ok(FormatType::Decimal),
        'b' => Ok(FormatType::Binary),
        'o' => Ok(FormatType::Octal),
        'x' => Ok(FormatType::Hex),
        'X' => Ok(FormatType::HexUpper),
        'e' => Ok(FormatType::Scientific),
        'E' => Ok(FormatType::ScientificUpper),
        'f' | 'F' => Ok(FormatType::Fixed),
        '%' => Ok(FormatType::Percent),
        '?' => Ok(FormatType::Debug),
        _ => Err(FormatSpecError::InvalidType(c)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_spec() {
        let spec = parse_format_spec("").unwrap();
        assert_eq!(spec.fill, ' ');
        assert_eq!(spec.alignment, Alignment::Left);
        assert_eq!(spec.width, None);
        assert_eq!(spec.precision, None);
        assert_eq!(spec.format_type, FormatType::Default);
    }

    #[test]
    fn test_alignment_only() {
        let spec = parse_format_spec("<").unwrap();
        assert_eq!(spec.alignment, Alignment::Left);

        let spec = parse_format_spec(">").unwrap();
        assert_eq!(spec.alignment, Alignment::Right);

        let spec = parse_format_spec("^").unwrap();
        assert_eq!(spec.alignment, Alignment::Center);
    }

    #[test]
    fn test_fill_and_alignment() {
        let spec = parse_format_spec("-<").unwrap();
        assert_eq!(spec.fill, '-');
        assert_eq!(spec.alignment, Alignment::Left);

        let spec = parse_format_spec("*^").unwrap();
        assert_eq!(spec.fill, '*');
        assert_eq!(spec.alignment, Alignment::Center);

        let spec = parse_format_spec("0>").unwrap();
        assert_eq!(spec.fill, '0');
        assert_eq!(spec.alignment, Alignment::Right);
    }

    #[test]
    fn test_width() {
        let spec = parse_format_spec("10").unwrap();
        assert_eq!(spec.width, Some(10));

        let spec = parse_format_spec(">8").unwrap();
        assert_eq!(spec.alignment, Alignment::Right);
        assert_eq!(spec.width, Some(8));
    }

    #[test]
    fn test_precision() {
        let spec = parse_format_spec(".2").unwrap();
        assert_eq!(spec.precision, Some(2));

        let spec = parse_format_spec("10.5").unwrap();
        assert_eq!(spec.width, Some(10));
        assert_eq!(spec.precision, Some(5));
    }

    #[test]
    fn test_format_types() {
        let spec = parse_format_spec("s").unwrap();
        assert_eq!(spec.format_type, FormatType::String);

        let spec = parse_format_spec("d").unwrap();
        assert_eq!(spec.format_type, FormatType::Decimal);

        let spec = parse_format_spec("x").unwrap();
        assert_eq!(spec.format_type, FormatType::Hex);

        let spec = parse_format_spec("X").unwrap();
        assert_eq!(spec.format_type, FormatType::HexUpper);

        let spec = parse_format_spec("?").unwrap();
        assert_eq!(spec.format_type, FormatType::Debug);
    }

    #[test]
    fn test_zero_pad() {
        let spec = parse_format_spec("08").unwrap();
        assert_eq!(spec.zero_pad, true);
        assert_eq!(spec.fill, '0');
        assert_eq!(spec.alignment, Alignment::Right);
        assert_eq!(spec.width, Some(8));

        let spec = parse_format_spec("08x").unwrap();
        assert_eq!(spec.zero_pad, true);
        assert_eq!(spec.width, Some(8));
        assert_eq!(spec.format_type, FormatType::Hex);
    }

    #[test]
    fn test_sign() {
        let spec = parse_format_spec("+").unwrap();
        assert_eq!(spec.sign, SignMode::Always);

        let spec = parse_format_spec("-").unwrap();
        assert_eq!(spec.sign, SignMode::Negative);

        let spec = parse_format_spec(" ").unwrap();
        assert_eq!(spec.sign, SignMode::Space);

        let spec = parse_format_spec("+10d").unwrap();
        assert_eq!(spec.sign, SignMode::Always);
        assert_eq!(spec.width, Some(10));
        assert_eq!(spec.format_type, FormatType::Decimal);
    }

    #[test]
    fn test_alternate_form() {
        let spec = parse_format_spec("#x").unwrap();
        assert_eq!(spec.alternate, true);
        assert_eq!(spec.format_type, FormatType::Hex);

        let spec = parse_format_spec("#08x").unwrap();
        assert_eq!(spec.alternate, true);
        assert_eq!(spec.zero_pad, true);
        assert_eq!(spec.width, Some(8));
        assert_eq!(spec.format_type, FormatType::Hex);
    }

    #[test]
    fn test_complex_spec() {
        // Right-aligned, width 10, precision 2, fixed float
        let spec = parse_format_spec(">10.2f").unwrap();
        assert_eq!(spec.alignment, Alignment::Right);
        assert_eq!(spec.width, Some(10));
        assert_eq!(spec.precision, Some(2));
        assert_eq!(spec.format_type, FormatType::Fixed);

        // Fill with *, center-aligned, width 20, string
        let spec = parse_format_spec("*^20s").unwrap();
        assert_eq!(spec.fill, '*');
        assert_eq!(spec.alignment, Alignment::Center);
        assert_eq!(spec.width, Some(20));
        assert_eq!(spec.format_type, FormatType::String);

        // Always show sign, alternate form, zero-pad, width 10, hex
        let spec = parse_format_spec("+#010x").unwrap();
        assert_eq!(spec.sign, SignMode::Always);
        assert_eq!(spec.alternate, true);
        assert_eq!(spec.zero_pad, true);
        assert_eq!(spec.width, Some(10));
        assert_eq!(spec.format_type, FormatType::Hex);
    }

    #[test]
    fn test_errors() {
        assert!(matches!(
            parse_format_spec("10."),
            Err(FormatSpecError::EmptyPrecision)
        ));

        assert!(matches!(
            parse_format_spec("z"),
            Err(FormatSpecError::InvalidType('z'))
        ));

        assert!(matches!(
            parse_format_spec("10xy"),
            Err(FormatSpecError::UnexpectedChar('y'))
        ));
    }
}
