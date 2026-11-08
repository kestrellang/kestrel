//! `@derive(...)` expansion.
//!
//! `@derive(Equatable) struct Point { var x: Int64 }` means a conformance whose
//! witnesses read every stored field. The builder writes that conformance as
//! Kestrel source:
//!
//! ```kestrel
//! extend Point: Equatable {
//!     public func isEqual(to other: Self) -> Bool { self.x.isEqual(to: other.x) }
//! }
//! ```
//!
//! parses it into a file of its own (a [`DerivedFile`]), and builds it next to
//! the type like any extension, marked [`DerivedFrom`]. From there name
//! resolution, type inference, witness lowering and codegen treat it as
//! hand-written code.
//!
//! Why here, and why text:
//! - **Incremental.** The source is a pure function of the declaration's
//!   syntax, so it is rebuilt exactly when the type is, and unbuilt with its
//!   file. Nothing downstream needs a new input.
//! - **One source of truth for semantics.** The generated bodies call the
//!   field types' own witnesses; there is no second implementation of
//!   equality, hashing or formatting to keep in sync.
//!
//! What is generated is chosen by the protocol path's last segment
//! ([`DeriveKind::from_name`]). That is a guess the builder has to make before
//! names resolve; the `DeriveSite` analyzer (kestrel-analyze) checks that the
//! path resolves to the protocol with the matching [`Builtin`] marker, and
//! reports everything the expansion cannot (E950–E958).

mod generate;
mod shape;

use kestrel_hecs::{Entity, World};
use kestrel_hir::Builtin;
use kestrel_span::Span;
use kestrel_syntax_tree::ast::{self, AstNode, HasAttributes};
use kestrel_syntax_tree::utils::get_node_span;

use crate::attrs::{ParsedAttr, parse::parse_standalone};
use crate::build::{Built, build_items};
use crate::builders::helpers::extract_attribute;
use crate::components::{
    AstAttribute, DerivedFile, DerivedFrom, FileSyntax, Name, NodeKind, TargetConfig,
};

pub use shape::{Shape, ShapeBody, Slot};

/// The protocols with a built-in deriver.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeriveKind {
    Equatable,
    Hashable,
    Comparable,
    Formattable,
    Defaultable,
}

impl DeriveKind {
    pub const ALL: [DeriveKind; 5] = [
        DeriveKind::Equatable,
        DeriveKind::Hashable,
        DeriveKind::Comparable,
        DeriveKind::Formattable,
        DeriveKind::Defaultable,
    ];

    /// The deriver for a protocol named `name` (a path's last segment).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.name() == name)
    }

    /// The deriver for the protocol marked `builtin`.
    pub fn from_builtin(builtin: Builtin) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.builtin() == builtin)
    }

    /// The stdlib protocol's name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Equatable => "Equatable",
            Self::Hashable => "Hashable",
            Self::Comparable => "Comparable",
            Self::Formattable => "Formattable",
            Self::Defaultable => "Defaultable",
        }
    }

    /// The marker on the stdlib protocol this deriver implements.
    pub fn builtin(self) -> Builtin {
        match self {
            Self::Equatable => Builtin::EquatableProtocol,
            Self::Hashable => Builtin::HashableProtocol,
            Self::Comparable => Builtin::ComparableProtocol,
            Self::Formattable => Builtin::FormattableProtocol,
            Self::Defaultable => Builtin::DefaultableProtocol,
        }
    }
}

/// Expand every `@derive` among the containers of one built file.
///
/// Each `@derive(...)` attribute becomes one [`DerivedFile`] holding one
/// extension per derivable protocol it names, built under the module of the
/// declaration that carries it.
pub(crate) fn expand(
    world: &mut World,
    file_entity: Entity,
    module_parent: Entity,
    target: Option<&TargetConfig>,
    built: &Built,
) {
    let file_id = file_entity.index();
    for (site, item) in &built.containers {
        let Some((attr, attr_span)) = derive_attribute(item, file_id) else {
            continue;
        };
        let Some(ParsedAttr::Derive(paths)) = parse_standalone(&attr) else {
            continue;
        };
        let Some((ty, type_item)) = derived_type(world, *site, item, &built.containers) else {
            continue;
        };
        let Some(shape) = Shape::read(world, ty, type_item, file_id, target) else {
            continue;
        };
        let header = generate::Header::for_site(item);

        let mut source = String::new();
        let mut protocols: Vec<Vec<String>> = Vec::new();
        let mut kinds: Vec<DeriveKind> = Vec::new();
        for path in paths {
            let Some(kind) = path.last().and_then(|n| DeriveKind::from_name(n)) else {
                continue;
            };
            // A protocol named twice is AttrsOf's duplicate-argument error.
            if kinds.contains(&kind) {
                continue;
            }
            let Some(text) = generate::extension(kind, &path.join("."), &header, &shape) else {
                continue;
            };
            source.push_str(&text);
            kinds.push(kind);
            protocols.push(path);
        }
        if source.is_empty() {
            continue;
        }
        kestrel_debug::ktrace!("derive", "@derive on {:?} expands to:\n{}", site, source);

        let parent = module_of(world, *site).unwrap_or(module_parent);
        let Some(top_level) = build_source(
            world,
            file_entity,
            *site,
            attr_span,
            &source,
            parent,
            target,
        ) else {
            continue;
        };
        for (ext, protocol) in top_level.into_iter().zip(protocols) {
            world.set(
                ext,
                DerivedFrom {
                    site: *site,
                    ty,
                    protocol,
                },
            );
        }
    }
}

/// The declaration's `@derive` attribute, with the span of all of it.
fn derive_attribute(item: &ast::Item, file_id: usize) -> Option<(AstAttribute, Span)> {
    let attrs: Vec<ast::Attribute> = match item {
        ast::Item::StructDeclaration(n) => n.attributes().collect(),
        ast::Item::EnumDeclaration(n) => n.attributes().collect(),
        ast::Item::ExtensionDeclaration(n) => n.attributes().collect(),
        _ => return None,
    };
    attrs.iter().find_map(|a| {
        let attr = extract_attribute(a, file_id).filter(|attr| attr.name == "derive")?;
        Some((attr, get_node_span(a.syntax(), file_id)))
    })
}

/// The struct or enum whose fields the derive reads: the site itself, or for
/// an extension, the one type of that name declared in the same file.
/// `None` sends the site to the `DeriveSite` analyzer (E954).
fn derived_type<'a>(
    world: &World,
    site: Entity,
    item: &'a ast::Item,
    containers: &'a [(Entity, ast::Item)],
) -> Option<(Entity, &'a ast::Item)> {
    let ast::Item::ExtensionDeclaration(ext) = item else {
        return Some((site, item));
    };
    let name = generate::last_path_segment(&ext.ty()?.syntax().text().to_string())?;
    let mut types = containers.iter().filter(|(e, i)| {
        !matches!(i, ast::Item::ExtensionDeclaration(_))
            && world.get::<Name>(*e).is_some_and(|n| n.0 == name)
    });
    let (ty, ty_item) = types.next()?;
    if types.next().is_some() {
        return None;
    }
    Some((*ty, ty_item))
}

/// The module `entity` is declared in.
fn module_of(world: &World, entity: Entity) -> Option<Entity> {
    let mut current = world.parent_of(entity)?;
    while world.get::<NodeKind>(current) != Some(&NodeKind::Module) {
        current = world.parent_of(current)?;
    }
    Some(current)
}

/// Parse `source` into a new [`DerivedFile`] owned by `origin` and build its
/// items under `parent`. Returns the extensions, in source order.
fn build_source(
    world: &mut World,
    origin: Entity,
    site: Entity,
    attr_span: Span,
    source: &str,
    parent: Entity,
    target: Option<&TargetConfig>,
) -> Option<Vec<Entity>> {
    let file = world.spawn();
    let tokens: Vec<_> = kestrel_lexer::lex(source, file.index())
        .filter_map(Result::ok)
        .map(|t| (t.value, t.span))
        .collect();
    let parsed = kestrel_parser::parse_source_file_from_source(source, tokens.into_iter());
    if !parsed.errors.is_empty() {
        // The generator wrote something the parser rejects (an unusual field
        // type, say). Nothing is built; `DeriveSite` reports the site, since
        // the conformance it checks for is missing.
        kestrel_debug::ktrace!(
            "derive",
            "generated source does not parse: {:?}",
            parsed.errors
        );
        world.despawn(file);
        return None;
    }
    let tree = parsed.tree();
    world.set(file, FileSyntax(tree.green().into_owned()));
    world.set(
        file,
        DerivedFile {
            origin,
            site,
            attr_span,
        },
    );
    let items = ast::SourceFile::cast(tree)?.items().collect();
    Some(build_items(world, file, items, parent, target).top_level)
}
