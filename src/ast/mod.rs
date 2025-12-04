pub mod value;
pub mod nodes;

// Re-export pour faciliter l'accès : use crate::ast::{Value, Instruction, ...}
pub use value::{Value, InstanceData};
pub use nodes::{Expression, Instruction, ClassDefinition};
