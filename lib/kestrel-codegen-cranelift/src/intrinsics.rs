//! Runtime intrinsics (panic, string operations, etc.)

use crate::context::CodegenContext;
use crate::error::CodegenError;

use cranelift_codegen::ir::InstBuilder;
use cranelift_frontend::FunctionBuilder;

// TODO: Implement intrinsics
// - panic: print message and abort
// - string operations: StrPtr, StrLen, StrFromParts
// - IntToString
