//! Parameter extraction from `ParameterList` views.
//!
//! `Parameter = ('mutating' | 'consuming')? Name? Pattern ':' Ty DefaultValue?`
//! — the `Name` is the argument label; the pattern binds the parameter (a
//! plain `BindingPattern` gives its name, anything else gets a synthetic
//! `_param_N` name and a `ParamPattern`).

use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::SyntaxNodePtr;
use kestrel_syntax_tree::ast::{self, AstNode};

use crate::ast_type::lower_opt_type;
use crate::components::{
    AstParam, FileId, NodeKind, ParamPattern, StructPatternField, TypeAnnotation, Valued,
};

/// Extract the parameters of `list`. Creates child entities for default
/// value expressions.
pub fn extract_params(
    world: &mut World,
    list: Option<ast::ParameterList>,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
) -> Vec<AstParam> {
    let Some(list) = list else {
        return Vec::new();
    };

    // Synthetic-name counter for destructured params in **this** parameter list.
    //
    // Names are `_param_{idx}` and they are user-visible: E611 and E613 print
    // them verbatim (e.g. "required parameter '_param_0' cannot follow
    // parameter 'a' which has a default value").
    //
    // This MUST stay a local. It used to be a process-lifetime
    // `static AtomicU32`, which made the name depend on how many declarations
    // the process had already built rather than on the source: the LSP holds one
    // long-lived `Compiler`, so the *same unedited* file reported `_param_0`,
    // then `_param_7`, then `_param_23` across rebuilds, and the test harness
    // runs every test in one process on parallel threads, so the number was a
    // race. Scoping it here makes the name a pure function of the parameter
    // list. See `lib/kestrel-ast-builder/AGENTS.md` (F43b).
    let mut synth_idx: u32 = 0;

    let params: Vec<AstParam> = list
        .parameters()
        .filter_map(|param| {
            extract_single_param(world, &param, parent, file_entity, file_id, &mut synth_idx)
        })
        .collect();

    // Detect defaults that reference sibling params. The marker component
    // lets the analyzer emit a proper diagnostic, and makes body lowering
    // treat the default as empty (suppressing "undefined name" from
    // inference).
    let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
    for param in &params {
        let Some(default_entity) = param.default_entity else {
            continue;
        };
        let Some(default) =
            crate::syntax::valued_node(&*world, default_entity).and_then(ast::DefaultValue::cast)
        else {
            continue;
        };
        if let Some(referenced) = default_references_param(&default, &param_names) {
            world.set(
                default_entity,
                crate::components::DefaultReferencesParam(referenced.to_string()),
            );
        }
    }

    params
}

/// Extract a single parameter.
///
/// `synth_idx` is the caller's per-parameter-list counter for synthetic
/// `_param_N` names; see [`extract_params`].
fn extract_single_param(
    world: &mut World,
    param: &ast::Parameter,
    parent: Entity,
    file_entity: Entity,
    file_id: usize,
    synth_idx: &mut u32,
) -> Option<AstParam> {
    let is_consuming = param.consuming_token().is_some();
    let is_mut = is_consuming || param.mutating_token().is_some();

    let pat = param.pattern()?.pat()?;
    // A plain binding names the parameter; a destructuring pattern gets a
    // synthetic name.
    let (name, pattern) = match &pat {
        ast::Pat::BindingPattern(b) => (b.identifier_token()?.text().to_string(), None),
        _ => {
            let param_pat = extract_param_pattern(&pat)?;
            let idx = *synth_idx;
            *synth_idx += 1;
            (format!("_param_{idx}"), Some(param_pat))
        },
    };
    let label = param.name().and_then(|n| n.text());
    let ty = lower_opt_type(param.ty(), file_id);

    // Create child entity for default value expression
    let default_entity = param.default_value().map(|default| {
        let entity = world.spawn();
        world.set(entity, NodeKind::ParamDefault);
        world.set(entity, FileId(file_entity));
        world.set(entity, Valued(SyntaxNodePtr::new(default.syntax())));
        // Store the param's type annotation so inference checks the default against it
        if let Some(ref param_ty) = ty {
            world.set(entity, TypeAnnotation(param_ty.clone()));
        }
        world.set_parent(entity, parent);
        entity
    });

    Some(AstParam {
        label,
        name,
        ty,
        is_consuming,
        default_entity,
        pattern,
        is_mut,
    })
}

/// The `ParamPattern` of a destructuring parameter: tuple, struct, wildcard,
/// or (nested) binding. Other patterns are not valid parameter patterns.
fn extract_param_pattern(pat: &ast::Pat) -> Option<ParamPattern> {
    Some(match pat {
        ast::Pat::WildcardPattern(_) => ParamPattern::Wildcard,
        ast::Pat::BindingPattern(b) => ParamPattern::Binding {
            name: b
                .identifier_token()
                .map(|t| t.text().to_string())
                .unwrap_or_default(),
            is_mut: b.var_token().is_some(),
        },
        ast::Pat::TuplePattern(t) => ParamPattern::Tuple {
            elements: t
                .tuple_pattern_elements()
                .filter_map(|e| extract_param_pattern(&e.pat()?))
                .collect(),
        },
        ast::Pat::StructPattern(s) => {
            let mut fields = Vec::new();
            let mut has_rest = false;
            for member in s.struct_pattern_members() {
                match member {
                    ast::StructPatternMember::StructPatternRest(_) => has_rest = true,
                    ast::StructPatternMember::StructPatternField(f) => {
                        let Some(field_name) = f.identifier_token().map(|t| t.text().to_string())
                        else {
                            continue;
                        };
                        // `field: pattern`, or the shorthand `field` binding
                        // its own name.
                        let pattern = f.pat().and_then(|p| extract_param_pattern(&p)).unwrap_or(
                            ParamPattern::Binding {
                                name: field_name.clone(),
                                is_mut: false,
                            },
                        );
                        fields.push(StructPatternField {
                            field_name,
                            pattern,
                        });
                    },
                }
            }
            ParamPattern::Struct {
                type_name: s
                    .identifier_token()
                    .map(|t| t.text().to_string())
                    .unwrap_or_default(),
                fields,
                has_rest,
            }
        },
        _ => return None,
    })
}

/// Whether a default value is exactly a bare name (`= x` — one identifier,
/// no type arguments, no member access) matching a sibling parameter.
/// Returns the matched name if found.
fn default_references_param<'a>(
    default: &ast::DefaultValue,
    param_names: &[&'a str],
) -> Option<&'a str> {
    let ast::Expr::ExprPath(path) = default.expression()?.expr()? else {
        return None;
    };
    if path.expression().is_some()
        || path.dot_token().is_some()
        || path.type_argument_lists().next().is_some()
    {
        return None;
    }
    let name = path.identifier_token()?;
    param_names.iter().find(|&&p| p == name.text()).copied()
}

#[cfg(test)]
mod tests {
    use crate::build::build_declarations;
    use crate::components::{Callable, Name, NodeKind};
    use kestrel_hecs::World;

    /// Parse `source`, build declarations into a **fresh** World, and return
    /// the synthetic parameter names of every callable, in declaration order.
    fn synthetic_param_names(source: &str) -> Vec<String> {
        let mut world = World::new();
        world.begin_revision();

        let root = world.spawn();
        world.set(root, NodeKind::Module);
        world.set(root, Name(Name::ROOT.to_string()));

        let file_entity = world.spawn();
        let tokens: Vec<_> = kestrel_lexer::lex(source, file_entity.index())
            .filter_map(|r| r.ok())
            .collect();
        let token_iter = tokens.iter().map(|t| (t.value.clone(), t.span.clone()));
        let result = kestrel_parser::parse_source_file_from_source(source, token_iter);
        build_declarations(&mut world, file_entity, &result.tree(), root, None);

        world
            .iter_component::<Callable>()
            .flat_map(|(_, c)| c.params.iter())
            .filter(|p| p.name.starts_with("_param_"))
            .map(|p| p.name.clone())
            .collect()
    }

    /// Two destructured params in ONE parameter list number from zero, in
    /// source order.
    #[test]
    fn synthetic_param_names_number_from_zero_within_one_list() {
        let names =
            synthetic_param_names("func f((a, b): (Int64, Int64), (c, d): (Int64, Int64)) {}\n");
        assert_eq!(
            names,
            vec!["_param_0".to_string(), "_param_1".to_string()],
            "synthetic names must be positional within the parameter list"
        );
    }

    /// The counter must NOT carry over between calls: building the same source
    /// twice in one process must produce the same names both times.
    ///
    /// This is the regression guard for F43b. `_param_N` reaches user-visible
    /// diagnostic text (E611/E613); a process-lifetime `static AtomicU32` made
    /// it depend on how many declarations the process had already built, so the
    /// LSP printed `_param_0`, then `_param_7`, then `_param_23` for the same
    /// unedited file, and the parallel test harness raced on it. Reintroducing
    /// a `static` fails here.
    #[test]
    fn synthetic_param_counter_does_not_carry_between_parameter_lists() {
        let source = "func f((a, b): (Int64, Int64)) {}\n";
        let first = synthetic_param_names(source);
        let second = synthetic_param_names(source);

        assert_eq!(first, vec!["_param_0".to_string()]);
        assert_eq!(
            first, second,
            "synthetic parameter names must be a pure function of the \
             parameter list, not of process history"
        );
    }

    /// Separate declarations each restart at zero — the scope is one parameter
    /// list, not one file and not one process.
    #[test]
    fn each_declaration_restarts_synthetic_numbering() {
        let names = synthetic_param_names(
            "func f((a, b): (Int64, Int64)) {}\nfunc g((c, d): (Int64, Int64)) {}\n",
        );
        assert_eq!(
            names,
            vec!["_param_0".to_string(), "_param_0".to_string()],
            "each parameter list numbers independently"
        );
    }
}
