//! Declarations: the top-level item list and the members of type bodies.
//!
//! ```text
//! item = ModuleDeclaration | ImportDeclaration | ExtensionDeclaration
//!      | ProtocolDeclaration | StructDeclaration | EnumDeclaration
//!      | FunctionDeclaration | SubscriptDeclaration | FieldDeclaration
//!      | TypeAliasDeclaration
//!      | InitializerDeclaration | DeinitDeclaration | EnumCaseDeclaration  // bodies only
//! header = AttributeList? Visibility modifiers keyword
//! ```
//!
//! Which items a context accepts is [`Ctx::allows`]. The kind of an item is
//! decided up front by scanning past its attributes, visibility and
//! modifiers to the introducing keyword (bounded: attribute arguments are
//! skipped as one bracket group). A malformed item keeps its node and
//! reports errors; tokens that cannot start an item are skipped into an
//! `Error` node up to the next item start.

use kestrel_syntax_tree::SyntaxKind as K;

use super::attrs::attribute_list;
use super::blocks::{expect_closer, top_block};
use super::exprs::expr;
use super::generics::{opt_conformance_list, opt_type_parameter_list, opt_where_clause};
use super::patterns::{at_param_pattern_start, param_pattern};
use super::types::{at_type_start, ty, type_argument_list};
use super::{delimited, is_label_keyword, name};
use crate::core::Parser;
use crate::syntax_error::SyntaxError;

/// Where a declaration appears.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Top,
    Struct,
    Enum,
    Protocol,
    Extension,
}

impl Ctx {
    /// Whether an item introduced by `kw` may appear here.
    fn allows(self, kw: K) -> bool {
        use Ctx::*;
        match kw {
            K::Func | K::Subscript | K::Let | K::Var | K::Type => true,
            K::Module | K::Import | K::Struct | K::Enum => matches!(self, Top | Struct | Enum),
            K::Protocol | K::Extend => self == Top,
            K::Init => matches!(self, Struct | Enum | Protocol | Extension),
            K::Deinit => self == Struct,
            K::Case => self == Enum,
            _ => false,
        }
    }

    fn what(self) -> &'static str {
        if self == Ctx::Top {
            "declaration"
        } else {
            "member declaration"
        }
    }
}

const VISIBILITY: &[K] = &[K::Public, K::Private, K::Internal, K::Fileprivate];
const MODIFIERS: &[K] = &[K::Static, K::Mutating, K::Consuming, K::Indirect];

/// Tokens that can begin an item (or its header).
fn at_item_start(p: &Parser<'_>) -> bool {
    p.at_any(&[
        K::Module,
        K::Import,
        K::Protocol,
        K::Struct,
        K::Enum,
        K::Extend,
        K::Func,
        K::Init,
        K::Deinit,
        K::Subscript,
        K::Type,
        K::Let,
        K::Var,
        K::Case,
        K::At,
    ]) || p.at_any(VISIBILITY)
        || p.at_any(MODIFIERS)
}

/// The keyword introducing the item at the cursor, looking past
/// attributes, a visibility keyword and modifiers. Also reports whether
/// anything preceded it.
fn scan_head(p: &Parser<'_>) -> (Option<K>, bool) {
    let start = p.token_pos();
    let mut i = start;
    while p.kind_at(i) == Some(K::At) {
        i += 1;
        if p.kind_at(i) == Some(K::Identifier) {
            i += 1;
        }
        if p.kind_at(i) == Some(K::LParen) {
            match p.closer_of(i) {
                Some(close) => i = close + 1,
                None => return (None, true),
            }
        }
    }
    if p.kind_at(i).is_some_and(|k| VISIBILITY.contains(&k)) {
        i += 1;
    }
    while p.kind_at(i).is_some_and(|k| MODIFIERS.contains(&k)) {
        i += 1;
    }
    (p.kind_at(i), i != start)
}

/// The top-level item list.
pub(super) fn item_list(p: &mut Parser<'_>) {
    while !p.at_eof() {
        item_or_recover(p, Ctx::Top);
    }
}

/// `{ member* }` of a type body, as a `kind` node.
fn member_list(p: &mut Parser<'_>, ctx: Ctx, kind: K) {
    if !p.at(K::LBrace) {
        p.error_expected(&[K::LBrace]);
        return;
    }
    let m = p.start();
    p.bump(K::LBrace);
    while !p.at(K::RBrace) && !p.at_eof() {
        item_or_recover(p, ctx);
    }
    expect_closer(p, K::RBrace);
    m.complete(p, kind);
}

fn item_or_recover(p: &mut Parser<'_>, ctx: Ctx) {
    let (kw, has_head) = scan_head(p);
    let headless = matches!(kw, Some(K::Module | K::Import | K::Extend | K::Deinit));
    let ok = at_item_start(p)
        && kw.is_some_and(|k| ctx.allows(k))
        && !(headless && has_head)
        && !(kw == Some(K::Case) && has_head && !p.at(K::At));
    if ok {
        item(p, ctx, kw.unwrap_or(K::Error));
        return;
    }
    p.error_expected_what(ctx.what());
    // Skip to the next item start (or the end of the body).
    let m = p.start();
    loop {
        p.bump_balanced();
        if p.at_eof() || (ctx != Ctx::Top && p.at(K::RBrace)) || at_item_start(p) {
            break;
        }
    }
    m.complete(p, K::Error);
}

fn item(p: &mut Parser<'_>, ctx: Ctx, kw: K) {
    match kw {
        K::Module => module_decl(p),
        K::Import => import_decl(p),
        K::Extend => extension_decl(p),
        K::Deinit => deinit_decl(p),
        K::Case => enum_case(p),
        K::Protocol => type_decl(p, K::Protocol),
        K::Struct => type_decl(p, K::Struct),
        K::Enum => type_decl(p, K::Enum),
        K::Init => init_decl(p),
        K::Func => function_decl(p),
        K::Subscript => subscript_decl(p),
        K::Let | K::Var => field_decl(p),
        K::Type => type_alias_decl(p),
        _ => unreachable!("Ctx::allows admitted {kw:?} in {:?}", ctx as u8),
    }
}

// ----- shared pieces ---------------------------------------------------------------

/// `Visibility` node (empty when no keyword is present).
fn visibility(p: &mut Parser<'_>) {
    let m = p.start();
    if p.at_any(VISIBILITY) {
        p.bump_any();
    }
    m.complete(p, K::Visibility);
}

/// Report and skip modifiers that do not apply to this kind of item.
fn stray_modifiers(p: &mut Parser<'_>) {
    while p.at_any(MODIFIERS) {
        let range = p.error_range();
        let found = p.current();
        p.push_error(SyntaxError::new(
            crate::syntax_error::codes::UNEXPECTED_TOKEN,
            format!(
                "{} is not allowed here",
                crate::syntax_error::describe(found)
            ),
            range,
        ));
        p.err_bump();
    }
}

/// `StaticModifier` node, if `static` is next.
fn opt_static(p: &mut Parser<'_>) {
    if p.at(K::Static) {
        let m = p.start();
        p.bump(K::Static);
        m.complete(p, K::StaticModifier);
    }
}

/// `ParameterList`: `( param, … )`.
fn parameter_list(p: &mut Parser<'_>) {
    let m = p.start();
    if p.at(K::LParen) {
        delimited(p, K::LParen, K::RParen, true, parameter);
    } else {
        p.error_expected(&[K::LParen]);
    }
    m.complete(p, K::ParameterList);
}

/// `mutating|consuming? label? pattern : Ty (= default)?`
fn parameter(p: &mut Parser<'_>) -> bool {
    let m = p.start();
    if p.at_any(&[K::Mutating, K::Consuming]) {
        p.bump_any();
    }
    let labelled = p
        .current()
        .is_some_and(|k| k == K::Identifier || is_label_keyword(k))
        && p.at_any_nth(1, &[K::Identifier, K::Underscore, K::LParen, K::Var]);
    if labelled {
        let n = p.start();
        p.bump_as(K::Identifier);
        n.complete(p, K::Name);
    }
    if !at_param_pattern_start(p) {
        p.error_expected_what("parameter");
        m.complete(p, K::Parameter);
        return false;
    }
    param_pattern(p);
    p.expect(K::Colon);
    ty(p);
    if p.at(K::Equals) {
        let d = p.start();
        p.bump(K::Equals);
        expr(p);
        d.complete(p, K::DefaultValue);
    }
    m.complete(p, K::Parameter);
    true
}

/// `ReturnType`: `-> Ty`, if present.
fn opt_return_type(p: &mut Parser<'_>) {
    if p.at(K::Arrow) {
        let m = p.start();
        p.bump(K::Arrow);
        ty(p);
        m.complete(p, K::ReturnType);
    }
}

/// `FunctionBody` with a block, if `{` is next.
fn opt_block_body(p: &mut Parser<'_>) {
    if p.at(K::LBrace) {
        let m = p.start();
        top_block(p);
        m.complete(p, K::FunctionBody);
    }
}

// ----- declarations ------------------------------------------------------------------

/// `module A.B.C`
fn module_decl(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(K::Module);
    module_path(p);
    m.complete(p, K::ModuleDeclaration);
}

/// `ModulePath`: identifiers joined by dots.
fn module_path(p: &mut Parser<'_>) {
    let m = p.start();
    p.expect(K::Identifier);
    while p.at(K::Dot) && p.nth_at(1, K::Identifier) {
        p.bump(K::Dot);
        p.bump(K::Identifier);
    }
    m.complete(p, K::ModulePath);
}

/// `import A.B`, `import A.B as C`, `import A.B.(X, Y as Z)`
fn import_decl(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(K::Import);
    module_path(p);
    if p.at(K::As) {
        p.bump(K::As);
        p.expect(K::Identifier);
    } else if p.at(K::Dot) && p.nth_at(1, K::LParen) {
        p.bump(K::Dot);
        p.bump(K::LParen);
        loop {
            let item = p.start();
            p.expect(K::Identifier);
            if p.eat(K::As) {
                p.expect(K::Identifier);
            }
            item.complete(p, K::ImportItem);
            if !p.eat(K::Comma) {
                break;
            }
        }
        expect_closer(p, K::RParen);
    }
    m.complete(p, K::ImportDeclaration);
}

/// `extend Ty (: conformances)? (where …)? { members }`
fn extension_decl(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(K::Extend);
    ty(p);
    opt_conformance_list(p);
    opt_where_clause(p);
    member_list(p, Ctx::Extension, K::ExtensionBody);
    m.complete(p, K::ExtensionDeclaration);
}

/// `struct`/`enum`/`protocol` declarations.
fn type_decl(p: &mut Parser<'_>, kw: K) {
    let m = p.start();
    attribute_list(p);
    visibility(p);
    if kw == K::Enum && p.at(K::Indirect) {
        let i = p.start();
        p.bump(K::Indirect);
        i.complete(p, K::IndirectModifier);
    }
    stray_modifiers(p);
    p.expect(kw);
    name(p);
    opt_type_parameter_list(p);
    opt_conformance_list(p);
    opt_where_clause(p);
    let (ctx, body, node) = match kw {
        K::Struct => (Ctx::Struct, K::StructBody, K::StructDeclaration),
        K::Enum => (Ctx::Enum, K::EnumBody, K::EnumDeclaration),
        _ => (Ctx::Protocol, K::ProtocolBody, K::ProtocolDeclaration),
    };
    member_list(p, ctx, body);
    m.complete(p, node);
}

/// `case Name` or `case Name(label: Ty, Ty)`
fn enum_case(p: &mut Parser<'_>) {
    let m = p.start();
    attribute_list(p);
    p.expect(K::Case);
    name(p);
    if p.at(K::LParen) {
        let list = p.start();
        delimited(p, K::LParen, K::RParen, true, |p| {
            let param = p.start();
            let labelled = p
                .current()
                .is_some_and(|k| k == K::Identifier || is_label_keyword(k))
                && p.nth_at(1, K::Colon);
            if labelled {
                let n = p.start();
                p.bump_as(K::Identifier);
                n.complete(p, K::Name);
                p.bump(K::Colon);
            }
            let ok = if at_type_start(p) {
                ty(p).is_some()
            } else {
                p.error_expected_what("type");
                false
            };
            param.complete(p, K::EnumCaseParameter);
            ok
        });
        list.complete(p, K::EnumCaseParameterList);
    }
    m.complete(p, K::EnumCaseDeclaration);
}

/// `init[T](params) (? | throws E)? (where …)? { body }?`
fn init_decl(p: &mut Parser<'_>) {
    let m = p.start();
    attribute_list(p);
    visibility(p);
    stray_modifiers(p);
    p.expect(K::Init);
    opt_type_parameter_list(p);
    parameter_list(p);
    if p.at(K::Question) || p.at(K::Throws) {
        let e = p.start();
        if p.eat(K::Throws) {
            ty(p);
        } else {
            p.bump(K::Question);
        }
        e.complete(p, K::InitEffect);
    }
    opt_where_clause(p);
    opt_block_body(p);
    m.complete(p, K::InitializerDeclaration);
}

/// `deinit { body }`
fn deinit_decl(p: &mut Parser<'_>) {
    let m = p.start();
    p.bump(K::Deinit);
    if p.at(K::LBrace) {
        opt_block_body(p);
    } else {
        p.error_expected(&[K::LBrace]);
    }
    m.complete(p, K::DeinitDeclaration);
}

/// `static? (mutating|consuming)? func name[T](params) (-> R)? (where …)? body?`
fn function_decl(p: &mut Parser<'_>) {
    let m = p.start();
    attribute_list(p);
    visibility(p);
    opt_static(p);
    if p.at_any(&[K::Mutating, K::Consuming]) {
        p.bump_any();
    }
    stray_modifiers(p);
    p.expect(K::Func);
    name(p);
    opt_type_parameter_list(p);
    parameter_list(p);
    opt_return_type(p);
    opt_where_clause(p);
    if p.at(K::Equals) {
        let b = p.start();
        p.bump(K::Equals);
        expr(p);
        b.complete(p, K::FunctionBody);
    } else {
        opt_block_body(p);
    }
    m.complete(p, K::FunctionDeclaration);
}

/// `static? subscript[T](params) -> R (where …)? body`
fn subscript_decl(p: &mut Parser<'_>) {
    let m = p.start();
    attribute_list(p);
    visibility(p);
    opt_static(p);
    stray_modifiers(p);
    p.expect(K::Subscript);
    opt_type_parameter_list(p);
    parameter_list(p);
    if p.at(K::Arrow) {
        opt_return_type(p);
    } else {
        p.error_expected(&[K::Arrow]);
    }
    opt_where_clause(p);
    let b = p.start();
    accessor_body(p, false);
    b.complete(p, K::SubscriptBody);
    m.complete(p, K::SubscriptDeclaration);
}

/// `static? let|var name: Ty accessors? (= expr)? ;?`
fn field_decl(p: &mut Parser<'_>) {
    let m = p.start();
    attribute_list(p);
    visibility(p);
    opt_static(p);
    stray_modifiers(p);
    p.bump_any(); // let / var
    name(p);
    p.expect(K::Colon);
    ty(p);
    if p.at(K::LBrace) {
        accessor_body(p, true);
    }
    if p.at(K::Equals) {
        p.bump(K::Equals);
        expr(p);
    }
    p.eat(K::Semicolon);
    m.complete(p, K::FieldDeclaration);
}

/// The accessor clause keyword(s) at token offset `n`, if a clause starts
/// there: `get {`, `set {`, `ref {`, `mutating ref {`.
fn at_accessor_clause(p: &Parser<'_>, n: usize) -> bool {
    match p.nth(n) {
        Some(K::Get | K::Set) => p.nth_at(n + 1, K::LBrace),
        Some(K::Mutating) => p.nth_is_contextual(n + 1, "ref") && p.nth_at(n + 2, K::LBrace),
        Some(K::Identifier) => p.nth_is_contextual(n, "ref") && p.nth_at(n + 1, K::LBrace),
        _ => false,
    }
}

/// A computed property's or subscript's body: `{ get set }` requirement,
/// `{ get { } set { } }` clauses, or a `{ … }` getter shorthand. A field
/// wraps all three in `PropertyAccessors`; a subscript only the first two
/// (its shorthand block sits directly in `SubscriptBody`).
fn accessor_body(p: &mut Parser<'_>, field: bool) {
    if !p.at(K::LBrace) {
        p.error_expected(&[K::LBrace]);
        return;
    }
    let requirement = p.nth_at(1, K::Get)
        && (p.nth_at(2, K::RBrace) || (p.nth_at(2, K::Set) && p.nth_at(3, K::RBrace)));
    if requirement {
        let m = p.start();
        p.bump(K::LBrace);
        p.bump(K::Get);
        p.eat(K::Set);
        p.bump(K::RBrace);
        m.complete(p, K::PropertyAccessors);
        return;
    }
    if at_accessor_clause(p, 1) {
        let m = p.start();
        p.bump(K::LBrace);
        while at_accessor_clause(p, 0) {
            accessor_clause(p);
        }
        expect_closer(p, K::RBrace);
        m.complete(p, K::PropertyAccessors);
        return;
    }
    if field {
        let m = p.start();
        top_block(p);
        m.complete(p, K::PropertyAccessors);
    } else {
        top_block(p);
    }
}

fn accessor_clause(p: &mut Parser<'_>) {
    let m = p.start();
    let kind = match p.current() {
        Some(K::Get) => {
            p.bump(K::Get);
            K::GetterClause
        },
        Some(K::Set) => {
            p.bump(K::Set);
            K::SetterClause
        },
        Some(K::Mutating) => {
            p.bump(K::Mutating);
            p.bump(K::Identifier);
            K::MutatingRefClause
        },
        _ => {
            p.bump(K::Identifier);
            K::RefClause
        },
    };
    top_block(p);
    m.complete(p, kind);
}

/// `type Name[T]? (: bounds)? (where …)? (= Ty)? ;?` — also associated
/// types (`type Item: P`) and qualified bindings (`type P[Int].Item = X`).
fn type_alias_decl(p: &mut Parser<'_>) {
    let m = p.start();
    attribute_list(p);
    visibility(p);
    stray_modifiers(p);
    p.expect(K::Type);
    type_alias_target(p);
    opt_type_parameter_list(p);
    if p.at(K::Colon) {
        let b = p.start();
        p.bump(K::Colon);
        loop {
            let item = p.start();
            ty(p);
            item.complete(p, K::ConformanceItem);
            if !p.eat(K::Comma) {
                break;
            }
        }
        b.complete(p, K::ConformanceList);
    }
    opt_where_clause(p);
    if p.at(K::Equals) {
        p.bump(K::Equals);
        let a = p.start();
        ty(p);
        a.complete(p, K::AliasedType);
    }
    p.eat(K::Semicolon);
    m.complete(p, K::TypeAliasDeclaration);
}

/// `Name`, or `AssociatedTypeTarget` for `Proto.Name` / `Proto[Args].Name`.
fn type_alias_target(p: &mut Parser<'_>) {
    if !p.at(K::Identifier) {
        p.error_expected(&[K::Identifier]);
        return;
    }
    let here = p.token_pos();
    let qualified = (p.nth_at(1, K::Dot) && p.nth_at(2, K::Identifier))
        || (p.nth_at(1, K::LBracket)
            && p.joined_at(here + 1)
            && p.closer_of(here + 1).is_some_and(|c| {
                p.kind_at(c + 1) == Some(K::Dot) && p.kind_at(c + 2) == Some(K::Identifier)
            }));
    if !qualified {
        name(p);
        return;
    }
    let m = p.start();
    let t = p.start();
    let tp = p.start();
    let path = p.start();
    let e = p.start();
    p.bump(K::Identifier);
    e.complete(p, K::PathElement);
    path.complete(p, K::Path);
    if p.at(K::LBracket) {
        type_argument_list(p);
    }
    tp.complete(p, K::TyPath);
    t.complete(p, K::Ty);
    p.bump(K::Dot);
    name(p);
    m.complete(p, K::AssociatedTypeTarget);
}
