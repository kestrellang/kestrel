//! Where-clause entailment — ONE three-valued kernel, N data sources.
//!
//! Every layer that asks "does type `T` satisfy protocol `P`?" or "do this
//! extension's `where` clauses hold for receiver `R`?" — the inference solver
//! (over `TyVar`), the analyzer (over `HirTy`) and mono (over `TyId`) — shares
//! the logic here (bidi design §7, D4). Before this crate there were five
//! evaluators with three different answers to "unknown" (G14), and equality
//! clauses were skipped as satisfied (N2).
//!
//! The kernel works on its own small type language, [`ETy`]. Each layer
//! converts its types into `ETy` and answers world facts through
//! [`EntailLayer`]: which declarations supply a conformance, what an
//! associated type is bound to, and structural questions only it can answer
//! (`Static`, `Copyable`, abstract params). Matching, substitution, the
//! three-valued AND/OR and specificity live here and only here.
//!
//! **Three values, never a guess.**
//! - [`Tri::Holds`] / [`Tri::Fails`] are proofs.
//! - [`Tri::Unknown`] means an open inference variable (or a depth cut-off)
//!   blocks the answer. It is *never* "permit": the solver re-queues the
//!   goal, and callers whose types are always complete (analyze, mono) decide
//!   explicitly what Unknown means for them.
//! - [`ETy::Poison`] (an already-reported error) answers `Holds`, so no
//!   cascade is reported on top of the first diagnostic.
//!
//! Dependency-graph leaf (only `kestrel-hecs` for `Entity`), so mono can use
//! it without pulling in the front end — the `kestrel-copy-fold` precedent.

use kestrel_hecs::Entity;

/// Deepest structural/refinement recursion before the kernel gives up with
/// [`Tri::Unknown`]. Protocol cycles are rejected elsewhere (E459), but the
/// kernel can run before that diagnostic fires.
const MAX_DEPTH: u32 = 32;

/// A three-valued truth: proven, disproven, or blocked on an unknown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tri {
    Holds,
    Fails,
    Unknown,
}

impl Tri {
    pub fn from_bool(b: bool) -> Tri {
        if b { Tri::Holds } else { Tri::Fails }
    }

    pub fn holds(self) -> bool {
        self == Tri::Holds
    }

    pub fn fails(self) -> bool {
        self == Tri::Fails
    }

    /// Logical negation; Unknown stays Unknown.
    pub fn not(self) -> Tri {
        match self {
            Tri::Holds => Tri::Fails,
            Tri::Fails => Tri::Holds,
            Tri::Unknown => Tri::Unknown,
        }
    }

    /// Three-valued AND: Fails dominates, then Unknown.
    pub fn and(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::Fails, _) | (_, Tri::Fails) => Tri::Fails,
            (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
            _ => Tri::Holds,
        }
    }

    /// Three-valued OR: Holds dominates, then Unknown.
    pub fn or(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::Holds, _) | (_, Tri::Holds) => Tri::Holds,
            (Tri::Unknown, _) | (_, Tri::Unknown) => Tri::Unknown,
            _ => Tri::Fails,
        }
    }

    /// AND over an iterator, short-circuiting on the first `Fails`.
    pub fn all(items: impl IntoIterator<Item = Tri>) -> Tri {
        let mut acc = Tri::Holds;
        for t in items {
            acc = acc.and(t);
            if acc == Tri::Fails {
                break;
            }
        }
        acc
    }

    /// OR over an iterator, short-circuiting on the first `Holds`. An empty
    /// iterator is `Fails` (no source supplies the goal).
    pub fn any(items: impl IntoIterator<Item = Tri>) -> Tri {
        let mut acc = Tri::Fails;
        for t in items {
            acc = acc.or(t);
            if acc == Tri::Holds {
                break;
            }
        }
        acc
    }
}

/// The kernel's type language. Layers convert into it; nothing here knows
/// about `HirTy`, `TyVar` or `TyId`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ETy {
    /// A struct, enum, protocol-as-type or synthetic `lang` entity with args.
    Nominal { entity: Entity, args: Vec<ETy> },
    /// `&T` / `&mutating T`. Its conformance args are `[inner]` (the
    /// synthetic `lang.&` entities carry one param, the pointee).
    Ref { inner: Box<ETy>, mutating: bool },
    /// A tuple; `()` is the empty tuple.
    Tuple(Vec<ETy>),
    /// `!`.
    Never,
    /// An abstract type parameter. Its conformances come from the asking
    /// site's assumptions, through [`EntailLayer::supply`].
    Param(Entity),
    /// `Self` in a where clause: substituted with the receiver being judged.
    SelfTy,
    /// `base.assoc`. Reduced through [`EntailLayer::assoc_binding`] when the
    /// base is concrete.
    Proj { base: Box<ETy>, assoc: Entity },
    /// Any other layer-specific type (function, opaque, unreduced alias). The
    /// payload is a layer-owned handle; the kernel never inspects it.
    Abstract(u32),
    /// An open inference variable. Every question touching it is `Unknown`.
    Var,
    /// An open *literal* variable (an integer, float, string … literal whose
    /// type is not fixed yet). Like [`ETy::Var`], except that matching it
    /// against a concrete nominal asks [`EntailLayer::literal_admits`], so a
    /// candidate the literal can never become (`String` for `1`) fails
    /// instead of staying Unknown. The payload is the layer's literal kind.
    LitVar(u32),
    /// An already-reported error. Answers `Holds` so nothing cascades.
    Poison,
}

impl ETy {
    /// The positional args a conformance source's target args match against.
    pub fn args(&self) -> &[ETy] {
        match self {
            ETy::Nominal { args, .. } | ETy::Tuple(args) => args,
            ETy::Ref { inner, .. } => std::slice::from_ref(inner.as_ref()),
            _ => &[],
        }
    }

    /// Does the type mention an open inference variable anywhere?
    pub fn has_var(&self) -> bool {
        match self {
            ETy::Var | ETy::LitVar(_) => true,
            ETy::Nominal { args, .. } | ETy::Tuple(args) => args.iter().any(ETy::has_var),
            ETy::Ref { inner, .. } => inner.has_var(),
            ETy::Proj { base, .. } => base.has_var(),
            _ => false,
        }
    }

    /// Is the head an abstract position (decided by assumptions, not by
    /// declarations)?
    pub fn is_abstract(&self) -> bool {
        matches!(
            self,
            ETy::Param(_) | ETy::SelfTy | ETy::Proj { .. } | ETy::Abstract(_)
        )
    }
}

/// One goal the kernel can decide.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Goal {
    /// `ty: protocol`. Protocol identity is by entity (protocol type args are
    /// pinned separately by the solver's `unify_bound_protocol_args`).
    Conforms { ty: ETy, protocol: Entity },
    /// `ty: not protocol`.
    NotConforms { ty: ETy, protocol: Entity },
    /// `a = b` — equality clauses (`where T = Int64`, `where I.Item = U`).
    Equal(ETy, ETy),
}

/// A declaration that supplies a conformance.
#[derive(Clone, Debug)]
pub enum Source {
    /// The type's own declaration header: holds for every instantiation.
    Unconditional,
    /// An extension on the type. `target_args` is the extension's own target
    /// pattern (`[T, E]` in `extend Result[T, E]`, `[Int64]` in
    /// `extend Box[Int64]`); `clauses` are its where clauses, phrased over
    /// the extension's params and [`ETy::SelfTy`].
    Extension {
        target_args: Vec<ETy>,
        clauses: Vec<Goal>,
    },
    /// Supplied through another protocol (refinement, or a blanket
    /// `extend Parent: P`): holds iff the type satisfies `Parent`.
    Refines(Entity),
}

/// A layer's answer to "who supplies `ty: protocol`?".
#[derive(Clone, Debug)]
pub enum Supply {
    /// Evaluate these sources (empty = not declared at all = `Fails`).
    Sources(Vec<Source>),
    /// The layer decided structurally (Static, Copyable, assumptions on an
    /// abstract param, a shape it cannot classify).
    Decided(Tri),
}

/// The world facts a layer supplies. Everything else is kernel logic.
pub trait EntailLayer {
    /// Who supplies `ty: protocol`. Never called with [`ETy::Var`] or
    /// [`ETy::Poison`]; the kernel answers those itself.
    fn supply(&self, ty: &ETy, protocol: Entity) -> Supply;

    /// The type `base` binds associated type `assoc` to, if it binds one
    /// concretely and unambiguously.
    fn assoc_binding(&self, base: &ETy, assoc: Entity) -> Option<ETy>;

    /// Equality involving an abstract position the structural rule can't
    /// decide (`Param` vs a nominal, two unreduced projections). Layers with
    /// equality assumptions in scope override this.
    fn equal_abstract(&self, _a: &ETy, _b: &ETy) -> Tri {
        Tri::Unknown
    }

    /// Could an open literal of layer kind `kind` become the nominal
    /// `entity`? `false` turns a would-be `Unknown` into `Fails`.
    fn literal_admits(&self, _kind: u32, _entity: Entity) -> bool {
        true
    }
}

/// The verdict on matching an open var against `other`: Unknown, unless the
/// var is a literal that can never become `other`'s head.
fn open_var<L: EntailLayer + ?Sized>(l: &L, var: &ETy, other: &ETy) -> Tri {
    match (var, other) {
        (ETy::LitVar(kind), ETy::Nominal { entity, .. }) if !l.literal_admits(*kind, *entity) => {
            Tri::Fails
        },
        _ => Tri::Unknown,
    }
}

/// Decide `goal`.
pub fn entails<L: EntailLayer + ?Sized>(l: &L, goal: &Goal) -> Tri {
    entails_at(l, goal, 0)
}

/// Do an extension's target pattern and where clauses hold for `recv`?
/// This is member applicability (§5.1 step 4) and conformance-source
/// evaluation in one: the target args must match the receiver's args, and
/// every clause, with the pattern's bindings and `Self := recv` substituted,
/// must hold.
pub fn extension_applies<L: EntailLayer + ?Sized>(
    l: &L,
    target_args: &[ETy],
    clauses: &[Goal],
    recv: &ETy,
) -> Tri {
    applies_at(l, target_args, clauses, recv, 0)
}

/// Is source pattern `a` strictly more specific than `b`? A strictly
/// narrower target pattern wins; at equal pattern specificity, more where
/// clauses win. Shared by the solver's extension selection and mono's
/// witness selection, so the two cannot disagree.
pub fn more_specific(a_args: &[ETy], a_clauses: usize, b_args: &[ETy], b_clauses: usize) -> bool {
    let a_inst_of_b = pattern_instance(b_args, a_args);
    let b_inst_of_a = pattern_instance(a_args, b_args);
    if a_inst_of_b != b_inst_of_a {
        return a_inst_of_b;
    }
    a_clauses > b_clauses
}

fn entails_at<L: EntailLayer + ?Sized>(l: &L, goal: &Goal, depth: u32) -> Tri {
    if depth > MAX_DEPTH {
        return Tri::Unknown;
    }
    match goal {
        Goal::Conforms { ty, protocol } => conforms(l, &normalize(l, ty), *protocol, depth),
        Goal::NotConforms { ty, protocol } => {
            let ty = normalize(l, ty);
            if ty == ETy::Poison {
                return Tri::Holds;
            }
            conforms(l, &ty, *protocol, depth).not()
        },
        Goal::Equal(a, b) => same(l, &normalize(l, a), &normalize(l, b)),
    }
}

fn conforms<L: EntailLayer + ?Sized>(l: &L, ty: &ETy, protocol: Entity, depth: u32) -> Tri {
    match ty {
        ETy::Var | ETy::LitVar(_) => return Tri::Unknown,
        ETy::Poison => return Tri::Holds,
        _ => {},
    }
    let sources = match l.supply(ty, protocol) {
        Supply::Decided(t) => return t,
        Supply::Sources(sources) => sources,
    };
    Tri::any(sources.iter().map(|src| match src {
        Source::Unconditional => Tri::Holds,
        Source::Refines(parent) => {
            if depth + 1 > MAX_DEPTH {
                return Tri::Unknown;
            }
            conforms(l, ty, *parent, depth + 1)
        },
        Source::Extension {
            target_args,
            clauses,
        } => applies_at(l, target_args, clauses, ty, depth + 1),
    }))
}

fn applies_at<L: EntailLayer + ?Sized>(
    l: &L,
    target_args: &[ETy],
    clauses: &[Goal],
    recv: &ETy,
    depth: u32,
) -> Tri {
    if depth > MAX_DEPTH {
        return Tri::Unknown;
    }
    let mut binds: Vec<(Entity, ETy)> = Vec::new();
    let recv_args = recv.args();
    // Args the pattern names but the receiver lacks (arity drift on a
    // malformed type) are skipped, as the layers always did.
    let matched = Tri::all(
        target_args
            .iter()
            .zip(recv_args.iter())
            .map(|(pat, actual)| match_pattern(l, pat, actual, &mut binds)),
    );
    if matched.fails() {
        return Tri::Fails;
    }
    let clauses_hold = Tri::all(clauses.iter().map(|clause| {
        let goal = subst_goal(clause, &binds, recv);
        entails_at(l, &goal, depth + 1)
    }));
    matched.and(clauses_hold)
}

/// Match an extension target pattern against an actual receiver arg,
/// recording `Param` bindings. A repeated param must bind equal types.
fn match_pattern<L: EntailLayer + ?Sized>(
    l: &L,
    pat: &ETy,
    actual: &ETy,
    binds: &mut Vec<(Entity, ETy)>,
) -> Tri {
    if let ETy::Param(p) = pat {
        if let Some((_, bound)) = binds.iter().find(|(e, _)| e == p) {
            let bound = bound.clone();
            return same(l, &bound, actual);
        }
        binds.push((*p, actual.clone()));
        return Tri::Holds;
    }
    match actual {
        ETy::Var | ETy::LitVar(_) => return open_var(l, actual, pat),
        ETy::Poison => return Tri::Holds,
        _ => {},
    }
    match (pat, actual) {
        (
            ETy::Nominal { entity: pe, args: pa },
            ETy::Nominal { entity: ae, args: aa },
        ) => {
            if pe != ae || pa.len() != aa.len() {
                return Tri::Fails;
            }
            Tri::all(pa.iter().zip(aa).map(|(p, a)| match_pattern(l, p, a, binds)))
        },
        (ETy::Tuple(pe), ETy::Tuple(ae)) => {
            if pe.len() != ae.len() {
                return Tri::Fails;
            }
            Tri::all(pe.iter().zip(ae).map(|(p, a)| match_pattern(l, p, a, binds)))
        },
        (
            ETy::Ref { inner: pi, mutating: pm },
            ETy::Ref { inner: ai, mutating: am },
        ) => {
            if pm != am {
                return Tri::Fails;
            }
            match_pattern(l, pi, ai, binds)
        },
        // Non-param abstract patterns (`Self`, projections) compare as types.
        _ => same(l, pat, actual),
    }
}

/// Structural type equality, three-valued.
fn same<L: EntailLayer + ?Sized>(l: &L, a: &ETy, b: &ETy) -> Tri {
    match (a, b) {
        (ETy::Poison, _) | (_, ETy::Poison) => return Tri::Holds,
        (ETy::Var | ETy::LitVar(_), _) => return open_var(l, a, b),
        (_, ETy::Var | ETy::LitVar(_)) => return open_var(l, b, a),
        _ => {},
    }
    match (a, b) {
        (
            ETy::Nominal { entity: ae, args: aa },
            ETy::Nominal { entity: be, args: ba },
        ) => {
            if ae != be || aa.len() != ba.len() {
                return Tri::Fails;
            }
            Tri::all(aa.iter().zip(ba).map(|(x, y)| same(l, x, y)))
        },
        (ETy::Tuple(ae), ETy::Tuple(be)) => {
            if ae.len() != be.len() {
                return Tri::Fails;
            }
            Tri::all(ae.iter().zip(be).map(|(x, y)| same(l, x, y)))
        },
        (
            ETy::Ref { inner: ai, mutating: am },
            ETy::Ref { inner: bi, mutating: bm },
        ) => {
            if am != bm {
                return Tri::Fails;
            }
            same(l, ai, bi)
        },
        (ETy::Never, ETy::Never) | (ETy::SelfTy, ETy::SelfTy) => Tri::Holds,
        (ETy::Param(x), ETy::Param(y)) if x == y => Tri::Holds,
        (ETy::Abstract(x), ETy::Abstract(y)) if x == y => Tri::Holds,
        (
            ETy::Proj { base: ab, assoc: aa },
            ETy::Proj { base: bb, assoc: ba },
        ) if aa == ba && same(l, ab, bb).holds() => Tri::Holds,
        _ if a.is_abstract() || b.is_abstract() => l.equal_abstract(a, b),
        // Two concrete heads of different shapes.
        _ => Tri::Fails,
    }
}

/// Reduce projections whose base is concrete, recursively.
fn normalize<L: EntailLayer + ?Sized>(l: &L, ty: &ETy) -> ETy {
    match ty {
        ETy::Proj { base, assoc } => {
            let base = normalize(l, base);
            match base {
                ETy::Var | ETy::LitVar(_) => ETy::Var,
                ETy::Poison => ETy::Poison,
                _ if !base.is_abstract() => match l.assoc_binding(&base, *assoc) {
                    Some(bound) => normalize(l, &bound),
                    None => ETy::Proj {
                        base: Box::new(base),
                        assoc: *assoc,
                    },
                },
                _ => ETy::Proj {
                    base: Box::new(base),
                    assoc: *assoc,
                },
            }
        },
        ETy::Nominal { entity, args } => ETy::Nominal {
            entity: *entity,
            args: args.iter().map(|a| normalize(l, a)).collect(),
        },
        ETy::Tuple(elems) => ETy::Tuple(elems.iter().map(|a| normalize(l, a)).collect()),
        ETy::Ref { inner, mutating } => ETy::Ref {
            inner: Box::new(normalize(l, inner)),
            mutating: *mutating,
        },
        other => other.clone(),
    }
}

fn subst_goal(goal: &Goal, binds: &[(Entity, ETy)], recv: &ETy) -> Goal {
    match goal {
        Goal::Conforms { ty, protocol } => Goal::Conforms {
            ty: subst(ty, binds, recv),
            protocol: *protocol,
        },
        Goal::NotConforms { ty, protocol } => Goal::NotConforms {
            ty: subst(ty, binds, recv),
            protocol: *protocol,
        },
        Goal::Equal(a, b) => Goal::Equal(subst(a, binds, recv), subst(b, binds, recv)),
    }
}

/// Substitute pattern bindings and `Self := recv`. A param the pattern did
/// not bind stays abstract.
pub fn subst(ty: &ETy, binds: &[(Entity, ETy)], recv: &ETy) -> ETy {
    match ty {
        ETy::Param(p) => binds
            .iter()
            .find(|(e, _)| e == p)
            .map_or_else(|| ty.clone(), |(_, t)| t.clone()),
        ETy::SelfTy => recv.clone(),
        ETy::Nominal { entity, args } => ETy::Nominal {
            entity: *entity,
            args: args.iter().map(|a| subst(a, binds, recv)).collect(),
        },
        ETy::Tuple(elems) => ETy::Tuple(elems.iter().map(|a| subst(a, binds, recv)).collect()),
        ETy::Ref { inner, mutating } => ETy::Ref {
            inner: Box::new(subst(inner, binds, recv)),
            mutating: *mutating,
        },
        ETy::Proj { base, assoc } => ETy::Proj {
            base: Box::new(subst(base, binds, recv)),
            assoc: *assoc,
        },
        other => other.clone(),
    }
}

/// Is `actual` an instance of `pattern` (params in `pattern` match
/// anything, consistently)? Pure structure: used only for specificity.
fn pattern_instance(pattern: &[ETy], actual: &[ETy]) -> bool {
    fn go(p: &ETy, a: &ETy, binds: &mut Vec<(Entity, ETy)>) -> bool {
        if let ETy::Param(e) = p {
            if let Some((_, b)) = binds.iter().find(|(x, _)| x == e) {
                return b == a;
            }
            binds.push((*e, a.clone()));
            return true;
        }
        match (p, a) {
            (ETy::Nominal { entity: pe, args: pa }, ETy::Nominal { entity: ae, args: aa }) => {
                pe == ae && pa.len() == aa.len() && pa.iter().zip(aa).all(|(x, y)| go(x, y, binds))
            },
            (ETy::Tuple(pe), ETy::Tuple(ae)) => {
                pe.len() == ae.len() && pe.iter().zip(ae).all(|(x, y)| go(x, y, binds))
            },
            (ETy::Ref { inner: pi, mutating: pm }, ETy::Ref { inner: ai, mutating: am }) => {
                pm == am && go(pi, ai, binds)
            },
            _ => p == a,
        }
    }
    let mut binds = Vec::new();
    pattern.len() == actual.len() && pattern.iter().zip(actual).all(|(p, a)| go(p, a, &mut binds))
}

#[cfg(test)]
mod tests;
