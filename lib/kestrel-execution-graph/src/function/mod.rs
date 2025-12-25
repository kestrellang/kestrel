//! Function internals: locals, places, values, statements, terminators, blocks.

mod local;
mod type_param;
mod place;
mod immediate;
mod value;
mod statement;
mod terminator;
mod basic_block;

pub use local::*;
pub use type_param::*;
pub use place::*;
pub use immediate::*;
pub use value::*;
pub use statement::*;
pub use terminator::*;
pub use basic_block::*;
