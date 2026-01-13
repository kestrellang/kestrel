pub use codespan_reporting::files::SimpleFile;

/// A span representing a region of source code.
///
/// Contains the file ID and byte range within that file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub file_id: usize,
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// Create a new span from a file ID and byte range.
    pub fn new(file_id: usize, range: std::ops::Range<usize>) -> Self {
        Self {
            file_id,
            start: range.start,
            end: range.end,
        }
    }

    /// Get the byte range of this span.
    pub fn range(&self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned {
            value: f(self.value),
            span: self.span,
        }
    }

    pub fn as_ref(&self) -> Spanned<&T> {
        Spanned {
            value: &self.value,
            span: self.span.clone(),
        }
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: std::hash::Hash> std::hash::Hash for Spanned<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

pub type Name = Spanned<String>;

pub type SourceFile = SimpleFile<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span() {
        let span = Span::new(1, 0..10);
        assert_eq!(span.file_id, 1);
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 10);
        assert_eq!(span.range(), 0..10);
    }

    #[test]
    fn test_spanned() {
        let spanned = Spanned::new(42, Span::new(0, 0..2));
        assert_eq!(spanned.value, 42);
        assert_eq!(spanned.span.range(), 0..2);
    }

    #[test]
    fn test_map() {
        let spanned = Spanned::new(42, Span::new(0, 0..2));
        let mapped = spanned.map(|x| x * 2);
        assert_eq!(mapped.value, 84);
        assert_eq!(mapped.span.range(), 0..2);
    }
}
