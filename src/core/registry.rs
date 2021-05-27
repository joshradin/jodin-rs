use crate::core::error::{JodinError, JodinResult};
use crate::core::identifier::{Identifier, Namespaced};
use crate::core::identifier_resolution::IdentifierResolver;
use crate::core::types::{CompoundType, JodinTypeReference};
use std::collections::HashMap;
use std::ops::{Deref, Index, IndexMut};

pub struct Registry<T> {
    resolver: IdentifierResolver,
    mapping: HashMap<Identifier, T>,
}

impl<T> Registry<T> {
    pub fn new() -> Self {
        Self {
            resolver: IdentifierResolver::new(),
            mapping: Default::default(),
        }
    }

    pub fn with_resolver(resolver: IdentifierResolver) -> Self {
        Self {
            resolver,
            mapping: Default::default(),
        }
    }

    pub fn insert(&mut self, val: T) -> JodinResult<Identifier>
    where
        T: Namespaced,
    {
        let identifier = val.get_identifier().clone();
        self.insert_with_identifier(val, identifier)
    }

    pub fn insert_with_identifier(&mut self, val: T, path: Identifier) -> JodinResult<Identifier> {
        let path = self.resolver.create_absolute_path(&path);
        if self.mapping.contains_key(&path) {
            return Err(JodinError::IdentifierAlreadyExists(path));
        }
        self.mapping.insert(path.clone(), val);
        Ok(path)
    }

    pub fn update_absolute_identity(&mut self, absolute: &Identifier, val: T) -> JodinResult<&T> {
        if !self.resolver.contains_absolute_identifier(absolute) {
            return Err(JodinError::IdentifierDoesNotExist(absolute.clone()));
        }
        self.mapping.insert(absolute.clone(), val);
        Ok(&self.mapping[absolute])
    }

    /// Pushes a namespace onto the current namespace
    pub fn push_namespace(&mut self, namespace: Identifier) {
        self.resolver.push_namespace(namespace);
    }

    /// Pops the outermost namespace from the current namespace
    pub fn pop_namespace(&mut self) {
        self.resolver.pop_namespace()
    }

    /// Adds a namespace to search from relatively
    pub fn use_namespace(&mut self, namespace: Identifier) -> JodinResult<()> {
        self.resolver.use_namespace(namespace)
    }

    /// Removes a namespace to search from, if it exists
    pub fn stop_use_namespace(&mut self, namespace: &Identifier) -> JodinResult<()> {
        self.resolver.stop_use_namespace(namespace)
    }

    pub fn get(&self, path: &Identifier) -> JodinResult<&T> {
        let full_path = self.resolver.resolve_path(path.clone())?;
        self.mapping
            .get(&full_path)
            .ok_or(JodinError::IdentifierDoesNotExist(path.clone()))
    }

    pub fn get_mut(&mut self, path: &Identifier) -> JodinResult<&mut T> {
        let full_path = self.resolver.resolve_path(path.clone())?;
        self.mapping
            .get_mut(&full_path)
            .ok_or(JodinError::IdentifierDoesNotExist(path.clone()))
    }
}

impl<I: Into<Identifier>, T> Index<I> for Registry<T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.get(&index.into()).unwrap()
    }
}

impl<I: Into<Identifier>, T> IndexMut<I> for Registry<T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.get_mut(&index.into()).unwrap()
    }
}

/// Enables registration of an object to a proper registry. Implementations must include all children into
/// registration
pub trait Registrable<T = Self>: Sized {
    /// Registers both this item and all related children to this registry
    fn register(self, register: &mut Registry<T>) -> JodinResult<Identifier>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::identifier::Identifiable;
    use std::iter::FromIterator;

    #[test]
    fn insert_entries() {
        let mut register = Registry::new();
        register.push_namespace(Identifier::from("std"));
        register.insert_with_identifier(3, Identifier::from("best value"));
        let value = &register[Identifier::from_iter(&["std", "best value"])];
        assert_eq!(*value, 3);

        let mut registry = Registry::new();
        registry.insert(Identifiable::new("val1", 1)).unwrap();
        registry.insert(Identifiable::new("val2", 2)).unwrap();
        registry.insert(Identifiable::new("val3", 3)).unwrap();
    }
}