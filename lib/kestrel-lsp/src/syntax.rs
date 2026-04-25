//! CST navigation helpers.
//!
//! Used by completion to find the receiver of a `.`-trigger, the
//! identifier prefix at the cursor, and the enclosing declaration.

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

/// True if the byte immediately before `offset` is `.`. Tells us whether we
/// should be doing member completion.
pub fn is_after_dot(source: &str, offset: usize) -> bool {
    let bytes = source.as_bytes();
    let mut i = offset.min(bytes.len());
    // Skip the in-progress identifier (LSP often invokes completion with the
    // cursor mid-word: `foo.ba|`).
    while i > 0 {
        let b = bytes[i - 1];
        if b == b'_' || b.is_ascii_alphanumeric() || b >= 0x80 {
            i -= 1;
        } else {
            break;
        }
    }
    i > 0 && bytes[i - 1] == b'.'
}

/// The bare identifier text before the dot at `offset`, if any. Returns
/// `None` when the receiver is more complex (call, parenthesized, etc.).
pub fn dot_receiver_identifier(source: &str, offset: usize) -> Option<&str> {
    let bytes = source.as_bytes();
    let mut i = offset.min(bytes.len());
    while i > 0 {
        let b = bytes[i - 1];
        if b == b'_' || b.is_ascii_alphanumeric() || b >= 0x80 {
            i -= 1;
        } else {
            break;
        }
    }
    if i == 0 || bytes[i - 1] != b'.' {
        return None;
    }
    let dot_at = i - 1;
    let mut start = dot_at;
    while start > 0 {
        let b = bytes[start - 1];
        if b == b'_' || b.is_ascii_alphanumeric() || b >= 0x80 {
            start -= 1;
        } else {
            break;
        }
    }
    if start == dot_at {
        None
    } else {
        Some(&source[start..dot_at])
    }
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

    #[test]
    fn detects_dot_with_partial_member() {
        assert!(is_after_dot("foo.", 4));
        assert!(is_after_dot("foo.b", 5));
        assert!(is_after_dot("foo.bar", 7));
        assert!(!is_after_dot("foo", 3));
        assert!(!is_after_dot("foo bar", 7));
    }

    #[test]
    fn extracts_dot_receiver_identifier() {
        assert_eq!(dot_receiver_identifier("foo.", 4), Some("foo"));
        assert_eq!(dot_receiver_identifier("foo.bar", 7), Some("foo"));
        assert_eq!(dot_receiver_identifier("a.b.c", 5), Some("b"));
        assert_eq!(dot_receiver_identifier(".bar", 4), None); // no receiver
        assert_eq!(dot_receiver_identifier("foo", 3), None); // no dot
    }
}
