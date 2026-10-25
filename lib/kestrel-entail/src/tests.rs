//! Kernel tests over a toy world: `Box[T]` with `extend Box[T]: Show where
//! T: Show`, `Int64: Show`, `String` (no Show), and `extend Box[T] where
//! T = Int64`.

use super::*;

fn e(n: u32) -> Entity {
    Entity::from_raw(n)
}

const BOX: u32 = 1;
const INT: u32 = 2;
const STR: u32 = 3;
const SHOW: u32 = 10;
const EQUATABLE: u32 = 11;
const COMPARABLE: u32 = 12;
const T: u32 = 20;
const ITEM: u32 = 30;

fn nom(n: u32, args: Vec<ETy>) -> ETy {
    ETy::Nominal { entity: e(n), args }
}

struct World;

/// Literal kind 0 = integer: admits only Int64 in this world.
impl EntailLayer for World {
    fn literal_admits(&self, kind: u32, entity: Entity) -> bool {
        kind != 0 || entity == e(INT)
    }

    fn supply(&self, ty: &ETy, protocol: Entity) -> Supply {
        let ETy::Nominal { entity, .. } = ty else {
            return Supply::Decided(Tri::Fails);
        };
        let p = protocol.index() as u32;
        match (entity.index() as u32, p) {
            (INT, SHOW) | (INT, EQUATABLE) => Supply::Sources(vec![Source::Unconditional]),
            // Comparable refines Equatable's supply in reverse for the test:
            // Int64: Comparable via the type decl.
            (INT, COMPARABLE) => Supply::Sources(vec![Source::Unconditional]),
            (BOX, SHOW) => Supply::Sources(vec![Source::Extension {
                target_args: vec![ETy::Param(e(T))],
                clauses: vec![Goal::Conforms {
                    ty: ETy::Param(e(T)),
                    protocol: e(SHOW),
                }],
            }]),
            // Box: Equatable is supplied through Comparable (refinement).
            (BOX, EQUATABLE) => Supply::Sources(vec![Source::Refines(e(COMPARABLE))]),
            (BOX, COMPARABLE) => Supply::Sources(vec![Source::Extension {
                target_args: vec![ETy::Param(e(T))],
                clauses: vec![Goal::Conforms {
                    ty: ETy::Param(e(T)),
                    protocol: e(COMPARABLE),
                }],
            }]),
            _ => Supply::Sources(vec![]),
        }
    }

    fn assoc_binding(&self, base: &ETy, assoc: Entity) -> Option<ETy> {
        // Box[T].Item = T
        match base {
            ETy::Nominal { entity, args } if entity.index() as u32 == BOX && assoc == e(ITEM) => {
                args.first().cloned()
            },
            _ => None,
        }
    }
}

fn conforms(ty: ETy, p: u32) -> Tri {
    entails(&World, &Goal::Conforms { ty, protocol: e(p) })
}

#[test]
fn nominal_and_conditional_conformance() {
    assert_eq!(conforms(nom(INT, vec![]), SHOW), Tri::Holds);
    assert_eq!(conforms(nom(STR, vec![]), SHOW), Tri::Fails);
    assert_eq!(conforms(nom(BOX, vec![nom(INT, vec![])]), SHOW), Tri::Holds);
    assert_eq!(conforms(nom(BOX, vec![nom(STR, vec![])]), SHOW), Tri::Fails);
}

#[test]
fn open_var_is_unknown_never_permit() {
    // N1: the element type of an inline literal is still open.
    assert_eq!(conforms(nom(BOX, vec![ETy::Var]), SHOW), Tri::Unknown);
    assert_eq!(conforms(ETy::Var, SHOW), Tri::Unknown);
}

#[test]
fn poison_holds() {
    assert_eq!(conforms(ETy::Poison, SHOW), Tri::Holds);
    assert_eq!(conforms(nom(BOX, vec![ETy::Poison]), SHOW), Tri::Holds);
}

#[test]
fn refinement_source_recurses() {
    assert_eq!(conforms(nom(BOX, vec![nom(INT, vec![])]), EQUATABLE), Tri::Holds);
    assert_eq!(conforms(nom(BOX, vec![nom(STR, vec![])]), EQUATABLE), Tri::Fails);
}

#[test]
fn equality_clause_gates_applicability() {
    // N2: extend Box[T] where T = Int64
    let clauses = vec![Goal::Equal(ETy::Param(e(T)), nom(INT, vec![]))];
    let target = vec![ETy::Param(e(T))];
    let ok = nom(BOX, vec![nom(INT, vec![])]);
    let bad = nom(BOX, vec![nom(STR, vec![])]);
    let open = nom(BOX, vec![ETy::Var]);
    assert_eq!(extension_applies(&World, &target, &clauses, &ok), Tri::Holds);
    assert_eq!(extension_applies(&World, &target, &clauses, &bad), Tri::Fails);
    assert_eq!(extension_applies(&World, &target, &clauses, &open), Tri::Unknown);
}

#[test]
fn specialized_target_pattern() {
    // extend Box[Int64]
    let target = vec![nom(INT, vec![])];
    assert_eq!(
        extension_applies(&World, &target, &[], &nom(BOX, vec![nom(INT, vec![])])),
        Tri::Holds
    );
    assert_eq!(
        extension_applies(&World, &target, &[], &nom(BOX, vec![nom(STR, vec![])])),
        Tri::Fails
    );
    assert_eq!(
        extension_applies(&World, &target, &[], &nom(BOX, vec![ETy::Var])),
        Tri::Unknown
    );
}

#[test]
fn literal_var_prunes_impossible_patterns() {
    let int_target = vec![nom(INT, vec![])];
    let str_target = vec![nom(STR, vec![])];
    let recv = nom(BOX, vec![ETy::LitVar(0)]);
    assert_eq!(extension_applies(&World, &int_target, &[], &recv), Tri::Unknown);
    assert_eq!(extension_applies(&World, &str_target, &[], &recv), Tri::Fails);
}

#[test]
fn self_and_projection_subjects() {
    // where Self.Item = Int64, judged at Box[Int64] / Box[String].
    let clauses = vec![Goal::Equal(
        ETy::Proj {
            base: Box::new(ETy::SelfTy),
            assoc: e(ITEM),
        },
        nom(INT, vec![]),
    )];
    let target = vec![ETy::Param(e(T))];
    assert_eq!(
        extension_applies(&World, &target, &clauses, &nom(BOX, vec![nom(INT, vec![])])),
        Tri::Holds
    );
    assert_eq!(
        extension_applies(&World, &target, &clauses, &nom(BOX, vec![nom(STR, vec![])])),
        Tri::Fails
    );
}

#[test]
fn not_conforms() {
    let g = |ty| entails(&World, &Goal::NotConforms { ty, protocol: e(SHOW) });
    assert_eq!(g(nom(STR, vec![])), Tri::Holds);
    assert_eq!(g(nom(INT, vec![])), Tri::Fails);
    assert_eq!(g(ETy::Var), Tri::Unknown);
}

#[test]
fn specificity() {
    let generic = vec![ETy::Param(e(T))];
    let special = vec![nom(INT, vec![])];
    assert!(more_specific(&special, 0, &generic, 0));
    assert!(!more_specific(&generic, 0, &special, 0));
    // Same pattern: more clauses win.
    assert!(more_specific(&generic, 1, &generic, 0));
    assert!(!more_specific(&generic, 0, &generic, 0));
}

#[test]
fn tri_algebra() {
    assert_eq!(Tri::all([]), Tri::Holds);
    assert_eq!(Tri::any([]), Tri::Fails);
    assert_eq!(Tri::all([Tri::Holds, Tri::Unknown]), Tri::Unknown);
    assert_eq!(Tri::all([Tri::Unknown, Tri::Fails]), Tri::Fails);
    assert_eq!(Tri::any([Tri::Fails, Tri::Unknown]), Tri::Unknown);
    assert_eq!(Tri::any([Tri::Unknown, Tri::Holds]), Tri::Holds);
}
