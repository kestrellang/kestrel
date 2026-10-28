//! The parser engine: a token source over non-trivia tokens, a rust-analyzer
//! style marker API, and the event buffer the grammar writes into.
//!
//! Grammar functions (in `grammar/`) drive a [`Parser`]: they peek at tokens
//! with [`Parser::at`] / [`Parser::nth`], consume them with [`Parser::bump`],
//! and bracket nodes with [`Parser::start`] → [`Marker::complete`]. A node
//! whose kind is only known after its first child has been parsed (binary
//! operators, postfix chains) is wrapped retroactively with
//! [`CompletedMarker::precede`], so the parser never backtracks and never
//! re-parses a subtree.
//!
//! Trivia never reaches the grammar: [`Parser::new`] keeps only significant
//! tokens and records, per token, whether a newline separated it from the
//! previous one (the few grammar positions where a line break matters ask
//! [`Parser::nl_before`]). `TreeBuilder` re-inserts trivia from the source
//! when it builds the tree, so the CST stays lossless.
//!
//! Errors are events too. [`Parser::finish`] turns the buffer into the
//! public [`crate::event::Event`] stream, resolving `precede` links and
//! deduplicating + position-sorting the diagnostics.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use crate::event::Event;
use crate::syntax_error::SyntaxError;

/// One significant (non-trivia) token.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Tok {
    pub kind: SyntaxKind,
    pub start: usize,
    pub end: usize,
    /// A `Newline` trivia token sits between this token and the previous
    /// significant one.
    pub nl_before: bool,
}

/// Internal event. `Start` carries a forward-parent link (rust-analyzer's
/// `precede` trick); `Tombstone` is an abandoned or forwarded start.
#[derive(Debug, Clone)]
enum Ev {
    Start {
        kind: SyntaxKind,
        forward_parent: Option<u32>,
    },
    Tombstone,
    Token {
        kind: SyntaxKind,
        start: usize,
        end: usize,
    },
    Finish,
    Error(SyntaxError),
}

/// Upper bound on `nth`/`at` calls without consuming a token. A grammar bug
/// that loops without progress panics in debug and stops parsing in release
/// instead of hanging the compiler.
const FUEL: u32 = 4096;

pub(crate) struct Parser<'s> {
    source: &'s str,
    file_id: usize,
    tokens: Vec<Tok>,
    /// For each opening bracket token, the index of its closing bracket
    /// (`u32::MAX` if unmatched or not an opener). Computed once, so every
    /// bracket-bounded lookahead is O(1) and parsing stays linear.
    closers: Vec<u32>,
    pos: usize,
    events: Vec<Ev>,
    fuel: std::cell::Cell<u32>,
    /// Nesting depth of interpolation holes being parsed.
    hole_depth: u32,
}

/// Saved parser state for [`Parser::rollback`].
pub(crate) struct Checkpoint {
    pos: usize,
    events: usize,
}

/// A started, not yet completed node.
#[must_use = "a Marker must be completed or abandoned"]
pub(crate) struct Marker {
    pos: u32,
    completed: bool,
}

/// A completed node, which can still be wrapped by a new parent.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompletedMarker {
    pos: u32,
}

impl<'s> Parser<'s> {
    /// Build a parser over a lexed token stream (trivia included, as the
    /// lexer produced it).
    pub fn new<I>(source: &'s str, tokens: I, file_id: usize) -> Self
    where
        I: Iterator<Item = (Token, Span)>,
    {
        let mut out = Vec::new();
        let mut nl = false;
        for (token, span) in tokens {
            if token.is_trivia() {
                nl |= matches!(token, Token::Newline);
                continue;
            }
            out.push(Tok {
                kind: SyntaxKind::from(token),
                start: span.start,
                end: span.end,
                nl_before: nl,
            });
            nl = false;
        }
        let closers = match_brackets(&out);
        Self {
            source,
            file_id,
            tokens: out,
            closers,
            pos: 0,
            events: Vec::new(),
            fuel: std::cell::Cell::new(FUEL),
            hole_depth: 0,
        }
    }

    // ----- token inspection -------------------------------------------------

    /// Kind of the token `n` ahead of the cursor; `None` at end of input.
    pub fn nth(&self, n: usize) -> Option<SyntaxKind> {
        let fuel = self.fuel.get();
        if fuel == 0 {
            debug_assert!(false, "parser made no progress (grammar loop)");
            return None;
        }
        self.fuel.set(fuel - 1);
        self.tokens.get(self.pos + n).map(|t| t.kind)
    }

    pub fn current(&self) -> Option<SyntaxKind> {
        self.nth(0)
    }

    pub fn at(&self, kind: SyntaxKind) -> bool {
        self.nth(0) == Some(kind)
    }

    pub fn nth_at(&self, n: usize, kind: SyntaxKind) -> bool {
        self.nth(n) == Some(kind)
    }

    pub fn at_any(&self, kinds: &[SyntaxKind]) -> bool {
        self.nth(0).is_some_and(|k| kinds.contains(&k))
    }

    pub fn at_any_nth(&self, n: usize, kinds: &[SyntaxKind]) -> bool {
        self.nth(n).is_some_and(|k| kinds.contains(&k))
    }

    pub fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Whether a line break separates the token `n` ahead from the one
    /// before it. End of input counts as "on a new line".
    pub fn nth_nl_before(&self, n: usize) -> bool {
        self.tokens.get(self.pos + n).is_none_or(|t| t.nl_before)
    }

    pub fn nl_before(&self) -> bool {
        self.nth_nl_before(0)
    }

    /// Source text of the token `n` ahead (empty at end of input). Used for
    /// contextual keywords (`ref`, `escaping`), which lex as identifiers.
    pub fn nth_text(&self, n: usize) -> &'s str {
        self.tokens
            .get(self.pos + n)
            .map_or("", |t| &self.source[t.start..t.end])
    }

    /// Whether the token `n` ahead is the contextual keyword `kw`.
    pub fn nth_is_contextual(&self, n: usize, kw: &str) -> bool {
        self.nth_at(n, SyntaxKind::Identifier) && self.nth_text(n) == kw
    }

    /// Absolute token index of the cursor (for bounded lookahead helpers).
    pub fn token_pos(&self) -> usize {
        self.pos
    }

    /// Absolute index of the bracket closing the opener at absolute index
    /// `open`, if any.
    pub fn closer_of(&self, open: usize) -> Option<usize> {
        self.closers
            .get(open)
            .copied()
            .filter(|&c| c != u32::MAX)
            .map(|c| c as usize)
    }

    /// Kind of the token at an absolute index.
    pub fn kind_at(&self, idx: usize) -> Option<SyntaxKind> {
        self.tokens.get(idx).map(|t| t.kind)
    }

    /// Whether the token at absolute index `idx` starts exactly where the
    /// previous token ends (no trivia between them).
    pub fn joined_at(&self, idx: usize) -> bool {
        match (
            idx.checked_sub(1).and_then(|i| self.tokens.get(i)),
            self.tokens.get(idx),
        ) {
            (Some(prev), Some(tok)) => prev.end == tok.start,
            _ => false,
        }
    }

    /// Byte range of the most recently consumed token.
    pub fn prev_range(&self) -> Option<std::ops::Range<usize>> {
        self.pos
            .checked_sub(1)
            .and_then(|i| self.tokens.get(i))
            .map(|t| t.start..t.end)
    }

    // ----- consuming ---------------------------------------------------------

    /// Consume the current token, emitting it with its own kind.
    pub fn bump_any(&mut self) {
        let Some(tok) = self.tokens.get(self.pos).copied() else {
            return;
        };
        self.bump_as(tok.kind);
    }

    /// Consume the current token, emitting it as `kind` (the CST sometimes
    /// spells a keyword as an `Identifier`, e.g. `init` in member position).
    pub fn bump_as(&mut self, kind: SyntaxKind) {
        let Some(tok) = self.tokens.get(self.pos).copied() else {
            return;
        };
        self.fuel.set(FUEL);
        self.pos += 1;
        self.events.push(Ev::Token {
            kind,
            start: tok.start,
            end: tok.end,
        });
    }

    /// Consume the current token, which the caller has checked is `kind`.
    pub fn bump(&mut self, kind: SyntaxKind) {
        debug_assert!(self.at(kind), "bump({kind:?}) at {:?}", self.current());
        self.bump_as(kind);
    }

    /// Consume `kind` if it is next.
    pub fn eat(&mut self, kind: SyntaxKind) -> bool {
        if !self.at(kind) {
            return false;
        }
        self.bump(kind);
        true
    }

    /// Consume `kind` or report it missing. The missing token is simply
    /// absent from the tree.
    pub fn expect(&mut self, kind: SyntaxKind) -> bool {
        if self.eat(kind) {
            return true;
        }
        self.error_expected(&[kind]);
        false
    }

    // ----- errors -------------------------------------------------------------

    pub fn push_error(&mut self, mut error: SyntaxError) {
        if self.hole_depth > 0 {
            error.message = format!(
                "invalid expression in string interpolation: {}",
                error.message
            );
        }
        self.events.push(Ev::Error(error));
    }

    /// Errors reported until [`Parser::exit_hole`] are inside a `\( … )`.
    pub fn enter_hole(&mut self) {
        self.hole_depth += 1;
    }

    pub fn exit_hole(&mut self) {
        self.hole_depth -= 1;
    }

    /// Byte range of the token at absolute index `idx`.
    pub fn range_of(&self, idx: usize) -> std::ops::Range<usize> {
        self.tokens
            .get(idx)
            .map_or_else(|| self.error_range(), |t| t.start..t.end)
    }

    /// "expected X, found Y" at the current token.
    pub fn error_expected(&mut self, kinds: &[SyntaxKind]) {
        let found = self.current();
        let range = self.error_range();
        self.push_error(SyntaxError::expected_tokens(kinds, found, range));
    }

    /// "expected <what>, found Y" at the current token.
    pub fn error_expected_what(&mut self, what: &str) {
        let found = self.current();
        let range = self.error_range();
        self.push_error(SyntaxError::expected_what(what, found, range));
    }

    /// Where to anchor an error about the current position: the current
    /// token, or — at end of input — the last real token, so the squiggle is
    /// visible.
    pub fn error_range(&self) -> std::ops::Range<usize> {
        if let Some(t) = self.tokens.get(self.pos) {
            return t.start..t.end;
        }
        self.tokens
            .last()
            .map_or(self.source.len()..self.source.len(), |t| t.start..t.end)
    }

    /// Wrap the current token in an `Error` node and consume it.
    pub fn err_bump(&mut self) {
        let m = self.start();
        self.bump_any();
        m.complete(self, SyntaxKind::Error);
    }

    /// Consume the current token; if it opens a bracket, consume through its
    /// matching closer so recovery never stops inside a nested group.
    pub fn bump_balanced(&mut self) {
        let here = self.pos;
        let close = self.closer_of(here);
        self.bump_any();
        if let Some(close) = close {
            while self.pos <= close && !self.at_eof() {
                self.bump_any();
            }
        }
    }

    /// Consume tokens into one `Error` node until `stop` holds (or EOF),
    /// skipping bracketed groups whole. Always consumes at least one token.
    pub fn err_recover_balanced(&mut self, stop: impl Fn(&Parser<'s>) -> bool) -> bool {
        if self.at_eof() {
            return false;
        }
        let m = self.start();
        self.bump_balanced();
        while !self.at_eof() && !stop(self) {
            self.bump_balanced();
        }
        m.complete(self, SyntaxKind::Error);
        true
    }

    // ----- markers ---------------------------------------------------------------

    pub fn start(&mut self) -> Marker {
        let pos = self.events.len() as u32;
        self.events.push(Ev::Start {
            kind: SyntaxKind::Error,
            forward_parent: None,
        });
        Marker {
            pos,
            completed: false,
        }
    }

    // ----- speculation -----------------------------------------------------------

    /// Remember the parser state, for a bounded speculative parse.
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            pos: self.pos,
            events: self.events.len(),
        }
    }

    /// Whether any error was reported since `cp`.
    pub fn has_errors_since(&self, cp: &Checkpoint) -> bool {
        self.events[cp.events..]
            .iter()
            .any(|e| matches!(e, Ev::Error(_)))
    }

    /// Undo everything since `cp`. Every marker started since then must
    /// already be completed or abandoned, and nothing before `cp` may have
    /// been `precede`d since.
    pub fn rollback(&mut self, cp: Checkpoint) {
        self.pos = cp.pos;
        self.events.truncate(cp.events);
    }

    // ----- output ----------------------------------------------------------------

    /// Resolve the internal buffer into public events: forward parents are
    /// unfolded into nested `StartNode`s, and errors are deduplicated and
    /// sorted by position (emitted after the tree events, which the tree
    /// builder ignores anyway).
    pub fn finish(mut self) -> Vec<Event> {
        let file_id = self.file_id;
        let mut out = Vec::with_capacity(self.events.len());
        let mut errors: Vec<SyntaxError> = Vec::new();
        let mut parents = Vec::new();
        for i in 0..self.events.len() {
            match std::mem::replace(&mut self.events[i], Ev::Tombstone) {
                Ev::Start {
                    kind,
                    forward_parent,
                } => {
                    parents.clear();
                    parents.push(kind);
                    let mut fp = forward_parent;
                    while let Some(idx) = fp {
                        match std::mem::replace(&mut self.events[idx as usize], Ev::Tombstone) {
                            Ev::Start {
                                kind,
                                forward_parent,
                            } => {
                                parents.push(kind);
                                fp = forward_parent;
                            },
                            _ => unreachable!("forward parent is not a Start event"),
                        }
                    }
                    for kind in parents.drain(..).rev() {
                        out.push(Event::StartNode(kind));
                    }
                },
                Ev::Tombstone => {},
                Ev::Token { kind, start, end } => {
                    out.push(Event::AddToken(kind, Span::new(file_id, start..end)));
                },
                Ev::Finish => out.push(Event::FinishNode),
                Ev::Error(e) => errors.push(e),
            }
        }
        // One diagnostic per position: the rest are cascades of the same
        // gap. An unclosed delimiter outranks whatever else ends there (a
        // missing `;` before a missing `}` at end of file is the `}`'s
        // cascade); otherwise the first reported cause wins (stable sort).
        let unclosed = |e: &SyntaxError| e.code != crate::syntax_error::codes::UNCLOSED_DELIMITER;
        errors.sort_by_key(|e| (e.range.start, unclosed(e), e.range.end));
        errors.dedup_by(|a, b| a.range.start == b.range.start);
        for e in errors {
            out.push(Event::Error {
                message: e.message,
                span: Some(Span::new(file_id, e.range)),
                code: Some(e.code),
            });
        }
        out
    }
}

/// Pair every opening bracket with its closer (kind-agnostic nesting, the
/// same way a reader counts depth).
fn match_brackets(tokens: &[Tok]) -> Vec<u32> {
    let mut closers = vec![u32::MAX; tokens.len()];
    let mut stack = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        match t.kind {
            SyntaxKind::LParen | SyntaxKind::LBracket | SyntaxKind::LBrace => stack.push(i),
            SyntaxKind::RParen | SyntaxKind::RBracket | SyntaxKind::RBrace => {
                if let Some(open) = stack.pop() {
                    closers[open] = i as u32;
                }
            },
            _ => {},
        }
    }
    closers
}

impl Marker {
    pub fn complete(mut self, p: &mut Parser<'_>, kind: SyntaxKind) -> CompletedMarker {
        self.completed = true;
        match &mut p.events[self.pos as usize] {
            Ev::Start { kind: slot, .. } => *slot = kind,
            _ => unreachable!("marker does not point at a Start event"),
        }
        p.events.push(Ev::Finish);
        CompletedMarker { pos: self.pos }
    }

    /// Drop the node, keeping whatever it contained as children of the
    /// enclosing node.
    pub fn abandon(mut self, p: &mut Parser<'_>) {
        self.completed = true;
        let idx = self.pos as usize;
        if idx == p.events.len() - 1 {
            p.events.pop();
        } else {
            p.events[idx] = Ev::Tombstone;
        }
    }
}

impl Drop for Marker {
    fn drop(&mut self) {
        if !self.completed && !std::thread::panicking() {
            panic!("Marker dropped without being completed or abandoned");
        }
    }
}

impl CompletedMarker {
    /// Start a new node that will wrap this one.
    pub fn precede(self, p: &mut Parser<'_>) -> Marker {
        let new = p.start();
        match &mut p.events[self.pos as usize] {
            Ev::Start { forward_parent, .. } => *forward_parent = Some(new.pos),
            _ => unreachable!("completed marker does not point at a Start event"),
        }
        new
    }
}
