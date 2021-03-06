//! This module contains all of the relevant parts for how ASTs are created.
//!
//! The abstract syntax tree should be made up of as few different types as possible, and when
//! instead of adding more fields to a variant of the [node type] enum, instead tags should be added.
//!
//! Tags are a way of adding information to the AST without needing to have many different fields for
//! every single instance of a [JodinNode]
//!
//! [node type]: self::node_type::JodinNodeType
//! [JodinNode]: self::jodin_node::JodinNode

pub use crate::ast::jodin_node::*;
pub use crate::ast::node_type::{CompoundType, JodinNodeType};
use crate::error::JodinResult;
use crate::identifier::Identifier;

mod jodin_node;
mod node_type;

/// parse ids
pub fn parse_identifier() -> JodinResult<Identifier> {
    todo!()
}
