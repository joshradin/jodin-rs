use crate::core::error::{JodinError, JodinErrorType, JodinResult};
use crate::core::identifier::Identifier;
use crate::core::identifier_resolution::{IdentifierResolver, Registry};

use crate::ast::JodinNode;
use crate::ast::JodinNodeInner;

use crate::ast::tags::Tag;
use crate::core::import::{Import, ImportType};
use crate::core::privacy::{Visibility, VisibilityTag};
use std::any::Any;
use std::cmp::Ordering;

/// A toolchain that assigns identities to every node that needs to be resolved. For example, the
/// types must all be resolved.
pub struct IdentityResolutionTool {
    creator: IdentifierCreator,
    setter: IdentifierSetter,
    visibility: Registry<Visibility>,
}

impl IdentityResolutionTool {
    /// Creates a new id resolution tool.
    pub fn new() -> Self {
        Self {
            creator: IdentifierCreator::new(),
            setter: IdentifierSetter::new(),
            visibility: Registry::new(),
        }
    }

    /// Resolve identifiers
    pub fn resolve_identities(
        &mut self,
        input: JodinNode,
    ) -> JodinResult<(JodinNode, IdentifierResolver)> {
        let (mut tree, mut resolver) = self.creator.start(input, &mut self.visibility)?;
        let base = resolver.base_namespace();
        self.visibility
            .insert_with_identifier(Visibility::Public, base.clone())?;
        self.setter
            .set_identities(&mut tree, &mut resolver, &self.visibility)
            .map(|_| (tree, resolver))
    }
}

/// This tag adds a resolved [Identifier](crate::core::identifier::Identifier) to a node. This resolved
/// identifier is absolute.
#[derive(Debug, Clone)]
pub struct ResolvedIdentityTag(Identifier);

impl ResolvedIdentityTag {
    /// The absolute identifier of the tag.
    pub fn absolute_id(&self) -> &Identifier {
        &self.0
    }

    /// Creates a new tag from an identifier-like value.
    pub fn new<I: Into<Identifier>>(id: I) -> Self {
        ResolvedIdentityTag(id.into())
    }
}

impl Tag for ResolvedIdentityTag {
    fn tag_type(&self) -> String {
        String::from("ResolvedId")
    }

    fn tag_info(&self) -> String {
        format!("[Resolved {}]", self.absolute_id())
    }

    fn max_of_this_tag(&self) -> u32 {
        1
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A tag that assigns an identifier to an individual block.
#[derive(Debug)]
pub struct BlockIdentifierTag(usize);

impl BlockIdentifierTag {
    /// Creates a new block identifier
    ///
    /// # Arguments
    ///
    /// * `val`: The value to use as the base for the identifier
    ///
    /// returns: BlockIdentifierTag
    pub fn new(val: usize) -> Self {
        Self(val)
    }

    /// Gets the block number of the tag
    pub fn block_num(&self) -> usize {
        self.0
    }
}

#[derive(Debug)]
pub struct IdentifierCreator {
    block_num: Vec<usize>,
}

impl Tag for BlockIdentifierTag {
    fn tag_type(&self) -> String {
        "BlockNum".to_string()
    }

    fn max_of_this_tag(&self) -> u32 {
        1
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl IdentifierCreator {
    fn new() -> Self {
        Self { block_num: vec![0] }
    }

    fn get_block_num(&mut self) -> usize {
        let block_num = self.block_num.last_mut().unwrap();
        let ret = *block_num;
        *block_num += 1;
        ret
    }

    fn create_identities(
        &mut self,
        tree: &mut JodinNode,
        id_resolver: &mut IdentifierResolver,
        visibility_registry: &mut Registry<Visibility>,
    ) -> JodinResult<()> {
        match tree.inner_mut() {
            // This one only occurs when requested
            JodinNodeInner::Identifier(id) => {
                match id_resolver
                    .resolve_path(id.clone(), false)
                    .map_err(|e| e.error_type)
                {
                    Ok(path) => {
                        if visibility_registry.get(&path).unwrap() > &Visibility::Private {
                            return Err(JodinErrorType::IdentifierAlreadyExists(id.clone()).into());
                        }
                    }
                    Err(JodinErrorType::AmbiguousIdentifierError { given: _, found }) => {
                        for found in found {
                            if visibility_registry.get(&found).unwrap() > &Visibility::Private {
                                return Err(
                                    JodinErrorType::IdentifierAlreadyExists(id.clone()).into()
                                );
                            }
                        }
                    }
                    _ => {}
                }

                let abs = id_resolver.create_absolute_path_no_strip(id);
                visibility_registry.insert_with_identifier(Visibility::Protected, abs.clone())?;
                if let Ok(tag) = tree.get_tag::<VisibilityTag>() {
                    let vis = tag.visibility().clone();
                    visibility_registry.update_absolute_identity(&abs, vis)?;
                }
                let tag = ResolvedIdentityTag(abs);
                tree.add_tag(tag)?;
            }
            JodinNodeInner::VarDeclarations {
                var_type: _, names, ..
            } => {
                for name in names {
                    self.create_identities(name, id_resolver, visibility_registry)?;
                }
            }
            JodinNodeInner::FunctionDefinition {
                name,
                return_type: _,
                arguments,
                generic_parameters,
                block,
            } => {
                self.create_identities(name, id_resolver, visibility_registry)?;
                let tag = name.get_tag::<ResolvedIdentityTag>()?.clone();
                let name = Identifier::from(tag.absolute_id().this());
                id_resolver.push_namespace(name);

                for argument in arguments {
                    self.create_identities(argument, id_resolver, visibility_registry)?;
                }

                for generic in generic_parameters {
                    self.create_identities(generic, id_resolver, visibility_registry)?;
                }

                self.create_identities(block, id_resolver, visibility_registry)?;

                id_resolver.pop_namespace();
            }
            JodinNodeInner::Block { expressions } => {
                self.start_block(id_resolver);

                let mut blocks = vec![];

                for expression in expressions {
                    if let JodinNodeInner::VarDeclarations { .. } = expression.inner() {
                        self.create_identities(expression, id_resolver, visibility_registry)?;
                    } else {
                        blocks.push(expression);
                    }
                }

                // Allows for forwards and backwards scope in blocks
                for block in blocks {
                    self.create_identities(block, id_resolver, visibility_registry)?;
                }

                self.end_block(id_resolver);
            }
            JodinNodeInner::StructureDefinition { name, members } => {
                self.create_identities(name, id_resolver, visibility_registry)?;

                let tag = name.get_tag::<ResolvedIdentityTag>()?.clone();
                // tags_to_add.push(Box::new(tag.clone()));
                let name = Identifier::from(tag.absolute_id().this());
                id_resolver.push_namespace(name);

                for member in members {
                    self.create_identities(member, id_resolver, visibility_registry)?;
                }

                id_resolver.pop_namespace();
            }
            JodinNodeInner::NamedValue { name, .. } => {
                self.create_identities(name, id_resolver, visibility_registry)?
            }
            JodinNodeInner::InNamespace { namespace, inner } => {
                self.create_identities(namespace, id_resolver, visibility_registry)?;
                let tag = namespace.get_tag::<ResolvedIdentityTag>()?.clone();
                let name = Identifier::from(tag.absolute_id().this());
                id_resolver.push_namespace(name);
                self.create_identities(inner, id_resolver, visibility_registry)?;
                id_resolver.pop_namespace();
            }
            JodinNodeInner::ImportIdentifiers {
                import_data: _,
                affected,
            } => {
                self.create_identities(affected, id_resolver, visibility_registry)?;
            }
            JodinNodeInner::TopLevelDeclarations { decs } => {
                for child in decs {
                    self.create_identities(child, id_resolver, visibility_registry)?;
                }
            }
            JodinNodeInner::WhileStatement { cond: _, statement } => {
                self.start_block(id_resolver);
                self.create_identities(statement, id_resolver, visibility_registry)?;
                self.end_block(id_resolver);
            }
            JodinNodeInner::IfStatement {
                cond: _,
                statement,
                else_statement,
            } => {
                self.start_block(id_resolver);
                self.create_identities(statement, id_resolver, visibility_registry)?;
                self.end_block(id_resolver);

                if let Some(statement) = else_statement {
                    self.start_block(id_resolver);
                    self.create_identities(statement, id_resolver, visibility_registry)?;
                    self.end_block(id_resolver);
                }
            }
            JodinNodeInner::SwitchStatement {
                to_switch: _,
                labeled_statements,
            } => {
                self.start_block(id_resolver);
                for statement in labeled_statements {
                    self.create_identities(statement, id_resolver, visibility_registry)?;
                }
                self.end_block(id_resolver);
            }
            JodinNodeInner::ForStatement {
                init,
                cond: _,
                delta: _,
                statement,
            } => {
                self.start_block(id_resolver);

                if let Some(init) = init {
                    self.create_identities(init, id_resolver, visibility_registry)?;
                }
                self.create_identities(statement, id_resolver, visibility_registry)?;

                self.end_block(id_resolver);
            }
            other => {
                println!("Unsupported: {:?}", other);
            }
        }
        Ok(())
    }

    fn start_block(&mut self, id_resolver: &mut IdentifierResolver) {
        let block_num = self.get_block_num();
        let string = Identifier::from(format!("{{block {}}}", block_num));
        let last = id_resolver.current_namespace_with_base();
        self.block_num.push(0);

        id_resolver.push_namespace(string.clone());
        //id_resolver.create_absolute_path(&Identifier::from(""));
        id_resolver.use_namespace(last).unwrap();
        println!("{:#?}", id_resolver);
    }

    fn end_block(&mut self, id_resolver: &mut IdentifierResolver) {
        id_resolver.pop_namespace();
        self.block_num.pop();
        let current = id_resolver.current_namespace_with_base();
        id_resolver.stop_use_namespace(&current).unwrap();
    }

    fn start(
        &mut self,
        mut input: JodinNode,
        registry: &mut Registry<Visibility>,
    ) -> JodinResult<(JodinNode, IdentifierResolver)> {
        let mut resolver = IdentifierResolver::new();
        self.create_identities(&mut input, &mut resolver, registry)?;
        println!("{:#?}", registry);
        Ok((input, resolver))
    }
}

fn find_first_tag<T: 'static + Tag>(node: &JodinNode) -> Option<&T> {
    if let Ok(ret) = node.get_tag() {
        return Some(ret);
    } else {
        for child in node {
            if let Some(ret) = find_first_tag(child) {
                return Some(ret);
            }
        }
        None
    }
}

pub struct IdentifierSetter {
    aliases: Registry<Identifier>,
}

impl IdentifierSetter {
    fn new() -> Self {
        Self {
            aliases: Registry::new(),
        }
    }

    fn set_identities(
        &mut self,
        tree: &mut JodinNode,
        id_resolver: &mut IdentifierResolver,
        visibility_resolver: &Registry<Visibility>,
    ) -> JodinResult<()> {
        let has_id = tree.get_tag::<ResolvedIdentityTag>().is_ok();

        match tree.inner_mut() {
            JodinNodeInner::InNamespace { namespace, inner } => {
                let namespace = namespace
                    .get_tag::<ResolvedIdentityTag>()
                    .unwrap()
                    .absolute_id()
                    .this_as_id();
                id_resolver.push_namespace(namespace.clone());
                self.aliases.push_namespace(namespace);
                self.set_identities(inner, id_resolver, visibility_resolver)?;
                id_resolver.pop_namespace();
                self.aliases.pop_namespace();
            }
            JodinNodeInner::ImportIdentifiers {
                import_data,
                affected,
            } => {
                let imports =
                    self.add_import_data(import_data, id_resolver, visibility_resolver)?;
                println!("Imports: {:#?}", self.aliases);
                self.set_identities(affected, id_resolver, visibility_resolver)?;
                for import in imports {
                    self.aliases.remove_absolute_identity(&import)?;
                }
            }
            JodinNodeInner::Identifier(id) => {
                if !has_id {
                    println!(
                        "Attempting to find {} from {}",
                        id,
                        id_resolver.current_namespace_with_base()
                    );
                    let resolved =
                        self.try_get_absolute_identifier(id, id_resolver, visibility_resolver)?;
                    println!("Found {}", resolved);
                    let resolved_tag = ResolvedIdentityTag::new(resolved);

                    tree.add_tag(resolved_tag)?;
                }
            }
            JodinNodeInner::FunctionDefinition {
                name,
                return_type: _,
                arguments,
                generic_parameters,
                block,
            } => {
                self.create_identities(name, id_resolver, visibility_registry)?;
                let tag = name.get_tag::<ResolvedIdentityTag>()?.clone();
                let name = Identifier::from(tag.absolute_id().this());
                id_resolver.push_namespace(name);

                for argument in arguments {
                    self.create_identities(argument, id_resolver, visibility_registry)?;
                }

                for generic in generic_parameters {
                    self.create_identities(generic, id_resolver, visibility_registry)?;
                }

                self.create_identities(block, id_resolver, visibility_registry)?;

                id_resolver.pop_namespace();
            }
            JodinNodeInner::Block { expressions } => {
                self.start_block(id_resolver);

                let mut blocks = vec![];

                for expression in expressions {
                    if let JodinNodeInner::VarDeclarations { .. } = expression.inner() {
                        self.create_identities(expression, id_resolver, visibility_registry)?;
                    } else {
                        blocks.push(expression);
                    }
                }

                // Allows for forwards and backwards scope in blocks
                for block in blocks {
                    self.create_identities(block, id_resolver, visibility_registry)?;
                }

                self.end_block(id_resolver);
            }
            JodinNodeInner::StructureDefinition { name, members } => {
                self.create_identities(name, id_resolver, visibility_registry)?;

                let tag = name.get_tag::<ResolvedIdentityTag>()?.clone();
                // tags_to_add.push(Box::new(tag.clone()));
                let name = Identifier::from(tag.absolute_id().this());
                id_resolver.push_namespace(name);

                for member in members {
                    self.create_identities(member, id_resolver, visibility_registry)?;
                }

                id_resolver.pop_namespace();
            }
            nNodeInner::WhileStatement { cond: _, statement } => {
                self.start_block(id_resolver);
                self.create_identities(statement, id_resolver, visibility_registry)?;
                self.end_block(id_resolver);
            }
            JodinNodeInner::IfStatement {
                cond: _,
                statement,
                else_statement,
            } => {
                self.start_block(id_resolver);
                self.create_identities(statement, id_resolver, visibility_registry)?;
                self.end_block(id_resolver);

                if let Some(statement) = else_statement {
                    self.start_block(id_resolver);
                    self.create_identities(statement, id_resolver, visibility_registry)?;
                    self.end_block(id_resolver);
                }
            }
            JodinNodeInner::SwitchStatement {
                to_switch: _,
                labeled_statements,
            } => {
                self.start_block(id_resolver);
                for statement in labeled_statements {
                    self.create_identities(statement, id_resolver, visibility_registry)?;
                }
                self.end_block(id_resolver);
            }
            JodinNodeInner::ForStatement {
                init,
                cond: _,
                delta: _,
                statement,
            } => {
                self.start_block(id_resolver);

                if let Some(init) = init {
                    self.create_identities(init, id_resolver, visibility_registry)?;
                }
                self.create_identities(statement, id_resolver, visibility_registry)?;

                self.end_block(id_resolver);
            }
            other => {
                for child in other.children_mut() {
                    self.set_identities(child, id_resolver, visibility_resolver)?;
                }
            }
        }
        Ok(())
    }

    fn try_get_absolute_identifier(
        &self,
        id: &Identifier,
        id_resolver: &IdentifierResolver,
        visibility: &Registry<Visibility>,
    ) -> JodinResult<Identifier> {
        // first get alias if it exist
        let alias =
            self.aliases
                .get(id)
                .ok()
                .filter(|&alias_id| {
                    let visibility = visibility.get(alias_id).ok();
                    match visibility {
                        None => true,
                        Some(visibility) => visibility
                            .is_visible(alias_id, &id_resolver.current_namespace_with_base()),
                    }
                })
                .cloned();
        let as_normal = id_resolver
            .resolve_path(id.clone(), false)
            .ok()
            .filter(|resolved| {
                let visibility = visibility.get(resolved).ok();
                match visibility {
                    None => true,
                    Some(visibility) => {
                        visibility.is_visible(resolved, &id_resolver.current_namespace_with_base())
                    }
                }
            });

        match (alias, as_normal) {
            (Some(alias), None) => Ok(alias),
            (None, Some(as_normal)) => Ok(as_normal),
            (Some(a), Some(n)) => Err(JodinErrorType::AmbiguousIdentifierError {
                given: id.clone(),
                found: vec![a, n],
            }
            .into()),
            (None, None) => Err(JodinErrorType::IdentifierDoesNotExist(id.clone()).into()),
        }
    }

    /// Add imports from an import data, returning a list of created identifiers
    fn add_import_data(
        &mut self,
        import: &Import,
        id_resolver: &IdentifierResolver,
        visibility: &Registry<Visibility>,
    ) -> JodinResult<Vec<Identifier>> {
        println!("import base = {}", import.id());
        let mut aliases = vec![];
        let resolved = &id_resolver.resolve_path(import.id().clone(), true)?;
        let current = id_resolver.current_namespace_with_base();
        if !identifier_is_visible_from(&current, resolved, visibility)? {
            return Err(JodinErrorType::IdentifierProtected {
                target: import.id().clone(),
                origin_namespace: current.strip_highest_parent().unwrap(),
            }
            .into());
        }

        match import.import_type() {
            ImportType::Direct => {
                self.aliases
                    .insert_with_identifier(resolved.clone(), &current + &resolved.this_as_id())?;
                aliases.push(current + resolved.this_as_id());
            }
            ImportType::Aliased { alias } => {
                self.aliases
                    .insert_with_identifier(resolved.clone(), &current + alias)?;
                aliases.push(&current + alias);
            }
            ImportType::Wildcard => {
                let tree = id_resolver.namespace_tree();
                let path = resolved.clone();
                let relevant = tree.get_relevant_objects(&path).ok_or(JodinError::from(
                    JodinErrorType::IdentifierDoesNotExist(path),
                ))?;
                for relevant_object in relevant {
                    let target = relevant_object.clone();
                    println!(
                        "Checking if {} is visible from {} for wildcard",
                        target, current
                    );
                    if identifier_is_visible_from(&current, &target, visibility)? {
                        /*
                        return Err(JodinErrorType::IdentifierProtected {
                            target: import.id().clone(),
                            origin_namespace: current.strip_highest_parent().unwrap(),
                        }
                        .into());



                         */

                        let alias = relevant_object.this_as_id();
                        println!("Found in wildcard: {}", alias);
                        self.aliases
                            .insert_with_identifier(target.clone(), &current + &alias)?;
                        aliases.push(&current + &alias);
                    }
                }
            }
            ImportType::Children { children } => {
                for child in children {
                    let true_child_import = child
                        .concat_parent_to_id(&resolved.clone().strip_highest_parent().unwrap());
                    let imports =
                        self.add_import_data(&true_child_import, id_resolver, visibility)?;
                    aliases.extend(imports);
                }
            }
        }
        println!("Imported {:?}", aliases);
        Ok(aliases)
    }
}

/// Check whether an identifier can be see from the original namespace. Both origin and destination
/// should be absolute paths.
pub fn identifier_is_visible_from(
    origin_namespace: &Identifier,
    target: &Identifier,
    visibility: &Registry<Visibility>,
) -> JodinResult<bool> {
    println!(
        "Checking if {} is visible from {}",
        target, origin_namespace
    );
    if target.iter().count() == 0 {
        return Err(JodinErrorType::IdentifierDoesNotExist(target.clone()).into());
    }
    let mut target_iter = target.iter();
    let mut target = Identifier::from(target_iter.next().unwrap());

    loop {
        let target_visibility = visibility.get(&target)?;
        println!("Visibility of target {} is {:?}", target, target_visibility);

        match target_visibility {
            Visibility::Public => {
                if let Some(next) = target_iter.next() {
                    target = target + Identifier::from(next);
                } else {
                    break;
                }
            }
            Visibility::Protected => {
                let target_parent = target.parent().unwrap();
                let comparison = target_parent.partial_cmp(origin_namespace);
                println!(
                    "Comparison of {} and {} = {:?}",
                    target_parent, origin_namespace, comparison
                );

                match comparison {
                    Some(Ordering::Greater) => {
                        if let Some(next) = target_iter.next() {
                            target = target + Identifier::from(next);
                        } else {
                            break;
                        }
                    }
                    _ => return Ok(false),
                }
            }
            Visibility::Private => return Ok(false),
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use crate::core::error::JodinResult;

    #[test]
    fn label_structure_members() -> JodinResult<()> {
        Ok(())
    }
}
