use jodin_common::identifier::Identifier;
use std::iter::FromIterator;

/// Creates dependency tags
pub struct DependencyTool {
    major_id: Vec<String>,
}

impl DependencyTool {
    fn major_namespace(&self) -> Identifier {
        Identifier::from_iter(&self.major_id)
    }

    fn id_within_major_namespace(&self, _id: &Identifier) -> bool {
        unimplemented!()
    }
}
