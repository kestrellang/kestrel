//! Modal lexing of cooked strings.
//!
//! Logos lexes ordinary code and recognises a string *opener* (`"` or
//! `"""`). From there this driver scans the string body itself, switching
//! between three modes kept on a stack:
//!
//! - **code** — plain logos lexing (the bottom of the stack);
//! - **string** — literal text up to `\(`, the closing quote(s), or the end;
//! - **hole** — logos lexing again, inside `\( … )`, counting `()[]{}` so the
//!   `)` that closes the hole is found; a `:` at depth 0 starts the format
//!   spec, which runs to that `)`. Strings nested in a hole push another
//!   string frame, so nesting is unbounded and every scanner is the same one.
//!
//! Output for a string with holes:
//!
//! ```text
//! StringStart (StringFragment | InterpStart <tokens> (Colon FormatSpec)? InterpEnd)* StringEnd
//! ```
//!
//! A string without holes is collapsed into a single `String` token holding
//! its whole source text, exactly as before, so literal patterns, attribute
//! arguments and plain literals keep one token.
//!
//! **Escapes.** In string mode `\` always takes the next character with it,
//! so `\"` does not close the string and `\\(` is not a hole.
//!
//! **Unterminated strings.** A single-line string that reaches the end of the
//! file without its closing `"` is cut at its first line break (it may
//! legally span lines when it *is* closed), so the rest of the file still
//! lexes. The token keeps no closer, which downstream reports (E707 for a
//! plain string, the parser for an interpolated one).

use std::ops::Range;

use logos::Logos;

use crate::Token;

type Lexed = (Result<Token, ()>, Range<usize>);

enum Frame {
    /// Inside a string body. `start` indexes the `StringStart` in the output.
    Str {
        multiline: bool,
        start: usize,
        has_hole: bool,
    },
    /// Inside `\( … )`, at bracket depth `depth`.
    Hole { depth: u32 },
}

/// Lex `source` into tokens with byte ranges (lex errors as `Err(())`).
pub(crate) fn lex_modal(source: &str) -> Vec<Lexed> {
    let mut out: Vec<Lexed> = Vec::new();
    let mut lexer = Token::lexer(source);
    let mut stack: Vec<Frame> = Vec::new();

    loop {
        if let Some(Frame::Str { multiline, .. }) = stack.last() {
            let multiline = *multiline;
            scan_string_step(&mut lexer, &mut out, &mut stack, multiline);
            continue;
        }
        let Some(token) = lexer.next() else {
            break;
        };
        let span = lexer.span();
        match (token.clone(), stack.last_mut()) {
            (Ok(Token::StringStart), _) => {
                let multiline = span.len() == 3;
                stack.push(Frame::Str {
                    multiline,
                    start: out.len(),
                    has_hole: false,
                });
                out.push((Ok(Token::StringStart), span));
            },
            (Ok(Token::LParen | Token::LBracket | Token::LBrace), Some(Frame::Hole { depth })) => {
                *depth += 1;
                out.push((token, span));
            },
            (Ok(Token::RParen), Some(Frame::Hole { depth: 0 })) => {
                stack.pop();
                out.push((Ok(Token::InterpEnd), span));
            },
            (Ok(Token::RParen | Token::RBracket | Token::RBrace), Some(Frame::Hole { depth })) => {
                *depth = depth.saturating_sub(1);
                out.push((token, span));
            },
            (Ok(Token::Colon), Some(Frame::Hole { depth: 0 })) => {
                out.push((Ok(Token::Colon), span.clone()));
                // The format spec runs to the hole's closing `)`.
                let rest = lexer.remainder();
                let len = rest.find([')', '\n', '"']).unwrap_or(rest.len());
                if len > 0 {
                    out.push((Ok(Token::FormatSpec), span.end..span.end + len));
                    lexer.bump(len);
                }
            },
            _ => out.push((token, span)),
        }
    }
    // Close frames left open at end of input.
    while let Some(frame) = stack.pop() {
        if let Frame::Str {
            start, has_hole, ..
        } = frame
            && !has_hole
        {
            collapse(&mut out, start, source.len());
        }
    }
    out
}

/// Scan one piece of a string body: a fragment, then a hole opener, the
/// closer, or the end of input.
fn scan_string_step(
    lexer: &mut logos::Lexer<'_, Token>,
    out: &mut Vec<Lexed>,
    stack: &mut Vec<Frame>,
    multiline: bool,
) {
    let base = lexer.span().end;
    let rest = lexer.remainder();
    let bytes = rest.as_bytes();
    let mut i = 0;
    let stop = loop {
        if i >= bytes.len() {
            break Stop::Eof;
        }
        match bytes[i] {
            b'\\' if bytes.get(i + 1) == Some(&b'(') => break Stop::Hole,
            b'\\' => {
                // An escape takes the next character with it.
                i += 1;
                if let Some(c) = rest[i..].chars().next() {
                    i += c.len_utf8();
                }
            },
            b'"' if !multiline => break Stop::Close(1),
            b'"' if rest[i..].starts_with("\"\"\"") => break Stop::Close(3),
            _ => i += 1,
        }
    };
    let Some(Frame::Str {
        start, has_hole, ..
    }) = stack.last_mut()
    else {
        unreachable!("scan_string_step outside a string frame");
    };
    let (start, has_hole_now) = (*start, *has_hole);

    // Unterminated single-line string: cut it at its first line break.
    let mut frag_end = i;
    if matches!(stop, Stop::Eof)
        && !multiline
        && let Some(nl) = rest[..i].find(['\n', '\r'])
    {
        frag_end = nl;
    }
    if frag_end > 0 {
        out.push((Ok(Token::StringFragment), base..base + frag_end));
    }
    // `bump` advances the logos lexer past the text consumed here, so the
    // next `lexer.span().end` is where the following token starts.
    match stop {
        Stop::Hole => {
            out.push((Ok(Token::InterpStart), base + i..base + i + 2));
            lexer.bump(i + 2);
            if let Some(Frame::Str { has_hole, .. }) = stack.last_mut() {
                *has_hole = true;
            }
            stack.push(Frame::Hole { depth: 0 });
        },
        Stop::Close(n) => {
            out.push((Ok(Token::StringEnd), base + i..base + i + n));
            lexer.bump(i + n);
            stack.pop();
            if !has_hole_now {
                collapse(out, start, base + i + n);
            }
        },
        Stop::Eof => {
            lexer.bump(frag_end);
            stack.pop();
            if !has_hole_now {
                collapse(out, start, base + frag_end);
            }
            // Text after the cut is lexed as code again.
        },
    }
}

enum Stop {
    Hole,
    Close(usize),
    Eof,
}

/// Replace `StringStart Fragment? StringEnd?` (from `start`) with a single
/// `String` token ending at `end`.
fn collapse(out: &mut Vec<Lexed>, start: usize, end: usize) {
    let begin = out[start].1.start;
    out.truncate(start);
    out.push((Ok(Token::String), begin..end));
}
