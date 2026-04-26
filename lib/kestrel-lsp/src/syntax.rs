//! CST navigation helpers.
//!
//! `identifier_prefix` is the only consumer-facing helper now — used by
//! completion (both member and scope modes) to filter results by the
//! partial word the user has already typed. Member-vs-scope dispatch
//! itself moved to a HIR walk in `handlers::completion` once the
//! parser-recovery work made `foo.` parse to a real `HirExpr::Field`.

use kestrel_syntax_tree::{SyntaxNode, SyntaxToken};
use rowan::TextSize;

/// Token at `offset`, biased to the left when the cursor sits between two
/// tokens (typical for completion right after `.` or after a partial word).
pub fn token_at_offset(root: &SyntaxNode, offset: usize) -> Option<SyntaxToken> {
    let pos = TextSize::from(offset as u32);
    match root.token_at_offset(pos) {
        rowan::TokenAtOffset::None => None,
        rowan::TokenAtOffset::Single(t) => Some(t),
        rowan::TokenAtOffset::Between(left, _right) => Some(left),
    }
}

/// The identifier text immediately preceding `offset`. Used to filter
/// completions by the partial word the user has already typed.
pub fn identifier_prefix(source: &str, offset: usize) -> &str {
    let bytes = source.as_bytes();
    let end = offset.min(bytes.len());
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        // Identifier byte: ASCII alphanumeric / underscore, or any non-ASCII
        // byte (start or continuation of a multi-byte UTF-8 char). The caller
        // filters via name lookup, so over-inclusion is harmless.
        if b == b'_' || b.is_ascii_alphanumeric() || b >= 0x80 {
            start -= 1;
            continue;
        }
        break;
    }
    // Walk forward past any leading non-identifier byte (the loop above is
    // generous and may include a leading digit etc).
    while start < end {
        let c = bytes[start];
        if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 {
            break;
        }
        start += 1;
    }
    &source[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_finds_partial_word() {
        // "foo.bar" with cursor after 'r' (offset 7)
        assert_eq!(identifier_prefix("foo.bar", 7), "bar");
        assert_eq!(identifier_prefix("foo.bar", 6), "ba");
        assert_eq!(identifier_prefix("foo.bar", 4), "");
        // Pure identifier
        assert_eq!(identifier_prefix("hello", 5), "hello");
        assert_eq!(identifier_prefix("hello world", 8), "wo");
    }
}
