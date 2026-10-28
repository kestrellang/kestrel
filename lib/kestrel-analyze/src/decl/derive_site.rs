//! # Derive Site Analyzer
//!
//! Checks each `@derive(P, ...)` (design §C7). The builder has already
//! expanded the attribute into `extend S: P { ... }` (kestrel-ast-builder
//! `derive`), choosing the deriver by the path's last segment before names
//! resolve. This analyzer is where the path is resolved and the expansion is
//! held to the rules; it is also the one place that explains a derive that
//! cannot work, so errors from inside the generated code are dropped when it
//! has spoken (kestrel-compiler `remap_derived`).
//!
//! Checks, per protocol, in order (the first failure ends that protocol's
//! checks):
//! 1. The path resolves to a protocol (E950) with a built-in deriver (E951).
//! 2. The type doesn't also declare the protocol itself (E953).
//! 3. Parent protocols hold: derived alongside, declared, or inferred (E958).
//! 4. The type is derivable here: an extension's type is in this file (E954),
//!    no unsized tail (E954), an enum `Defaultable` has one `@default` (E957).
//! 5. Every field (or payload) conforms (E952). `Defaultable` skips fields
//!    with an initializer, and reads only the `@default` case.
//! 6. No member written for the type witnesses one of the protocol's
//!    requirements (E953).
//!
//! ## Diagnostics
//!
//! ### E950 — `derive_not_a_protocol` (Error, Correctness)
//! - "cannot find protocol '{P}'"
//! - "'{P}' in '@derive' is not a protocol"
//!
//! ### E951 — `derive_not_derivable` (Error, Correctness)
//! - "'{P}' cannot be derived; the derivable protocols are Equatable, Hashable,
//!   Comparable, Formattable and Defaultable"
//! - "'Copyable' is not derived: whether '{S}' is Copyable follows from its fields"
//! - "'Cloneable' is not derived: whether '{S}' is Cloneable follows from its fields"
//!
//! ### E952 — `derive_field_not_conforming` (Error, Correctness)
//! - "cannot derive '{P}' for '{S}': field '{f}' has type '{T}', which is not '{P}'"
//! - "cannot derive '{P}' for '{S}': payload '{p}' of case '{C}' has type '{T}', which is not '{P}'"
//!
//! ### E953 — `derive_conflicts_with_conformance` (Error, Correctness)
//! - "'{S}' already declares '{P}'"
//! - "'{S}' derives '{P}' but also implements '{member}'; remove one"
//! - "'{P}' is listed twice in '@derive'"
//!
//! ### E954 — `derive_unsupported_declaration` (Error, Correctness)
//! - "'@derive' on an extension needs a type declared in this module"
//! - "'@derive' on an extension needs the type declared in the same file"
//! - "cannot derive '{P}' for '{S}': its last field is unsized"
//! - "cannot derive '{P}' for '{S}'" (a field type the deriver can't write)
//!
//! ### E957 — `derive_default_case` (Error, Correctness)
//! - "cannot derive 'Defaultable' for '{E}': mark exactly one case '@default'"
//!
//! ### E958 — `derive_missing_parent` (Error, Correctness)
//! - "deriving '{P}' requires '{S}' to be '{Q}'; add '{Q}' to '@derive'"
//!
//! Every diagnostic's primary label is the protocol's argument in `@derive`.

use kestrel_ast::AstType;
use kestrel_ast_builder::attrs::attrs_of;
use kestrel_ast_builder::derive::DeriveKind;
use kestrel_ast_builder::{
    AttrLiteral, Attributes, Callable, ConformanceItem, Conformances, DerivedFrom, FieldClass,
    FileId, NodeKind, Valued,
};
use kestrel_hecs::Entity;
use kestrel_hir::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::{
    ConformingProtocols, ExtensionTargetEntity, ExtensionsFor, ProtocolMembers, ResolveBuiltin,
    ResolveTypePath, TypeResolution,
};
use kestrel_semantics::sizedness::{hir_type_is_sized, last_stored_field_type};
use kestrel_span::Span;
use kestrel_syntax_tree::ast::{self, AstNode};
use kestrel_syntax_tree::utils::get_node_span;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;

const fn error(id: &'static str, name: &'static str) -> DiagnosticDescriptor {
    DiagnosticDescriptor {
        id,
        name,
        default_severity: Severity::Error,
        category: Category::Correctness,
    }
}

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    error("E950", "derive_not_a_protocol"),
    error("E951", "derive_not_derivable"),
    error("E952", "derive_field_not_conforming"),
    error("E953", "derive_conflicts_with_conformance"),
    error("E954", "derive_unsupported_declaration"),
    error("E957", "derive_default_case"),
    error("E958", "derive_missing_parent"),
];

const E950: usize = 0;
const E951: usize = 1;
const E952: usize = 2;
const E953: usize = 3;
const E954: usize = 4;
const E957: usize = 5;
const E958: usize = 6;

pub struct DeriveSiteAnalyzer;

impl Describe for DeriveSiteAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::DeriveSite
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DeriveSiteAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum, NodeKind::Extension]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let args = derive_args(cx);
        if args.is_empty() {
            return Vec::new();
        }
        let site = Site::new(cx);
        let mut diags = Vec::new();
        // The protocols this `@derive` derives, for the parent check.
        let derived: Vec<Entity> = args
            .iter()
            .filter_map(|(path, _)| site.resolve(cx, path).ok())
            .collect();
        let mut seen: Vec<Entity> = Vec::new();
        for (path, span) in &args {
            // The builder derives a protocol once; a repeat is only reported.
            if let Ok(p) = site.resolve(cx, path) {
                if seen.contains(&p) {
                    let name = util::entity_name(cx.query, p);
                    diags.push(diag(
                        E953,
                        format!("'{name}' is listed twice in '@derive'"),
                        span,
                    ));
                    continue;
                }
                seen.push(p);
            }
            if let Err(diag) = site.check(cx, path, span, &derived) {
                diags.push(diag);
            }
        }
        diags
    }
}

/// `@derive`'s protocol arguments, as written, with their spans.
fn derive_args(cx: &DeclContext<'_>) -> Vec<(Vec<String>, Span)> {
    if attrs_of(cx.query, cx.entity).derives().is_empty() {
        return Vec::new();
    }
    let Some(attrs) = cx.query.get::<Attributes>(cx.entity) else {
        return Vec::new();
    };
    // Only the first `@derive`, like the builder: a second one is AttrsOf's
    // E901.
    attrs
        .0
        .iter()
        .find(|a| a.name == "derive")
        .into_iter()
        .flat_map(|a| &a.args)
        .filter_map(|arg| match &arg.value {
            AttrLiteral::Path(p) => Some((p.clone(), arg.span.clone())),
            _ => None,
        })
        .collect()
}

/// The declaration carrying `@derive`, and the type it derives for.
struct Site {
    entity: Entity,
    /// The struct or enum; `None` when an extension's target didn't resolve
    /// (name resolution reports that).
    ty: Option<Entity>,
}

type Checked = Result<(), AnalyzeDiagnostic>;

impl Site {
    fn new(cx: &DeclContext<'_>) -> Site {
        let ty = if cx.kind == NodeKind::Extension {
            cx.query.query(ExtensionTargetEntity {
                extension: cx.entity,
                root: cx.root,
            })
        } else {
            Some(cx.entity)
        };
        Site {
            entity: cx.entity,
            ty,
        }
    }

    fn type_name(&self, cx: &DeclContext<'_>) -> String {
        util::entity_name(cx.query, self.ty.unwrap_or(self.entity))
    }

    /// The protocol `path` names, or E950.
    fn resolve(&self, cx: &DeclContext<'_>, path: &[String]) -> Result<Entity, String> {
        let shown = path.join(".");
        match cx.query.query(ResolveTypePath {
            segments: path.to_vec(),
            context: self.entity,
            root: cx.root,
        }) {
            TypeResolution::Found(e)
                if cx.query.get::<NodeKind>(e) == Some(&NodeKind::Protocol) =>
            {
                Ok(e)
            },
            TypeResolution::Found(_) | TypeResolution::NotAType(_) | TypeResolution::SelfType => {
                Err(format!("'{shown}' in '@derive' is not a protocol"))
            },
            TypeResolution::NotFound(_) => Err(format!("cannot find protocol '{shown}'")),
        }
    }

    fn check(
        &self,
        cx: &DeclContext<'_>,
        path: &[String],
        span: &Span,
        derived: &[Entity],
    ) -> Checked {
        let fail = |code: usize, message: String| diag(code, message, span);
        let protocol = self.resolve(cx, path).map_err(|m| fail(E950, m))?;
        let kind = self.deriver(cx, protocol).map_err(|m| fail(E951, m))?;
        let Some(ty) = self.ty else {
            return Ok(());
        };
        let name = self.type_name(cx);
        let p = kind.name();

        if let Some(problem) = self.extension_site_problem(cx, ty) {
            return Err(fail(E954, problem));
        }
        if let Some(declared_by) = self.declares(cx, ty, protocol) {
            let mut d = fail(E953, format!("'{name}' already declares '{p}'"));
            d.labels.push(secondary(
                util::entity_span(cx.query, declared_by),
                "declared here",
            ));
            return Err(d);
        }
        self.parents(cx, ty, protocol, derived).map_err(|q| {
            fail(
                E958,
                format!("deriving '{p}' requires '{name}' to be '{q}'; add '{q}' to '@derive'"),
            )
        })?;
        let extension = self.derived_extension(cx, ty, path);
        if let Some(tail) = last_stored_field_type(cx.query, ty, cx.root)
            && !hir_type_is_sized(cx.query, &tail, ty, cx.root)
        {
            return Err(fail(
                E954,
                format!("cannot derive '{p}' for '{name}': its last field is unsized"),
            ));
        }
        if kind == DeriveKind::Defaultable && cx.query.get::<NodeKind>(ty) == Some(&NodeKind::Enum)
        {
            let marked = default_cases(cx, ty);
            if marked.len() != 1 {
                let mut d = fail(
                    E957,
                    format!(
                        "cannot derive 'Defaultable' for '{name}': mark exactly one case '@default'"
                    ),
                );
                for case in marked {
                    d.labels.push(secondary(
                        util::entity_span(cx.query, case),
                        "marked '@default'",
                    ));
                }
                return Err(d);
            }
        }
        for slot in slots(cx, ty, kind) {
            if kestrel_type_infer::conformance::type_satisfies(
                cx.query, &slot.ty, protocol, cx.root,
            ) {
                continue;
            }
            let mut d = fail(
                E952,
                format!(
                    "cannot derive '{p}' for '{name}': {} has type '{}', which is not '{p}'",
                    slot.describe, slot.ty_text
                ),
            );
            d.labels.push(secondary(slot.span, format!("not '{p}'")));
            if kind == DeriveKind::Comparable
                && cx.query.get::<NodeKind>(ty) == Some(&NodeKind::Enum)
            {
                d.notes
                    .push("a derived 'Comparable' orders cases by declaration order".into());
            }
            return Err(d);
        }
        if let Some(member) = self.hand_written_witness(cx, ty, protocol) {
            let shown = util::entity_name(cx.query, member);
            let mut d = fail(
                E953,
                format!("'{name}' derives '{p}' but also implements '{shown}'; remove one"),
            );
            d.labels.push(secondary(
                util::entity_span(cx.query, member),
                "written here",
            ));
            return Err(d);
        }
        if extension.is_none() {
            // Nothing above explains why the builder wrote no conformance.
            return Err(fail(E954, format!("cannot derive '{p}' for '{name}'")));
        }
        Ok(())
    }

    /// The deriver for `protocol`, or the E951 message.
    fn deriver(&self, cx: &DeclContext<'_>, protocol: Entity) -> Result<DeriveKind, String> {
        let is = |builtin: Builtin| {
            cx.query.query(ResolveBuiltin {
                builtin,
                root: cx.root,
            }) == Some(protocol)
        };
        if let Some(kind) = DeriveKind::ALL.into_iter().find(|k| is(k.builtin())) {
            return Ok(kind);
        }
        let name = self.type_name(cx);
        for (builtin, p) in [
            (Builtin::Copyable, "Copyable"),
            (Builtin::Cloneable, "Cloneable"),
        ] {
            if is(builtin) {
                return Err(format!(
                    "'{p}' is not derived: whether '{name}' is {p} follows from its fields"
                ));
            }
        }
        Err(format!(
            "'{}' cannot be derived; the derivable protocols are Equatable, Hashable, Comparable, \
             Formattable and Defaultable",
            util::entity_name(cx.query, protocol)
        ))
    }

    /// Hand-written declarations of `ty`: the type and its non-derived extensions.
    fn written_decls(&self, cx: &DeclContext<'_>, ty: Entity) -> Vec<Entity> {
        let mut decls = vec![ty];
        decls.extend(
            cx.query
                .query(ExtensionsFor {
                    target: ty,
                    root: cx.root,
                })
                .into_iter()
                .filter(|&e| !cx.query.has::<DerivedFrom>(e)),
        );
        decls
    }

    /// The declaration (the type or a hand-written extension) that lists
    /// `protocol` in its own conformances.
    fn declares(&self, cx: &DeclContext<'_>, ty: Entity, protocol: Entity) -> Option<Entity> {
        self.written_decls(cx, ty)
            .into_iter()
            .find(|&decl| declared_protocols(cx, decl).contains(&protocol))
    }

    /// Every derivable parent of `protocol` holds for `ty`; else the first
    /// missing one's name. A parent holds when this `@derive` derives it,
    /// another `@derive` does, or a hand-written conformance implies it.
    /// (`ConformingProtocols` of the type can't tell: it sees this derive's
    /// own conformance, and through it every parent.)
    fn parents(
        &self,
        cx: &DeclContext<'_>,
        ty: Entity,
        protocol: Entity,
        derived: &[Entity],
    ) -> Result<(), String> {
        let derivable = |p: Entity| {
            DeriveKind::ALL.into_iter().any(|k| {
                cx.query.query(ResolveBuiltin {
                    builtin: k.builtin(),
                    root: cx.root,
                }) == Some(p)
            })
        };
        let closure = |p: Entity| {
            let mut all = cx.query.query(ConformingProtocols {
                entity: p,
                root: cx.root,
            });
            all.push(p);
            all
        };
        let mut holds: Vec<Entity> = derived.to_vec();
        for decl in self.written_decls(cx, ty) {
            for declared in declared_protocols(cx, decl) {
                holds.extend(closure(declared));
            }
        }
        for ext in cx.query.query(ExtensionsFor {
            target: ty,
            root: cx.root,
        }) {
            if let Some(d) = cx.query.get::<DerivedFrom>(ext)
                && let TypeResolution::Found(p) = cx.query.query(ResolveTypePath {
                    segments: d.protocol.clone(),
                    context: d.site,
                    root: cx.root,
                })
            {
                holds.push(p);
            }
        }
        // Only parents with a deriver: `Comparable`'s operator protocols
        // (`Less[Self]`, ...) come with its own protocol extension.
        let missing = closure(protocol)
            .into_iter()
            .filter(|&q| q != protocol && derivable(q))
            .find(|q| !holds.contains(q));
        match missing {
            Some(q) => Err(util::entity_name(cx.query, q)),
            None => Ok(()),
        }
    }

    /// The extension the builder generated for this site and path.
    fn derived_extension(
        &self,
        cx: &DeclContext<'_>,
        ty: Entity,
        path: &[String],
    ) -> Option<Entity> {
        cx.query
            .query(ExtensionsFor {
                target: ty,
                root: cx.root,
            })
            .into_iter()
            .find(|&e| {
                cx.query
                    .get::<DerivedFrom>(e)
                    .is_some_and(|d| d.site == self.entity && d.protocol == path)
            })
    }

    /// Why an extension can't derive for its type: the type is in another
    /// module, or (a 0.18 limit, since the builder reads the fields from
    /// syntax) another file.
    fn extension_site_problem(&self, cx: &DeclContext<'_>, ty: Entity) -> Option<String> {
        if cx.kind != NodeKind::Extension {
            return None;
        }
        let module = |e: Entity| kestrel_name_res::helpers::ancestor_module(cx.query, e);
        if module(ty) != module(self.entity) {
            return Some("'@derive' on an extension needs a type declared in this module".into());
        }
        let file = |e: Entity| cx.query.get::<FileId>(e).map(|f| f.0);
        if file(ty) != file(self.entity) {
            return Some(
                "'@derive' on an extension needs the type declared in the same file".into(),
            );
        }
        None
    }

    /// A member written for `ty` with the name and labels of one of
    /// `protocol`'s own requirements.
    fn hand_written_witness(
        &self,
        cx: &DeclContext<'_>,
        ty: Entity,
        protocol: Entity,
    ) -> Option<Entity> {
        let members = cx.query.query(ProtocolMembers {
            protocol,
            root: cx.root,
        });
        let requirements: Vec<Entity> = members
            .iter()
            .filter(|m| m.declaring_protocol == protocol && m.extension.is_none())
            .map(|m| m.entity)
            .collect();
        self.written_decls(cx, ty)
            .into_iter()
            .flat_map(|decl| cx.query.children_of(decl).to_vec())
            .find(|&member| requirements.iter().any(|&r| same_signature(cx, member, r)))
    }
}

/// The protocols `decl` lists in its own conformance list.
fn declared_protocols(cx: &DeclContext<'_>, decl: Entity) -> Vec<Entity> {
    let Some(conformances) = cx.query.get::<Conformances>(decl) else {
        return Vec::new();
    };
    conformances
        .0
        .iter()
        .filter_map(|item| {
            let ConformanceItem::Positive(AstType::Named { segments, .. }, _) = item else {
                return None;
            };
            match cx.query.query(ResolveTypePath {
                segments: segments.iter().map(|s| s.name.clone()).collect(),
                context: decl,
                root: cx.root,
            }) {
                TypeResolution::Found(p) => Some(p),
                _ => None,
            }
        })
        .collect()
}

/// Same kind, name and argument labels.
fn same_signature(cx: &DeclContext<'_>, a: Entity, b: Entity) -> bool {
    let kind = |e| cx.query.get::<NodeKind>(e).cloned();
    if kind(a) != kind(b) || !matches!(kind(a), Some(NodeKind::Function | NodeKind::Initializer)) {
        return false;
    }
    if util::entity_name(cx.query, a) != util::entity_name(cx.query, b) {
        return false;
    }
    let labels = |e| {
        cx.query
            .get::<Callable>(e)
            .map(|c| c.params.iter().map(|p| p.label.clone()).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    labels(a) == labels(b)
}

fn default_cases(cx: &DeclContext<'_>, en: Entity) -> Vec<Entity> {
    util::children_of_kind(cx.query, en, NodeKind::EnumCase)
        .into_iter()
        .filter(|&c| attrs_of(cx.query, c).is_default_case())
        .collect()
}

/// A field or payload value the deriver reads.
struct SlotTy {
    describe: String,
    ty: HirTy,
    ty_text: String,
    span: Span,
}

/// The slots `kind` needs `P` of: every stored field or payload, except that
/// `Defaultable` skips initialized fields and reads only the `@default` case.
fn slots(cx: &DeclContext<'_>, ty: Entity, kind: DeriveKind) -> Vec<SlotTy> {
    let defaultable = kind == DeriveKind::Defaultable;
    if cx.query.get::<NodeKind>(ty) == Some(&NodeKind::Enum) {
        let cases = if defaultable {
            default_cases(cx, ty)
        } else {
            util::children_of_kind(cx.query, ty, NodeKind::EnumCase)
        };
        return cases
            .into_iter()
            .flat_map(|c| payload_slots(cx, c))
            .collect();
    }
    cx.query
        .children_of(ty)
        .iter()
        .copied()
        .filter(|&f| {
            cx.query
                .get::<FieldClass>(f)
                .is_some_and(FieldClass::is_stored_instance)
        })
        .filter(|&f| !(defaultable && cx.query.has::<Valued>(f)))
        .filter_map(|f| {
            let ty = cx.query.query(LowerTypeAnnotation {
                entity: f,
                root: cx.root,
            })?;
            let node = kestrel_ast_builder::syntax::cst_node(cx.query, f)?;
            let ty_text = ast::FieldDeclaration::cast(node)?
                .ty()?
                .syntax()
                .text()
                .to_string();
            Some(SlotTy {
                describe: format!("field '{}'", util::entity_name(cx.query, f)),
                ty,
                ty_text: ty_text.trim().to_string(),
                span: util::entity_span(cx.query, f),
            })
        })
        .collect()
}

fn payload_slots(cx: &DeclContext<'_>, case: Entity) -> Vec<SlotTy> {
    let Some(tys) = cx.query.query(LowerCallableTypes {
        entity: case,
        root: cx.root,
    }) else {
        return Vec::new();
    };
    let params: Vec<ast::EnumCaseParameter> = kestrel_ast_builder::syntax::cst_node(cx.query, case)
        .and_then(ast::EnumCaseDeclaration::cast)
        .and_then(|c| c.enum_case_parameter_list())
        .map(|l| l.enum_case_parameters().collect())
        .unwrap_or_default();
    let case_name = util::entity_name(cx.query, case);
    tys.into_iter()
        .zip(params)
        .enumerate()
        .filter_map(|(i, (ty, param))| {
            let ty_node = param.ty()?;
            let label = param
                .name()
                .map(|n| n.syntax().text().to_string())
                .unwrap_or_else(|| i.to_string());
            let file = util::entity_span(cx.query, case).file_id;
            Some(SlotTy {
                describe: format!("payload '{label}' of case '{case_name}'"),
                ty: ty?,
                ty_text: ty_node.syntax().text().to_string().trim().to_string(),
                span: get_node_span(ty_node.syntax(), file),
            })
        })
        .collect()
}

fn diag(code: usize, message: String, span: &Span) -> AnalyzeDiagnostic {
    let d = &DESCRIPTORS[code];
    AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message,
        labels: vec![DiagLabel {
            span: span.clone(),
            message: "derived here".into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}

fn secondary(span: Span, message: impl Into<String>) -> DiagLabel {
    DiagLabel {
        span,
        message: message.into(),
        is_primary: false,
    }
}
