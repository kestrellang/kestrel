//! `@sugarOnly` requirements: reached only through their syntax.
//!
//! A protocol requirement marked `@sugarOnly` (`remainderAssign` behind `%=`,
//! `tryExtract` behind `try`) is still implemented by conformers, but outside
//! the protocol's module nothing may call it — or any type's implementation
//! of it — by name. The compiler's own desugaring is exempt: callers that
//! resolve a member *for* a piece of syntax skip this check.
//!
//! `SugarOnlyRequirement` is the single source of truth for "is this member
//! sugar-only, and through which requirement". Inference, the LSP and the
//! doc generator all ask it.

use kestrel_ast_builder::{Callable, NodeKind, attrs::attrs_of};
use kestrel_hecs::{Entity, QueryContext, QueryFn};

use crate::conformances::ConformingProtocols;
use crate::extensions::ExtensionTargetEntity;
use crate::helpers::member_lookup_name;
use crate::protocol_members::ProtocolMembers;
use crate::visibility::internal_boundary;

// ===== SugarOnlyRequirement =====

/// Query: the `@sugarOnly` requirement `member` is, or implements.
///
/// - A requirement (or init requirement) carrying `@sugarOnly` answers itself.
/// - Any other member answers the first `@sugarOnly` requirement it
///   witnesses: a requirement of a protocol its owner conforms to, with the
///   same name and argument labels. That covers witnesses on the type
///   (`Int64.remainderAssign`), in its extensions, and default implementations in
///   protocol extensions (`extend Equatable { func notEqual }`).
///
/// Witnessing is decided by name + labels, which is how conformance itself
/// matches a witness to its requirement.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SugarOnlyRequirement {
    pub member: Entity,
    pub root: Entity,
}

impl QueryFn for SugarOnlyRequirement {
    type Output = Option<Entity>;

    fn describe(&self) -> String {
        format!("SugarOnlyRequirement({:?})", self.member)
    }

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Entity> {
        let parent = ctx.parent_of(self.member)?;
        let parent_kind = ctx.get::<NodeKind>(parent)?;
        if *parent_kind == NodeKind::Protocol && attrs_of(ctx, self.member).is_sugar_only() {
            return Some(self.member);
        }

        let owner = match parent_kind {
            NodeKind::Extension => ctx.query(ExtensionTargetEntity {
                extension: parent,
                root: self.root,
            })?,
            NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol => parent,
            _ => return None,
        };
        // A protocol's own extension defaults witness the protocol's
        // requirements as well as those of the protocols it refines.
        let own = (ctx.get::<NodeKind>(owner) == Some(&NodeKind::Protocol)).then_some(owner);
        let conformed = ctx.query(ConformingProtocols {
            entity: owner,
            root: self.root,
        });
        let name = member_lookup_name(ctx, self.member)?;
        let labels = labels_of(ctx, self.member);
        own.into_iter()
            .chain(conformed.iter().copied())
            .find_map(|protocol| {
                let members = ctx.query(ProtocolMembers {
                    protocol,
                    root: self.root,
                });
                members
                    .named(name)
                    .map(|m| m.entity)
                    .filter(|&req| req != self.member)
                    .find(|&req| {
                        ctx.parent_of(req)
                            .is_some_and(|p| ctx.get::<NodeKind>(p) == Some(&NodeKind::Protocol))
                            && attrs_of(ctx, req).is_sugar_only()
                            && labels_of(ctx, req) == labels
                    })
            })
    }
}

/// The `@sugarOnly` requirement that hides `member` from code in `context`,
/// or `None` when `context` may name it. Three places may still name it:
/// - code in the requirement's protocol's module (its `internal_boundary`);
/// - code in the module that wrote this implementation, so a type's own
///   module keeps calling its own methods and inits (a custom
///   `Interpolatable` builds itself with `init(literalCapacity:…)`);
/// - the body of another sugar-only implementation.
pub fn sugar_only_hidden(
    ctx: &QueryContext<'_>,
    member: Entity,
    context: Entity,
    root: Entity,
) -> Option<Entity> {
    let requirement = ctx.query(SugarOnlyRequirement { member, root })?;
    // An implementation of a sugar-only requirement may forward to others:
    // a type's `init(stringLiteral:length:)` builds its `String` field with
    // `String(stringLiteral:length:)`, which no literal syntax can spell.
    if ctx
        .query(SugarOnlyRequirement {
            member: context,
            root,
        })
        .is_some()
    {
        return None;
    }
    let here = internal_boundary(ctx, context);
    if internal_boundary(ctx, requirement) == here
        || (member != requirement && internal_boundary(ctx, member) == here)
    {
        return None;
    }
    Some(requirement)
}

/// Argument labels, `_` for an unlabelled parameter. Members without a
/// `Callable` (properties) have none.
fn labels_of(ctx: &QueryContext<'_>, entity: Entity) -> Vec<String> {
    ctx.get::<Callable>(entity)
        .map(|c| {
            c.params
                .iter()
                .map(|p| p.label.clone().unwrap_or_else(|| "_".into()))
                .collect()
        })
        .unwrap_or_default()
}
