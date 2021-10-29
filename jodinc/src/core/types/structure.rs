//! The most basic, complex type that is just a record

use crate::core::identifier::Identifier;
use crate::core::privacy::Visibility;

use crate::core::types::{get_type_id, CompoundType, JodinType, JodinTypeReference, Type, Field};
use crate::core::types::intermediate_type::IntermediateType;

/// Contains a name and its fields
#[derive(Debug)]
pub struct Structure {
    name: Identifier,
    type_id: u32,
    fields: Vec<Field>,
}

impl Structure {
    /// Creates a new named structure
    pub fn new(name: String, fields: Vec<(String, IntermediateType)>) -> Self {
        Structure {
            name: Identifier::from(name),
            type_id: get_type_id(),
            fields: fields.into_iter().map(|(name, ty)| Field {
                vis: Visibility::Public,
                jtype: ty,
                name: Identifier::from(name)
            })
                .collect(),
        }
    }

    /// Creates an anonymous structure
    pub fn anonymous_struct(fields: Vec<(String, IntermediateType)>) -> Self {
        let type_id = get_type_id();
        let name: Identifier = format!("<anonymous struct {}>", type_id).into();
        Structure {
            name: name,
            type_id,
            fields: fields.into_iter().map(|(name, ty)| Field {
                vis: Visibility::Public,
                jtype: ty,
                name: Identifier::from(name)
            })
                .collect(),
        }
    }

    /// Gets the fields of the structure
    pub fn fields(&self) -> &Vec<Field> {
        &self.fields
    }
}

impl Type for Structure {
    fn type_name(&self) -> Identifier {
        self.name.clone()
    }

    fn type_id(&self) -> u32 {
        self.type_id
    }
}

impl CompoundType for Structure {
    fn all_members(&self) -> Vec<(&Visibility, &IntermediateType, &Identifier)> {
        self.fields
            .iter()
            .map(|field| field.as_tuple())
            .collect()
    }
}

impl From<Structure> for JodinType {
    fn from(s: Structure) -> Self {
        JodinType::Structure(s)
    }
}
