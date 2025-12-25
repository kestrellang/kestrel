//! Fully-qualified names for MIR items.

use std::fmt;

/// A fully-qualified name like `std.vec.Vec` or `example.main."closures".0`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QualifiedNameData {
    pub segments: Vec<String>,
}

impl QualifiedNameData {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    pub fn from_parts(parts: &[&str]) -> Self {
        Self {
            segments: parts.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Create a new qualified name by appending a segment.
    pub fn join(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Self { segments }
    }

    /// Get the last segment (the simple name).
    pub fn name(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    /// Get the parent path (all segments except the last).
    pub fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            None
        } else {
            Some(Self {
                segments: self.segments[..self.segments.len() - 1].to_vec(),
            })
        }
    }
}

impl fmt::Display for QualifiedNameData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.segments.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            // Quote segments that need it (contain dots, start with numbers, etc.)
            if segment.starts_with(|c: char| c.is_ascii_digit())
                || segment.contains('.')
                || segment.contains(' ')
            {
                write!(f, "{:?}", segment)?;
            } else {
                write!(f, "{}", segment)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualified_name_display() {
        let name = QualifiedNameData::from_parts(&["std", "vec", "Vec"]);
        assert_eq!(name.to_string(), "std.vec.Vec");

        let name = QualifiedNameData::from_parts(&["example", "main", "closures", "0"]);
        assert_eq!(name.to_string(), "example.main.closures.\"0\"");
    }

    #[test]
    fn test_qualified_name_join() {
        let base = QualifiedNameData::from_parts(&["std", "vec"]);
        let extended = base.join("Vec");
        assert_eq!(extended.to_string(), "std.vec.Vec");
    }
}
