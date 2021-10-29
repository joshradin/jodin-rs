//! Responsible for managing types and translations from intermediate types.
//!
//! Used to determine type checking.

use crate::core::types::intermediate_type::{IntermediateType, TypeSpecifier, TypeTail};
use crate::core::error::{JodinError, JodinErrorType, JodinResult};
use crate::core::identifier::{Identifier, IdentifierChain, IdentifierChainIterator};
use crate::core::types::primitives::Primitive;
use crate::core::types::JodinType;
use std::collections::HashMap;
use std::ops::{Deref, Index};
use crate::ast::JodinNode;

/// Stores a lot of information about types and related identifier
#[derive(Debug, Default)]
pub struct TypeEnvironment<'node> {
    types: HashMap<Identifier, TypeInfo<'node>>,
    impl_types_to_trait_obj: HashMap<Vec<Identifier>, Identifier>
}

pub struct TypeInfo<'node> {
    /// The actual jodin type
    pub jtype: JodinType,
    /// The declaring node (if relevant)
    pub decl_node: Option<&'node JodinNode>
}

impl TypeEnvironment<'_> {
    /// Create a new type environment
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks whether the first argument can be considered the second type
    ///
    /// # Notable checks for type safety
    /// 1. void* is everything
    pub fn loosely_is(&self, my_type: &IntermediateType, target_type: &IntermediateType) -> bool {
        if Self::is_void_ptr(target_type) && Self::is_ptr(my_type) {
            return true;
        }

        if Self::is_ptr(my_type) && Self::is_ptr(target_type) {
            return self.loosely_is(
                &my_type.get_deref().unwrap(),
                &target_type.get_deref().unwrap(),
            );
        }

        false
    }

    /// Gets whether this a void*
    pub fn is_void_ptr(inter: &IntermediateType) -> bool {
        if let IntermediateType {
            is_const: false,
            type_specifier: TypeSpecifier::Primitive(Primitive::Void),
            generics,
            tails,
        } = inter
        {
            generics.is_empty() && &*tails == &[TypeTail::Pointer]
        } else {
            false
        }
    }

    /// Whether this a pointer
    pub fn is_ptr(inter: &IntermediateType) -> bool {
        match inter.tails.last() {
            Some(TypeTail::Pointer) => true,
            Some(TypeTail::Array(_)) => true,
            _ => false,
        }
    }

    pub fn base_type(&self) -> &JodinType {
        todo!()
    }

    pub fn get_type(&self, id: &Identifier) -> JodinResult<&JodinType> {
        self.types
            .get(id)
            .as_ref()
            .map(|info| &info.jtype)
            .ok_or(JodinError::new(JodinErrorType::IdentifierDoesNotExist(
                id.clone(),
            )))
    }

    pub fn chained_get_type(&self, id: &IdentifierChain) -> JodinResult<&JodinType> {
        let mut iter: IdentifierChainIterator = id.into_iter();
        let base = self.get_type(iter.next().unwrap());
        iter.fold(base, |id| id.map(|inner| inner))
    }

    pub fn is_child_type(&self, child: &Identifier, parent: &Identifier) -> bool {
        todo!()
    }
}

impl<'type> Index<&Identifier> for TypeEnvironment<'type> {
    type Output = JodinType;

    fn index(&self, index: &Identifier) -> &Self::Output {
        self.get_type(index).unwrap()
    }
}

impl<'type> Index<&IdentifierChain> for TypeEnvironment<'type>  {
    type Output = ();

    fn index(&self, index: IdentifierChain) -> &Self::Output {
        todo!()
    }
}
