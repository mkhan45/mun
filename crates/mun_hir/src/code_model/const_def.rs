use std::sync::Arc;

use crate::{
    ids::Lookup,
    ids::ConstDefId,
    type_ref::{LocalTypeRefId, TypeRefBuilder, TypeRefMap, TypeRefSourceMap},
    visibility::RawVisibility,
    DefDatabase, DiagnosticSink, FileId, HasVisibility, HirDatabase, Name, Visibility,
};

use super::Module;
use crate::expr::validator::ConstDefValidator;
use crate::resolve::HasResolver;
use crate::ty::lower::LowerBatchResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstDef {
    pub(crate) id: ConstDefId,
}

impl From<ConstDefId> for ConstDef {
    fn from(id: ConstDefId) -> Self {
        ConstDef { id }
    }
}

impl ConstDef {
    pub fn module(self, db: &dyn HirDatabase) -> Module {
        Module {
            id: self.id.lookup(db.upcast()).module,
        }
    }

    pub fn file_id(self, db: &dyn HirDatabase) -> FileId {
        self.id.lookup(db.upcast()).id.file_id
    }

    pub fn data(self, db: &dyn DefDatabase) -> Arc<ConstDefData> {
        db.type_alias_data(self.id)
    }

    pub fn name(self, db: &dyn HirDatabase) -> Name {
        self.data(db.upcast()).name.clone()
    }

    pub fn type_ref(self, db: &dyn HirDatabase) -> LocalTypeRefId {
        self.data(db.upcast()).type_ref_id
    }

    pub fn lower(self, db: &dyn HirDatabase) -> Arc<LowerBatchResult> {
        db.lower_type_alias(self)
    }

    pub fn diagnostics(self, db: &dyn HirDatabase, sink: &mut DiagnosticSink) {
        let data = self.data(db.upcast());
        let lower = self.lower(db);
        lower.add_diagnostics(db, self.file_id(db), data.type_ref_source_map(), sink);

        let validator = ConstDefValidator::new(self, db);
        validator.validate_target_type_existence(sink);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConstDefData {
    pub name: Name,
    pub visibility: RawVisibility,
    pub type_ref_id: LocalTypeRefId,
    type_ref_map: TypeRefMap,
    type_ref_source_map: TypeRefSourceMap,
}
impl ConstDefData {
    pub(crate) fn type_alias_data_query(
        db: &dyn DefDatabase,
        id: ConstDefId,
    ) -> Arc<ConstDefData> {
        let loc = id.lookup(db);
        let item_tree = db.item_tree(loc.id.file_id);
        let alias = &item_tree[loc.id.value];
        let src = item_tree.source(db, loc.id.value);
        let mut type_ref_builder = TypeRefBuilder::default();
        let type_ref_opt = src.type_ref();
        let type_ref_id = type_ref_builder.alloc_from_node_opt(type_ref_opt.as_ref());
        let (type_ref_map, type_ref_source_map) = type_ref_builder.finish();
        Arc::new(ConstDefData {
            name: alias.name.clone(),
            visibility: item_tree[alias.visibility].clone(),
            type_ref_id,
            type_ref_map,
            type_ref_source_map,
        })
    }

    pub fn type_ref_source_map(&self) -> &TypeRefSourceMap {
        &self.type_ref_source_map
    }

    pub fn type_ref_map(&self) -> &TypeRefMap {
        &self.type_ref_map
    }
}

impl HasVisibility for ConstDef {
    fn visibility(&self, db: &dyn HirDatabase) -> Visibility {
        self.data(db.upcast())
            .visibility
            .resolve(db.upcast(), &self.id.resolver(db.upcast()))
    }
}
