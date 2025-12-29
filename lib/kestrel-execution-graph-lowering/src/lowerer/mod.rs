//! Item lowering - converts semantic tree items to MIR.

mod enum_lowerer;
mod function;
mod item;
mod protocol;
mod struct_lowerer;
mod witness;

pub use enum_lowerer::lower_enum;
pub use function::lower_function;
pub use item::lower_item;
pub use protocol::lower_protocol;
pub use struct_lowerer::lower_struct;
pub use witness::{generate_witnesses_for_extension, generate_witnesses_for_struct};
