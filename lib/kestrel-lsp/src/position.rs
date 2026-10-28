//! Position translation between Kestrel byte offsets and LSP UTF-16 line/col.
//!
//! Single source of truth for offset math — every handler routes through this
//! module. LSP `Position` values are UTF-16 code units; the compiler operates
//! on byte offsets. `LineIndex` precomputes line-start byte offsets for fast
//! conversion in either direction.

use tower_lsp::lsp_types::{Position, Range};

/// Per-document index of line-start byte offsets, used to translate between
/// LSP `Position` (UTF-16 line/col) and Kestrel `Span` (byte offsets).
#[derive(Debug, Clone)]
pub struct LineIndex {
    text: String,
    /// Byte offset of the start of each line. Always begins with 0 and has
    /// `line_count + 1` entries — the last entry is `text.len()` so any
    /// offset can be located in `O(log n)`.
    line_starts: Vec<usize>,
}

impl LineIndex {
    pub fn new(text: String) -> Self {
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        line_starts.push(text.len());
        Self { text, line_starts }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Convert an LSP UTF-16 position to a byte offset. Clamps to text length
    /// when the position is past EOF (LSP allows this for end-exclusive ranges).
    pub fn position_to_offset(&self, pos: Position) -> usize {
        let line = pos.line as usize;
        if line >= self.line_starts.len() - 1 {
            return self.text.len();
        }
        let line_start = self.line_starts[line];
        let line_end = self.line_starts[line + 1];
        let line_text = &self.text[line_start..line_end];
        let mut utf16_left = pos.character as usize;
        let mut byte = line_start;
        for ch in line_text.chars() {
            if utf16_left == 0 {
                break;
            }
            let units = ch.len_utf16();
            if utf16_left < units {
                // Mid-surrogate — clamp to the char start.
                break;
            }
            utf16_left -= units;
            byte += ch.len_utf8();
        }
        byte
    }

    /// Convert a byte offset to an LSP UTF-16 position. Offsets past EOF clamp
    /// to the position one past the last character, and offsets landing *inside*
    /// a multi-byte character clamp down to that character's start.
    ///
    /// The clamp is a backstop, not a nicety. This is called from `refresh`,
    /// outside the compiler worker's `catch_unwind`, so a single span whose
    /// endpoint was derived by byte arithmetic used to panic here and take the
    /// entire `publish_diagnostics` call with it — every diagnostic for that
    /// edit disappeared with nothing logged (F24). The producer that made such
    /// spans is fixed; this makes the class of bug non-fatal.
    pub fn offset_to_position(&self, offset: usize) -> Position {
        let mut offset = offset.min(self.text.len());
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        // Binary search for the largest line_start <= offset.
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i.min(self.line_starts.len() - 2),
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line];
        let line_text = &self.text[line_start..offset];
        let character = line_text.chars().map(|c| c.len_utf16()).sum::<usize>();
        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    /// Map a byte range to an LSP `Range`.
    pub fn range_for(&self, start: usize, end: usize) -> Range {
        Range {
            start: self.offset_to_position(start),
            end: self.offset_to_position(end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(s: &str) -> LineIndex {
        LineIndex::new(s.to_string())
    }

    /// A non-boundary offset must clamp, not panic — see the note on
    /// `offset_to_position`. Before F24 this input reached the server from
    /// `add_token_or_missing` and killed the whole diagnostic publish.
    #[test]
    fn offset_inside_a_multibyte_char_clamps_instead_of_panicking() {
        let i = idx("let x = café");
        // "café" is 5 bytes: c a f + 2 for é. The final byte offset is 12;
        // offset 11 is between é's two bytes.
        assert_eq!(i.text.len(), 13);
        assert!(!i.text.is_char_boundary(12));
        let at_split = i.offset_to_position(12);
        let at_char_start = i.offset_to_position(11);
        assert_eq!(at_split, at_char_start);
        // And a whole range built from such offsets is still well-formed.
        let r = i.range_for(12, 13);
        assert!(r.start.character <= r.end.character);
    }

    #[test]
    fn ascii_round_trip() {
        let i = idx("abc\ndef\nghi");
        assert_eq!(
            i.offset_to_position(0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            i.offset_to_position(4),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            i.offset_to_position(6),
            Position {
                line: 1,
                character: 2
            }
        );
        assert_eq!(
            i.position_to_offset(Position {
                line: 1,
                character: 2
            }),
            6
        );
        assert_eq!(
            i.position_to_offset(Position {
                line: 2,
                character: 3
            }),
            11
        );
    }

    #[test]
    fn multibyte_utf8_one_utf16_unit() {
        // 'é' is 2 bytes in UTF-8, 1 unit in UTF-16.
        let i = idx("aé\nb");
        // Byte offsets: a=0 é=1..3 \n=3 b=4
        assert_eq!(
            i.offset_to_position(3),
            Position {
                line: 0,
                character: 2
            }
        );
        assert_eq!(
            i.position_to_offset(Position {
                line: 0,
                character: 2
            }),
            3
        );
    }

    #[test]
    fn emoji_two_utf16_units() {
        // '😀' is 4 bytes in UTF-8, 2 units in UTF-16 (surrogate pair).
        let i = idx("a😀b");
        // a=0 😀=1..5 b=5
        assert_eq!(
            i.offset_to_position(5),
            Position {
                line: 0,
                character: 3
            }
        );
        assert_eq!(
            i.position_to_offset(Position {
                line: 0,
                character: 3
            }),
            5
        );
        // Mid-surrogate clamps to start of char (offset 1).
        assert_eq!(
            i.position_to_offset(Position {
                line: 0,
                character: 2
            }),
            1
        );
    }

    #[test]
    fn crlf_line_breaks() {
        // We treat '\n' as the line break; '\r' is part of the previous line.
        // Standard LSP behaviour — clients send '\r' as a regular character.
        let i = idx("a\r\nb");
        assert_eq!(
            i.offset_to_position(3),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            i.offset_to_position(4),
            Position {
                line: 1,
                character: 1
            }
        );
    }

    #[test]
    fn past_eof_clamps() {
        let i = idx("hi");
        assert_eq!(
            i.position_to_offset(Position {
                line: 99,
                character: 99
            }),
            2
        );
        assert_eq!(
            i.offset_to_position(999),
            Position {
                line: 0,
                character: 2
            }
        );
    }

    #[test]
    fn empty_document() {
        let i = idx("");
        assert_eq!(
            i.offset_to_position(0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            i.position_to_offset(Position {
                line: 0,
                character: 0
            }),
            0
        );
    }
}
