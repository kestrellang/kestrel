//! Place (memory location) compilation.

use crate::context::CodegenContext;
use crate::error::CodegenError;

use kestrel_execution_graph::{Id, Local, Place, PlaceKind};

use cranelift_codegen::ir::{InstBuilder, Value as CraneliftValue};
use cranelift_frontend::{FunctionBuilder, Variable};

use std::collections::HashMap;

/// Read a value from a place.
pub fn compile_place_read(
    ctx: &CodegenContext<'_>,
    place: &Place,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<CraneliftValue, CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
            Ok(builder.use_var(*var))
        }

        PlaceKind::Field { parent, name } => {
            // TODO: Implement field access
            Err(CodegenError::Unsupported("field access".to_string()))
        }

        PlaceKind::Index { parent, index } => {
            // TODO: Implement tuple/array indexing
            Err(CodegenError::Unsupported("index access".to_string()))
        }

        PlaceKind::Downcast { parent, variant } => {
            // TODO: Implement enum downcast
            Err(CodegenError::Unsupported("enum downcast".to_string()))
        }

        PlaceKind::Deref(inner) => {
            // TODO: Implement dereference
            Err(CodegenError::Unsupported("dereference".to_string()))
        }
    }
}

/// Write a value to a place.
pub fn compile_place_write(
    ctx: &CodegenContext<'_>,
    place: &Place,
    value: CraneliftValue,
    builder: &mut FunctionBuilder<'_>,
    local_map: &HashMap<Id<Local>, Variable>,
) -> Result<(), CodegenError> {
    match &place.kind {
        PlaceKind::Local(local_id) => {
            let var = local_map
                .get(local_id)
                .ok_or_else(|| CodegenError::Unsupported("unknown local".to_string()))?;
            builder.def_var(*var, value);
            Ok(())
        }

        PlaceKind::Field { .. } => Err(CodegenError::Unsupported("field write".to_string())),

        PlaceKind::Index { .. } => Err(CodegenError::Unsupported("index write".to_string())),

        PlaceKind::Downcast { .. } => Err(CodegenError::Unsupported("downcast write".to_string())),

        PlaceKind::Deref(_) => Err(CodegenError::Unsupported("deref write".to_string())),
    }
}
