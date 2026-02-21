pub mod javascript;
pub mod typescript;
mod common;

pub use javascript::{JavaScript, JsMutationKind};
pub use typescript::{TypeScript, TsMutationKind};
