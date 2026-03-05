use std::fmt;

/// Stable identity derived from source structure.
///
/// Survives across compilations — the same path+kind always maps to the
/// same `Entity` handle when interned into a `World`. The `kind` field
/// is a user-defined discriminant (u16) so that e.g. "struct Foo" and
/// "func Foo" don't collide.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct EntityKey {
    path: Vec<String>,
    kind: u16,
}

impl EntityKey {
    pub fn new(path: Vec<String>, kind: u16) -> Self {
        Self { path, kind }
    }

    /// Create a root key with a single path segment.
    pub fn root(name: impl Into<String>, kind: u16) -> Self {
        Self {
            path: vec![name.into()],
            kind,
        }
    }

    /// Create a child key by appending a segment.
    pub fn child(&self, name: impl Into<String>, kind: u16) -> Self {
        let mut path = self.path.clone();
        path.push(name.into());
        Self { path, kind }
    }

    pub fn path(&self) -> &[String] {
        &self.path
    }

    pub fn kind(&self) -> u16 {
        self.kind
    }
}

impl fmt::Display for EntityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.path.iter().enumerate() {
            if i > 0 {
                write!(f, ".")?;
            }
            write!(f, "{segment}")?;
        }
        write!(f, ":{}", self.kind)
    }
}

/// Compact runtime handle. Index into the World's entity table.
///
/// Cheap to copy and compare. NOT stable across compilations —
/// use `EntityKey` for cross-compilation identity.
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
    fn entity_key_root_and_child() {
        let root = EntityKey::root("Main", 0);
        assert_eq!(root.path(), &["Main"]);
        assert_eq!(root.kind(), 0);

        let child = root.child("Point", 1);
        assert_eq!(child.path(), &["Main", "Point"]);
        assert_eq!(child.kind(), 1);

        let grandchild = child.child("x", 2);
        assert_eq!(grandchild.path(), &["Main", "Point", "x"]);
    }

    #[test]
    fn entity_key_display() {
        let key = EntityKey::new(vec!["Main".into(), "Point".into()], 1);
        assert_eq!(format!("{key}"), "Main.Point:1");
    }

    #[test]
    fn entity_key_equality_includes_kind() {
        let a = EntityKey::root("Foo", 0);
        let b = EntityKey::root("Foo", 1);
        assert_ne!(a, b);
    }

    #[test]
    fn entity_copy_and_eq() {
        let e = Entity::from_raw(42);
        let e2 = e;
        assert_eq!(e, e2);
        assert_eq!(e.index(), 42);
    }
}
