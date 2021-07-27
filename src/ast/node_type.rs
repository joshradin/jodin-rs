//! The differentiation of jodin nodes in the AST that allow for more complex information to be
//! stored within the AST.

use crate::core::identifier::Identifier;
use crate::core::import::Import;
use crate::core::literal::Literal;
use crate::core::operator::Operator;

use crate::ast::intermediate_type::IntermediateType;
use crate::ast::jodin_node::JodinNode;
use crate::parsing::JodinRule;

/// Contains JodinNode variant information.
#[derive(Debug)]
pub enum JodinNodeInner {
    /// Store an intermediate type.
    Type(IntermediateType),
    /// Store a literal.
    Literal(Literal),
    /// Store an identifier.
    Identifier(Identifier),
    /// Store variable declarations, roughly translates to `<type> (<name> (= <value>)?)*`.
    VarDeclarations {
        /// The type for the variable.
        var_type: JodinNode,
        /// The ids being declared.
        names: Vec<JodinNode>,
        /// The maybe values the variables are being initialized with.
        values: Vec<Option<JodinNode>>,
    },
    /// Stores a function definition, such as `int fibonacci(int n) { ... }`.
    FunctionDefinition {
        /// The name of the function.
        name: JodinNode,
        /// The return type.
        return_type: IntermediateType,
        /// The arguments of the function.
        arguments: Vec<JodinNode>,
        /// The generic parameters of the function.
        generic_parameters: Vec<JodinNode>,
        /// The associated block of code.
        block: JodinNode,
    },
    /// Stores a block
    Block {
        /// The statements that make up the block.
        expressions: Vec<JodinNode>,
    },
    /// Contains a structure definition, such as `struct s1 { int i; }`.
    StructureDefinition {
        /// The id of the struct.
        name: JodinNode,
        /// The members of the struct.
        members: Vec<JodinNode>,
    },
    /// Represents a named value, usually used as a parameter or a member of a structure.
    NamedValue {
        /// The id
        name: JodinNode,
        /// The type
        var_type: IntermediateType,
    },
    /// An operator that takes in only one argument, such as `-1`.
    Uniop {
        /// The operator
        op: Operator,
        /// The argument
        inner: JodinNode,
    },
    /// An expression to cast a value to another type.
    CastExpression {
        /// The expression to be casted
        to_type: IntermediateType,
        /// The destination type
        factor: JodinNode,
    },
    /// An operator that takes in only one argument that occurs after the value, such as `x++`.
    Postop {
        /// The operator.
        op: Operator,
        /// The argument.
        inner: JodinNode,
    },
    /// An operator that takes in two arguments, such as `1+2`.
    Binop {
        /// The operator.
        op: Operator,
        /// The left had side argument.
        lhs: JodinNode,
        /// The right had side argument.
        rhs: JodinNode,
    },
    /// Represents a ternary expression, which is a shorthand for conditional expressions. Expressed
    /// as `<cond> ? <if_true> : <if_false>`.
    Ternary {
        /// The condition.
        cond: JodinNode,
        /// The value if the condition is true.
        yes: JodinNode,
        /// The value if the condition is false.
        no: JodinNode,
    },
    /// The index operator, used mainly when getting a member of an array.
    Index {
        /// The value being indexed.
        indexed: JodinNode,
        /// The expression that is the index.
        expression: JodinNode,
    },
    /// The call operator, used to call functions or methods.
    Call {
        /// The expression being called.
        called: JodinNode,
        /// The generic types to use in the call.
        generics_instance: Vec<JodinNode>,
        /// The arguments to pass in the call.
        arguments: Vec<JodinNode>,
    },
    /// Get a member of a compound type.
    GetMember {
        /// The instance of a compound type.
        compound: JodinNode,
        /// The id of the member.
        id: JodinNode,
    },
    /// A list of top level declarations
    TopLevelDeclarations {
        /// The declarations.
        decs: Vec<JodinNode>,
    },
    /// The `in <namespace>` expression.
    InNamespace {
        /// The namespace.
        namespace: JodinNode,
        /// The part of the AST that is within this namespace.
        inner: JodinNode,
    },
    /// Contains import data.
    ImportIdentifiers {
        /// The import data.
        import_data: Import,
    },
    /// Unimplemented nodes represent parts of the parse tree that can't be converted into AST (yet).
    Unimplemented {
        /// The rule that wasn't converted.
        jodin_rule: JodinRule,
        /// The string from the original code that wasn't converted.
        affected_string: String,
    },
}

impl JodinNodeInner {
    /// Convert this value into an instance of Result.
    pub fn into_result<E>(self) -> Result<JodinNode, E> {
        Ok(self.into())
    }

    /// The child JodinNodes of this variant.
    pub fn children(&self) -> impl IntoIterator<Item = &JodinNode> {
        let vector: Vec<&JodinNode> = match self {
            JodinNodeInner::Type(_) => {
                vec![]
            }
            JodinNodeInner::Literal(_) => {
                vec![]
            }
            JodinNodeInner::Identifier(_) => {
                vec![]
            }
            JodinNodeInner::VarDeclarations {
                var_type,
                names,
                values,
            } => {
                let mut ret = vec![var_type];
                ret.extend(names);
                ret.extend(values.iter().filter_map(|node| node.as_ref()));
                ret
            }
            JodinNodeInner::FunctionDefinition {
                name,
                return_type: _,
                arguments: parameters,
                generic_parameters,
                block,
            } => {
                let mut ret = vec![name];
                ret.extend(parameters);
                ret.extend(generic_parameters);
                ret.push(block);
                ret
            }
            JodinNodeInner::Block { expressions } => expressions.into_iter().collect(),
            JodinNodeInner::StructureDefinition { name, members } => {
                let mut ret = vec![name];
                ret.extend(members);
                ret
            }
            JodinNodeInner::NamedValue { name, var_type: _ } => {
                vec![name]
            }
            JodinNodeInner::Uniop { op: _, inner } => {
                vec![inner]
            }
            JodinNodeInner::CastExpression { to_type: _, factor } => {
                vec![factor]
            }
            JodinNodeInner::Postop { op: _, inner } => {
                vec![inner]
            }
            JodinNodeInner::Binop { op: _, lhs, rhs } => {
                vec![lhs, rhs]
            }
            JodinNodeInner::Ternary { cond, yes, no } => {
                vec![cond, yes, no]
            }
            JodinNodeInner::Index {
                indexed,
                expression,
            } => {
                vec![indexed, expression]
            }
            JodinNodeInner::Call {
                called,
                generics_instance,
                arguments: parameters,
            } => {
                let mut ret = vec![called];
                ret.extend(generics_instance);
                ret.extend(parameters);
                ret
            }
            JodinNodeInner::GetMember { compound, id } => {
                vec![compound, id]
            }
            JodinNodeInner::TopLevelDeclarations { decs } => decs.iter().collect(),
            JodinNodeInner::InNamespace { namespace, inner } => {
                vec![namespace, inner]
            }
            JodinNodeInner::ImportIdentifiers { .. } => {
                vec![]
            }
            JodinNodeInner::Unimplemented { .. } => {
                vec![]
            }
        };
        vector
    }

    /// The mutable child JodinNodes of this variant.
    pub fn children_mut(&mut self) -> impl IntoIterator<Item = &mut JodinNode> {
        let vector: Vec<&mut JodinNode> = match self {
            JodinNodeInner::Type(_) => {
                vec![]
            }
            JodinNodeInner::Literal(_) => {
                vec![]
            }
            JodinNodeInner::Identifier(_) => {
                vec![]
            }
            JodinNodeInner::VarDeclarations {
                var_type,
                names,
                values,
            } => {
                let mut ret = vec![var_type];
                ret.extend(names);
                ret.extend(values.iter_mut().filter_map(|node| node.as_mut()));
                ret
            }
            JodinNodeInner::FunctionDefinition {
                name,
                return_type: _,
                arguments: parameters,
                generic_parameters,
                block,
            } => {
                let mut ret = vec![name];
                ret.extend(parameters);
                ret.extend(generic_parameters);
                ret.push(block);
                ret
            }
            JodinNodeInner::Block { expressions } => expressions.into_iter().collect(),
            JodinNodeInner::StructureDefinition { name, members } => {
                let mut ret = vec![name];
                ret.extend(members);
                ret
            }
            JodinNodeInner::NamedValue { name, var_type: _ } => {
                vec![name]
            }
            JodinNodeInner::Uniop { op: _, inner } => {
                vec![inner]
            }
            JodinNodeInner::CastExpression { to_type: _, factor } => {
                vec![factor]
            }
            JodinNodeInner::Postop { op: _, inner } => {
                vec![inner]
            }
            JodinNodeInner::Binop { op: _, lhs, rhs } => {
                vec![lhs, rhs]
            }
            JodinNodeInner::Ternary { cond, yes, no } => {
                vec![cond, yes, no]
            }
            JodinNodeInner::Index {
                indexed,
                expression,
            } => {
                vec![indexed, expression]
            }
            JodinNodeInner::Call {
                called,
                generics_instance,
                arguments: parameters,
            } => {
                let mut ret = vec![called];
                ret.extend(generics_instance);
                ret.extend(parameters);
                ret
            }
            JodinNodeInner::GetMember { compound, id } => {
                vec![compound, id]
            }
            JodinNodeInner::TopLevelDeclarations { decs } => decs.iter_mut().collect(),
            JodinNodeInner::InNamespace { namespace, inner } => {
                vec![namespace, inner]
            }
            JodinNodeInner::ImportIdentifiers { .. } => {
                vec![]
            }
            JodinNodeInner::Unimplemented { .. } => {
                vec![]
            }
        };
        vector
    }
}

impl From<JodinNodeInner> for JodinNode {
    fn from(i: JodinNodeInner) -> Self {
        JodinNode::new(i)
    }
}