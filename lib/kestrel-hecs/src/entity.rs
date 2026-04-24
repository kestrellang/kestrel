use std::fmt;

/// Compact runtime handle. Index into the World's entity table.
///
/// Cheap to copy and compare.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u32);

impl Entity {
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Debug for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Entity({})", self.0)
    }
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_copy_and_eq() {
        let e = Entity::from_raw(42);
        let e2 = e;
        assert_eq!(e, e2);
        assert_eq!(e.index(), 42);
    }
}
