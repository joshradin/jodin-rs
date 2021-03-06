//! The main method for tracking, then resolving identifiers.

use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};

// use ptree::{write_tree, Style, TreeItem};

use crate::error::{JodinErrorType, JodinResult};
use crate::identifier::{Identifier, IdentifierIterator, Namespaced};
use crate::utility::Tree;

mod _hidden {
    use super::*;

    /// The default base namespace that all identifiers added to the project will be part of.
    const BASE_NAMESPACE: &str = "{base}";

    /// Maintains a [NamespaceTree](self::NamespaceTree), the current namespace,
    /// the base namespace, and all namespaces that are being "used".
    ///
    /// The base namespace should **never** escape the resolver once it's been created. It's only used for
    /// bookkeeping.
    #[derive(Debug, Clone)]
    pub struct IdentifierResolver {
        current_namespace: Option<Identifier>,
        using_namespaces: Vec<Identifier>,
        base_namespace: Identifier,
        namespace_stash: Vec<Identifier>,
        tree: NamespaceTree<Identifier>,
    }

    impl IdentifierResolver {
        /// Creates a new, empty identifier resolver
        pub fn new() -> Self {
            Self::with_base_namespace(BASE_NAMESPACE)
        }

        /// Creates a new, empty identifier resolver with a custom base namespace. This is used instead of
        /// the `BASE_NAMESPACE`
        pub fn with_base_namespace<S: AsRef<str>>(base_namespace: S) -> Self {
            let mut tree = NamespaceTree::new();
            tree.add_namespace(Identifier::from(&base_namespace));
            Self {
                current_namespace: None,
                using_namespaces: vec![],
                base_namespace: base_namespace.as_ref().to_string().into(),
                namespace_stash: vec![],
                tree,
            }
        }

        /// Pushes a namespace onto the current namespace
        pub fn push_namespace(&mut self, namespace: Identifier) {
            let full_path = Identifier::new_concat(self.current_namespace_with_base(), namespace);
            self.tree.add_namespace(full_path.clone());
            self.current_namespace = Some(full_path.strip_highest_parent().unwrap());
            debug!(
                "Current namespace set to {}",
                self.current_namespace_with_base()
            );
        }

        fn current_namespace_with_base(&self) -> Identifier {
            match &self.current_namespace {
                None => self.base_namespace.clone(),
                Some(s) => Identifier::new_concat(&self.base_namespace, s),
            }
        }

        /// Pops the outermost namespace from the current namespace
        pub fn pop_namespace(&mut self) {
            let old = std::mem::replace(&mut self.current_namespace, None);
            if let Some(old) = old {
                self.current_namespace = old.into_parent();
            }
        }

        /// Adds a namespace to search from relatively
        #[must_use]
        pub fn use_namespace(&mut self, namespace: Identifier) -> JodinResult<()> {
            let resolved_set = self.tree.get_namespaces(
                &namespace,
                Some(&self.current_namespace_with_base()),
                &self.base_namespace,
            );
            if resolved_set.is_empty() {
                return Err(JodinErrorType::IdentifierDoesNotExist(namespace))?;
            } else if resolved_set.len() >= 2 {
                return Err(JodinErrorType::AmbiguousIdentifierError {
                    given: namespace,
                    found: Vec::from_iter(resolved_set.into_iter().map(|id| id.clone())),
                })?;
            }
            let resolved = resolved_set.into_iter().next().cloned().unwrap();
            debug!("Added used namespace: {:?}", resolved);
            self.using_namespaces.push(resolved);
            Ok(())
        }

        /// Gets a list of namespaces
        pub fn namespaces(&self) -> Vec<&Identifier> {
            let mut output = vec![];
            for node in self.tree.head.children_prefix() {
                output.push(&node.id);
            }
            output
        }

        /// Removes a namespace to search from, if it exists
        pub fn stop_use_namespace(&mut self, namespace: &Identifier) -> JodinResult<()> {
            let namespace = namespace.clone();
            let resolved_set = self.tree.get_namespaces(
                &namespace,
                self.current_namespace.as_ref(),
                &self.base_namespace,
            );
            if resolved_set.is_empty() {
                error!("No namespace known as {}", namespace);
                error!("Valid namespaces: {:?}", self.namespaces());
                return Err(JodinErrorType::IdentifierDoesNotExist(namespace).into());
            } else if resolved_set.len() >= 2 {
                return Err(JodinErrorType::AmbiguousIdentifierError {
                    given: namespace,
                    found: Vec::from_iter(resolved_set.into_iter().map(|id| id.clone())),
                }
                .into());
            }
            let resolved = resolved_set.into_iter().next().unwrap();
            self.using_namespaces.retain(|id| id != resolved);
            Ok(())
        }

        /// Creates an absolute path based off the current namespace
        pub fn create_absolute_path(&mut self, id: &Identifier) -> Identifier {
            let full_path = Identifier::new_concat(self.current_namespace_with_base(), id);
            trace!("Created abs path {:?}", full_path);
            let parent_path = &**full_path.parent().as_ref().unwrap();
            self.tree.add_namespace(parent_path.clone());
            let objects = self.tree.get_relevant_objects_mut(parent_path).unwrap();
            if !objects.contains(&full_path) {
                objects.push(full_path.clone())
            }
            full_path.strip_highest_parent().unwrap()
        }

        /// Add a new namespace relative to the current namespace to the resolver
        pub fn add_namespace<N: Into<Identifier>>(&mut self, namespace: N) {
            self.tree.add_namespace(Identifier::new_concat(
                self.current_namespace_with_base(),
                namespace,
            ));
        }

        /// Attempts to resolve a single, absolute identifier out of a given path.
        pub fn resolve_path(
            &self,
            path: Identifier,
            keep_highest_parent: bool,
        ) -> JodinResult<Identifier> {
            debug!("Attempting to resolve path {:?}...", path);
            let mut output = HashSet::new();

            let absolute_path = Identifier::new_concat(&self.base_namespace, &path);
            trace!("Checking path as absolute path: {:?}", absolute_path);
            if let Ok(val) = self.tree.get_from_absolute_identifier(&absolute_path) {
                debug!("Found absolute path: {}", absolute_path);
                output.insert(val);
            }
            if self.current_namespace.is_some() {
                let relative_path =
                    Identifier::new_concat(&self.current_namespace_with_base(), &path);
                trace!("Checking path as relative path: {:?}", relative_path);
                if relative_path != absolute_path {
                    if let Ok(val) = self.tree.get_from_absolute_identifier(&relative_path) {
                        debug!(
                            "Found relative path from {current}: {relative:?}",
                            current = self.current_namespace_with_base(),
                            relative = relative_path
                        );
                        output.insert(val);
                    }
                }
            }

            for using in &self.using_namespaces {
                let using_path = Identifier::new_concat(using, &path);
                trace!("Checking path as relative path: {:?}", using_path);
                if let Ok(id) = self.tree.get_from_absolute_identifier(&using_path) {
                    debug!(
                        "Found relative path from {current}: {relative:?}",
                        current = using,
                        relative = using_path
                    );
                    output.insert(id);
                }
            }

            match output.len() {
                0 => Err(JodinErrorType::IdentifierDoesNotExist(path))?,
                1 => {
                    let identifier = output.into_iter().next().cloned().unwrap();
                    debug!("Resolved {:?} -> {:?}", path, identifier);
                    if !keep_highest_parent {
                        Ok(identifier.strip_highest_parent().unwrap())
                    } else {
                        Ok(identifier)
                    }
                }
                _ => Err(JodinErrorType::AmbiguousIdentifierError {
                    given: path,
                    found: Vec::from_iter(
                        output
                            .into_iter()
                            .cloned()
                            .map(|id| id.strip_highest_parent().unwrap()),
                    ),
                })?,
            }
        }

        /// the current namespace.
        pub fn current_namespace(&self) -> Identifier {
            match &self.current_namespace {
                None => Identifier::empty(),
                Some(cur) => cur.clone(),
            }
        }

        /// Checks if the resolver contains the absolute identifier.
        pub fn contains_absolute_identifier(&self, path: &Identifier) -> bool {
            let path = Identifier::new_concat(&self.base_namespace, path);
            self.tree.get_from_absolute_identifier(&path).is_ok()
        }

        /// The semi push should both push the given id, and set the current id as a used namespace.
        pub fn semi_push(&mut self, id: Identifier) {
            let original_current = self.current_namespace.clone();
            if let Some(current) = &original_current {
                self.use_namespace(current.clone()).expect(
                    format!(
                        "Couldn't set current namespace {:?} as used namespace",
                        current
                    )
                    .as_str(),
                );
            }
            self.push_namespace(id);
        }

        /// Pops the current namespace, then releases the now current namespace from the used namespace
        /// collection.
        pub fn semi_pop(&mut self) {
            self.pop_namespace();
            let original_current = self.current_namespace.clone();
            if let Some(current) = &original_current {
                self.stop_use_namespace(current)
                    .expect("namespace not used");
            }
        }

        /// Add an alias
        pub fn add_alias(&mut self, alias: Identifier, absolute_path: &Identifier) {
            let identifier = self.create_absolute_path(&alias);
            let alias_absolute_path = Identifier::new_concat(&self.base_namespace, identifier);
            println!("Alias absolute path: {}", alias_absolute_path);
            let object = self
                .tree
                .mut_from_absolute_identifier(&alias_absolute_path)
                .expect("This value was just made");
            *object = absolute_path.clone();
        }

        /// Gets a reference to the namespace tree used by this resolver
        pub fn namespace_tree(&self) -> &NamespaceTree<Identifier> {
            &self.tree
        }

        /// Get the base namespace
        pub fn base_namespace(&self) -> &Identifier {
            &self.base_namespace
        }
    }
}

use crate::core::privacy::Visibility;
pub use _hidden::IdentifierResolver;

#[derive(Clone)]
struct Node<T: Namespaced> {
    id: Identifier,
    children: Vec<Node<T>>,
    related_values: Vec<T>,
}

impl<T: Namespaced> Node<T> {
    fn new(id: Identifier) -> Self {
        Node {
            id,
            children: vec![],
            related_values: vec![],
        }
    }

    fn add_child(&mut self, node: Self) {
        self.children.push(node)
    }

    #[allow(unused)]
    fn add_related_value(&mut self, related: T) {
        self.related_values.push(related)
    }

    pub fn id(&self) -> &Identifier {
        &self.id
    }
    pub fn children(&self) -> &Vec<Node<T>> {
        &self.children
    }
    pub fn related_values(&self) -> &Vec<T> {
        &self.related_values
    }
    pub fn children_mut(&mut self) -> Vec<&mut Node<T>> {
        self.children.iter_mut().collect()
    }
    pub fn related_values_mut(&mut self) -> &mut Vec<T> {
        &mut self.related_values
    }
}

/// Creates a tree of namespaces that allow for resolution by searching
#[derive(Clone)]
pub struct NamespaceTree<T: Namespaced> {
    head: Node<T>,
}

impl<T: Namespaced> NamespaceTree<T> {
    /// Creates a new namespace tree that's completely empty
    pub fn new() -> Self {
        Self {
            head: Node::new(Identifier::from("{base}")),
        }
    }

    /// Creates a new namespace tree that's completely empty
    pub fn new_with_initial_namespace(id: Identifier) -> Self {
        Self {
            head: Node::new(id),
        }
    }

    #[allow(unused)]
    fn top_namespaces(&self) -> &Vec<Node<T>> {
        self.head.children()
    }

    fn get_node(&self, namespace: &Identifier) -> Option<&Node<T>> {
        if let Some(parent) = namespace.parent() {
            let parent = self.get_node(parent);
            if parent.is_none() {
                return None;
            }
            for child in parent.unwrap().children() {
                if child.id() == namespace {
                    return Some(child);
                }
            }
        } else {
            for namespace_node in self.head.children() {
                if namespace_node.id() == namespace {
                    return Some(namespace_node);
                }
            }
        }
        None
    }

    fn get_node_mut(&mut self, namespace: &Identifier) -> Option<&mut Node<T>> {
        if let Some(parent) = namespace.parent() {
            let parent = self.get_node_mut(parent);
            if parent.is_none() {
                return None;
            }
            for child in parent.unwrap().children_mut() {
                if child.id() == namespace {
                    return Some(child);
                }
            }
        } else {
            for namespace_node in self.head.children_mut() {
                if namespace_node.id() == namespace {
                    return Some(namespace_node);
                }
            }
        }
        None
    }

    /// Checks if an absolute namespace exists
    pub fn namespace_exists(&self, namespace: &Identifier) -> bool {
        self.get_node(namespace).is_some()
    }

    /// Gets possible absolute namespaces given a path and a current absolute namespace. The queried path will
    /// be treated as both relative and absolute
    pub fn get_namespaces(
        &self,
        path: &Identifier,
        current_namespace: Option<&Identifier>,
        base_namespace: &Identifier,
    ) -> HashSet<&Identifier> {
        //println!("Attempting to find namespace {}", path);
        debug!("Trying to find namespace {}", path);
        trace!(
            "Trying to find possible namespaces with id {:?} (from: {:?})",
            path,
            current_namespace
        );
        if let Some(_current) = current_namespace {
            //println!("Using {} as current namespace", current);
        }
        let mut output = HashSet::new();
        let abs_path = base_namespace / path;
        debug!("Searching for absolute namespace {}...", abs_path);
        if let Some(abs) = self.get_namespace_absolute(&abs_path) {
            debug!("Absolute found.");
            output.insert(abs.id());
        }
        debug!("Searching for a relative path...");
        if let Some(current) = current_namespace {
            if let Some(current_node) = self.get_namespace_absolute(current) {
                let mut iter: IdentifierIterator = path.into_iter();
                let mut ptr = current_node;
                let mut found = true;
                while let Some(lookahead) = iter.next() {
                    for child in ptr.children() {
                        if child.id().this() == lookahead {
                            ptr = child;
                            continue;
                        }
                    }
                    found = false;
                    break;
                }
                if found && iter.next().is_none() {
                    trace!("Found {}.", ptr.id());
                    output.insert(ptr.id());
                }
            }
        }
        debug!("Found possible namespaces: {:?}", output);
        output
    }

    fn get_namespace_absolute(&self, namespace: &Identifier) -> Option<&Node<T>> {
        let mut iter: IdentifierIterator = namespace.into_iter();
        let mut ptr = &self.head;

        'outer: while let Some(lookahead) = iter.next() {
            //println!("lookahead: {}", lookahead);
            for child in ptr.children() {
                //println!("Child: {}", child.id);
                if child.id().this() == lookahead {
                    ptr = child;
                    continue 'outer;
                }
            }
            return None;
        }
        if iter.next().is_none() {
            Some(ptr)
        } else {
            None
        }
    }

    /// Get the associated, relevant objects for an absolute path
    pub fn get_relevant_objects(&self, absolute_path: &Identifier) -> Option<&Vec<T>> {
        self.get_node(absolute_path)
            .map(|node| node.related_values())
    }

    /// Gets mutable references to the associated, relevant objects for an absolute path
    pub fn get_relevant_objects_mut(&mut self, absolute_path: &Identifier) -> Option<&mut Vec<T>> {
        self.get_node_mut(absolute_path)
            .map(|node| node.related_values_mut())
    }

    /// Adds a new namespace to the namespace tree.
    pub fn add_namespace(&mut self, namespace: Identifier) {
        if self.namespace_exists(&namespace) {
            return;
        }
        if let Some(parent) = namespace.parent() {
            if !self.namespace_exists(parent) {
                self.add_namespace(parent.clone())
            }
            self.get_node_mut(parent)
                .unwrap()
                .add_child(Node::new(namespace))
        } else {
            self.head.add_child(Node::new(namespace))
        }
    }

    /// Gets the base associated objects
    pub fn get_base_values(&self) -> &Vec<T> {
        &self.head.related_values
    }

    /// Gets a mutable reference to the base associated objects.
    pub fn get_base_values_mut(&mut self) -> &mut Vec<T> {
        &mut self.head.related_values
    }

    /// Attempts to get the associated value from an absolute path.
    ///
    /// # Arguments
    ///
    /// * `path`: The absolute path
    ///
    /// returns: Result<&T, JodinError> the associated value, or an error
    pub fn get_from_absolute_identifier(&self, path: &Identifier) -> JodinResult<&T> {
        let mut ptr = &self.head;
        let names: Vec<String> = path.into_iter().collect();

        let namespaces = &names[..names.len() - 1];
        trace!("Searching through namespaces {:?}", namespaces);
        for name in namespaces {
            /*
            if ptr.id() != name {
                return Err(IdentifierDoesNotExist(path.clone()));
            }

             */
            let mut found = false;
            trace!("At node {:?} out of {:?}", NodeInfo::from(ptr), path);
            for child in ptr.children() {
                {
                    let node = NodeInfo::from(child);
                    trace!(
                        "Checking {node:?}.id.this() = {this} is equal to {name}",
                        node = node.id,
                        this = node.id.this(),
                        name = name
                    );
                }
                if child.id().this() == name {
                    trace!("found!, setting ptr to {:?}", NodeInfo::from(child));
                    ptr = child;
                    found = true;
                    break;
                }
            }
            if !found {
                debug!(
                    "Couldn't find identifier with namespace path: {:?}",
                    Identifier::from_iter(namespaces)
                );
                return Err(JodinErrorType::IdentifierDoesNotExist(path.clone()).into());
            }
        }
        trace!("At node {} out of {:?}", ptr.id, path);
        let last_ptr = ptr;
        for value in last_ptr.related_values() {
            let full_id = value.get_identifier();
            if full_id == path {
                return Ok(value);
            }
        }
        debug!("{} is not an identifier.", path);
        Err(JodinErrorType::IdentifierDoesNotExist(path.clone()).into())
    }

    /// Attempts to get a mutable reference to the associated value from an absolute path.
    ///
    /// # Arguments
    ///
    /// * `path`: The absolute path
    ///
    /// returns: Result<&T, JodinError> the associated value, or an error
    pub fn mut_from_absolute_identifier(&mut self, path: &Identifier) -> JodinResult<&mut T> {
        let objects = if let Some(parent) = path.parent() {
            &mut self
                .get_node_mut(parent)
                .ok_or(JodinErrorType::IdentifierDoesNotExist(path.clone()))?
                .related_values
        } else {
            self.get_base_values_mut()
        };

        for object in objects {
            if object.get_identifier() == path {
                return Ok(object);
            }
        }
        Err(JodinErrorType::IdentifierDoesNotExist(path.clone()).into())
    }
}

impl<T: Namespaced> Tree for Node<T> {
    fn direct_children(&self) -> Vec<&Self> {
        self.children.iter().collect()
    }
}

#[derive(Clone)]
struct NodeInfo {
    id: Identifier,
    children: Vec<NodeInfo>,
    relevant: Vec<(Identifier, Option<Identifier>)>,
    is_namespace: bool,
}

impl<T: Namespaced> From<&Node<T>> for NodeInfo {
    fn from(n: &Node<T>) -> Self {
        NodeInfo {
            id: n.id.clone(),
            children: n.children.iter().map(|node| NodeInfo::from(node)).collect(),
            relevant: n
                .related_values
                .iter()
                .map(|r| {
                    let id = r.get_identifier();
                    let alias = if id.parent().unwrap() != &n.id {
                        Some(id.clone())
                    } else {
                        None
                    };
                    (id.this().into(), alias)
                })
                .collect(),
            is_namespace: true,
        }
    }
}

impl Debug for NodeInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut map = f.debug_map();
        // map.entry(&"id", &self.id);
        for child in &self.children {
            map.entry(&child.id, &child);
        }
        map.entry(
            &"relevant",
            &self.relevant.iter().map(|(a, _b)| a).collect::<Vec<_>>(),
        );
        map.finish()
    }
}

impl<T: Namespaced> Debug for NamespaceTree<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        NodeInfo::from(&self.head).fmt(f)
    }
}
/// Contains an identifier resolver and a mapping between full identifiers and it's associated value.
pub struct Registry<T> {
    resolver: IdentifierResolver,
    mapping: HashMap<Identifier, T>,
}

impl<T: Debug> Registry<T> {
    /// Creates a new, empty registry
    pub fn new() -> Self {
        Self {
            resolver: IdentifierResolver::new(),
            mapping: Default::default(),
        }
    }

    /// Creates a new registry that contains identifiers within the tree already.
    pub fn new_with_resolver(resolver: IdentifierResolver) -> Self {
        Self {
            resolver,
            mapping: Default::default(),
        }
    }

    /// Insert a new value into the registry. This identifier should be relative.
    pub fn insert(&mut self, val: T) -> JodinResult<Identifier>
    where
        T: Namespaced,
    {
        let identifier = val.get_identifier().clone();
        self.insert_with_identifier(val, identifier)
    }

    /// Inserts a value into the registry associated to an identifier.
    pub fn insert_with_identifier(&mut self, val: T, path: Identifier) -> JodinResult<Identifier> {
        let path = self.resolver.create_absolute_path(&path);
        if self.mapping.contains_key(&path) {
            return Err(JodinErrorType::IdentifierAlreadyExists(path).into());
        }
        self.mapping.insert(path.clone(), val);
        Ok(path)
    }

    /// Updates the value of an identifier, but only if it already exists within the registry.
    pub fn update_absolute_identity(&mut self, absolute: &Identifier, val: T) -> JodinResult<&T> {
        //let absolute = Identifier::new_concat(&self.resolver.base_namespace, absolute);
        if !self.resolver.contains_absolute_identifier(&absolute) {
            return Err(JodinErrorType::IdentifierDoesNotExist(absolute.clone()).into());
        }
        trace!("Setting visibility of {:?} to {:?}", absolute, val);
        self.mapping.insert(absolute.clone(), val);
        Ok(&self.mapping[&absolute])
    }

    /// Remove an identity from the registry
    pub fn remove_absolute_identity(&mut self, absolute: &Identifier) -> JodinResult<()> {
        if !self.resolver.contains_absolute_identifier(&absolute) {
            return Err(JodinErrorType::IdentifierDoesNotExist(absolute.clone()).into());
        }
        self.mapping.remove(absolute);
        Ok(())
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

    /// Attempts to get a value from a given path.
    pub fn get(&self, path: &Identifier) -> JodinResult<&T> {
        let full_path = self.resolver.resolve_path(path.clone(), false)?;
        self.mapping
            .get(&full_path)
            .ok_or(JodinErrorType::IdentifierDoesNotExist(path.clone()).into())
    }

    /// Attempts to get a mutable value from a given path.
    pub fn get_mut(&mut self, path: &Identifier) -> JodinResult<&mut T> {
        let full_path = self.resolver.resolve_path(path.clone(), false)?;
        self.mapping
            .get_mut(&full_path)
            .ok_or(JodinErrorType::IdentifierDoesNotExist(path.clone()).into())
    }
}

impl Registry<Visibility> {
    /// Checks if `check_path` is visible from `from_path`.
    ///
    /// # Example
    ///
    /// ```
    /// use std::iter::FromIterator;
    /// use jodin_common::identifier::Identifier;
    /// use jodin_common::core::identifier_resolution::Registry;
    /// use jodin_common::core::privacy::Visibility;
    ///
    ///
    /// let mut registry: Registry<Visibility> = Registry::new();
    /// registry.insert_with_identifier(Visibility::Public, Identifier::from_iter(["{base}", "namespace", "v1"]));
    /// registry.insert_with_identifier(Visibility::Protected, Identifier::from_iter(["{base}", "namespace", "v2"]));
    /// assert!(registry.is_visible(&Identifier::from_iter(["{base}", "namespace", "v1"]), &Identifier::from("{base}")));
    /// assert!(!registry.is_visible(&Identifier::from_iter(["{base}", "namespace", "v2"]), &Identifier::from("{base}")));
    /// ```
    pub fn is_visible(&self, check_path: &Identifier, from_namespace: &Identifier) -> bool {
        debug!(
            "Checking if {:?} is visible from {:?}",
            check_path, from_namespace
        );
        if !self.mapping.contains_key(check_path) {
            error!("Checked path ({:?}) not in visibility registry", check_path);
            error!("Registry: {:#?}", self);
            return false;
        } /*else if !self.mapping.contains_key(from_namespace) {
              error!(
                  "From-namespace ({:?}) not in visibility registry",
                  from_namespace
              );
              error!("Registry: {:#?}", self);
              return false;
          }
          */
        if let Ok(Visibility::Public) = self.get(check_path) {
            return true;
        }

        let diff = Identifier::diff(check_path, from_namespace);
        let mut is_first = true;
        for (check, _) in diff {
            if check.is_none() {
                break;
            }
            let check = check.unwrap();
            match self.get(&check) {
                Ok(Visibility::Public) => {}
                Ok(Visibility::Protected) => {
                    if !is_first {
                        return false;
                    }
                }
                Err(_) | Ok(Visibility::Private) => {
                    return false;
                }
            }
            is_first = false;
        }
        true
    }
}

impl<I: Into<Identifier>, T: Debug> Index<I> for Registry<T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.get(&index.into()).unwrap()
    }
}

impl<I: Into<Identifier>, T: Debug> IndexMut<I> for Registry<T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.get_mut(&index.into()).unwrap()
    }
}

impl<T: Debug> Debug for Registry<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("resolver", &self.resolver)
            .field("mapping", &self.mapping)
            .finish()
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
    use log::LevelFilter;
    use std::iter::FromIterator;

    use super::*;
    use crate::error::JodinErrorType;
    use crate::identifier::Identifiable;
    use crate::identifier::Identifier;
    use crate::init_logging;

    #[test]
    fn insert_entries() {
        let mut register = Registry::<i32>::new();
        register.push_namespace(Identifier::from("std"));
        register.insert_with_identifier(3, Identifier::from("best value"));
        let value = &register[Identifier::from_iter(&["std", "best value"])];
        assert_eq!(*value, 3);

        let mut registry = Registry::new();
        registry.insert(Identifiable::new("val1", 1)).unwrap();
        registry.insert(Identifiable::new("val2", 2)).unwrap();
        registry.insert(Identifiable::new("val3", 3)).unwrap();
    }

    #[test]
    fn is_visible() {
        init_logging(LevelFilter::Trace);
        let mut registry: Registry<Visibility> = Registry::new();
        registry.insert_with_identifier(
            Visibility::Public,
            Identifier::from_iter(["{base}", "namespace", "v1"]),
        );
        registry.insert_with_identifier(
            Visibility::Protected,
            Identifier::from_iter(["{base}", "namespace", "v2"]),
        );
        registry.insert_with_identifier(Visibility::Public, id!("{base}"));
        registry.insert_with_identifier(Visibility::Public, id!("{base}", "namespace"));
        assert!(registry.is_visible(&id!("{base}", "namespace", "v1"), &id!("{base}")));
        assert!(!registry.is_visible(
            &Identifier::from_iter(["{base}", "namespace", "v2"]),
            &Identifier::from("{base}")
        ));
    }

    #[test]
    fn id_resolution() {
        let mut resolver = IdentifierResolver::new();
        resolver.add_namespace(Identifier::from_iter("n1::n2::n3".split("::")));
        resolver.add_namespace(Identifier::from_iter("n1::n2::n4".split("::")));
        let path =
            resolver.create_absolute_path(&Identifier::from_iter("n1::n2::object".split("::")));
        println!("{:#?}", resolver);
        assert_eq!(path, "n1::n2::object");
        resolver.push_namespace(Identifier::from("n2"));
        println!("{:#?}", resolver);
        let path = resolver.create_absolute_path(&Identifier::from("object"));
        assert_eq!(path, "n2::object");
        println!("{:#?}", resolver);
        let result = resolver
            .resolve_path(Identifier::from_iter(&["n2", "object"]), false)
            .unwrap();
        assert_eq!(result, "n2::object");
        resolver.pop_namespace();
        resolver.push_namespace(Identifier::from("n1"));
        println!("{:#?}", resolver);
        let result = resolver.resolve_path(Identifier::from_iter(&["n2", "object"]), false);
        if let Err(JodinErrorType::AmbiguousIdentifierError { given: _, found }) =
            result.map_err(|err| err.into_err_and_bt().0)
        {
            assert!(
                (found
                    == vec![
                        Identifier::from_iter(&["n2", "object"]),
                        Identifier::from_iter(&["n1", "n2", "object"])
                    ])
                    || (found
                        == vec![
                            Identifier::from_iter(&["n1", "n2", "object"]),
                            Identifier::from_iter(&["n2", "object"]),
                        ])
            )
        } else {
            panic!("This should be ambiguous from this position, as both n1::n2::object (relative) and n2::object (absolute) exists");
        }
    }
}
