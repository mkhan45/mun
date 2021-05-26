//! This module implements the logic to convert an AST to an `ItemTree`.

use super::{
    diagnostics, Field, Fields, Function, IdRange, ItemTree, ItemTreeData, ItemTreeNode,
    LocalItemTreeId, ModItem, RawVisibilityId, Struct, StructDefKind, TypeAlias, Const
};
use crate::{
    arena::{Idx, RawId},
    name::AsName,
    source_id::AstIdMap,
    type_ref::TypeRef,
    visibility::RawVisibility,
    DefDatabase, FileId, Name,
};
use mun_syntax::{
    ast,
    ast::{ExternOwner, ModuleItemOwner, NameOwner, StructKind, TypeAscriptionOwner},
};
use std::{collections::HashMap, convert::TryInto, marker::PhantomData, sync::Arc};

impl<N: ItemTreeNode> From<Idx<N>> for LocalItemTreeId<N> {
    fn from(index: Idx<N>) -> Self {
        LocalItemTreeId {
            index,
            _p: PhantomData,
        }
    }
}

pub(super) struct Context {
    file: FileId,
    source_ast_id_map: Arc<AstIdMap>,
    data: ItemTreeData,
    diagnostics: Vec<diagnostics::ItemTreeDiagnostic>,
}

impl Context {
    /// Constructs a new `Context` for the specified file
    pub(super) fn new(db: &dyn DefDatabase, file: FileId) -> Self {
        Self {
            file,
            source_ast_id_map: db.ast_id_map(file),
            data: ItemTreeData::default(),
            diagnostics: Vec::new(),
        }
    }

    /// Lowers all the items in the specified `ModuleItemOwner` and returns an `ItemTree`
    pub(super) fn lower_module_items(mut self, item_owner: &impl ModuleItemOwner) -> ItemTree {
        let top_level = item_owner
            .items()
            .flat_map(|item| self.lower_mod_item(&item))
            .collect::<Vec<_>>();

        // Check duplicates
        let mut set = HashMap::<Name, &ModItem>::new();
        for item in top_level.iter() {
            let name = match item {
                ModItem::Function(item) => &self.data.functions[item.index].name,
                ModItem::Struct(item) => &self.data.structs[item.index].name,
                ModItem::TypeAlias(item) => &self.data.type_aliases[item.index].name,
                ModItem::Const(item) => &self.data.constants[item.index].name,
            };
            if let Some(first_item) = set.get(&name) {
                self.diagnostics
                    .push(diagnostics::ItemTreeDiagnostic::DuplicateDefinition {
                        name: name.clone(),
                        first: **first_item,
                        second: *item,
                    })
            } else {
                set.insert(name.clone(), item);
            }
        }

        ItemTree {
            file_id: self.file,
            top_level,
            data: self.data,
            diagnostics: self.diagnostics,
        }
    }

    /// Lowers a single module item
    fn lower_mod_item(&mut self, item: &ast::ModuleItem) -> Option<ModItem> {
        match item.kind() {
            ast::ModuleItemKind::FunctionDef(ast) => self.lower_function(&ast).map(Into::into),
            ast::ModuleItemKind::StructDef(ast) => self.lower_struct(&ast).map(Into::into),
            ast::ModuleItemKind::TypeAliasDef(ast) => self.lower_type_alias(&ast).map(Into::into),
            ast::ModuleItemKind::ConstDef(ast) => self.lower_const_def(&ast).map(Into::into),
        }
    }

    /// Lowers a function
    fn lower_function(&mut self, func: &ast::FunctionDef) -> Option<LocalItemTreeId<Function>> {
        let name = func.name()?.as_name();
        let visibility = self.lower_visibility(func);

        // Lower all the params
        let mut params = Vec::new();
        if let Some(param_list) = func.param_list() {
            for param in param_list.params() {
                let type_ref = self.lower_type_ref_opt(param.ascribed_type());
                params.push(type_ref);
            }
        }

        // Lowers the return type
        let ret_type = func
            .ret_type()
            .and_then(|rt| rt.type_ref())
            .map_or_else(|| TypeRef::Empty, |ty| self.lower_type_ref(&ty));

        let is_extern = func.is_extern();

        let ast_id = self.source_ast_id_map.ast_id(func);
        let res = Function {
            name,
            visibility,
            is_extern,
            params: params.into_boxed_slice(),
            ret_type,
            ast_id,
        };

        Some(self.data.functions.alloc(res).into())
    }

    /// Lowers a struct
    fn lower_struct(&mut self, strukt: &ast::StructDef) -> Option<LocalItemTreeId<Struct>> {
        let name = strukt.name()?.as_name();
        let visibility = self.lower_visibility(strukt);
        let fields = self.lower_fields(&strukt.kind());
        let ast_id = self.source_ast_id_map.ast_id(strukt);
        let kind = match strukt.kind() {
            StructKind::Record(_) => StructDefKind::Record,
            StructKind::Tuple(_) => StructDefKind::Tuple,
            StructKind::Unit => StructDefKind::Unit,
        };
        let res = Struct {
            name,
            visibility,
            fields,
            ast_id,
            kind,
        };
        Some(self.data.structs.alloc(res).into())
    }

    /// Lowers the fields of a struct or enum
    fn lower_fields(&mut self, struct_kind: &ast::StructKind) -> Fields {
        match struct_kind {
            StructKind::Record(it) => {
                let range = self.lower_record_fields(it);
                Fields::Record(range)
            }
            StructKind::Tuple(it) => {
                let range = self.lower_tuple_fields(it);
                Fields::Tuple(range)
            }
            StructKind::Unit => Fields::Unit,
        }
    }

    /// Lowers records fields (e.g. `{ a: i32, b: i32 }`)
    fn lower_record_fields(&mut self, fields: &ast::RecordFieldDefList) -> IdRange<Field> {
        let start = self.next_field_idx();
        for field in fields.fields() {
            if let Some(data) = self.lower_record_field(&field) {
                let _idx = self.data.fields.alloc(data);
            }
        }
        let end = self.next_field_idx();
        IdRange::new(start..end)
    }

    /// Lowers a record field (e.g. `a:i32`)
    fn lower_record_field(&mut self, field: &ast::RecordFieldDef) -> Option<Field> {
        let name = field.name()?.as_name();
        let type_ref = self.lower_type_ref_opt(field.ascribed_type());
        let res = Field { name, type_ref };
        Some(res)
    }

    /// Lowers tuple fields (e.g. `(i32, u8)`)
    fn lower_tuple_fields(&mut self, fields: &ast::TupleFieldDefList) -> IdRange<Field> {
        let start = self.next_field_idx();
        for (i, field) in fields.fields().enumerate() {
            let data = self.lower_tuple_field(i, &field);
            let _idx = self.data.fields.alloc(data);
        }
        let end = self.next_field_idx();
        IdRange::new(start..end)
    }

    /// Lowers a tuple field (e.g. `i32`)
    fn lower_tuple_field(&mut self, idx: usize, field: &ast::TupleFieldDef) -> Field {
        let name = Name::new_tuple_field(idx);
        let type_ref = self.lower_type_ref_opt(field.type_ref());
        Field { name, type_ref }
    }

    /// Lowers a type alias (e.g. `type Foo = Bar`)
    fn lower_type_alias(
        &mut self,
        type_alias: &ast::TypeAliasDef,
    ) -> Option<LocalItemTreeId<TypeAlias>> {
        let name = type_alias.name()?.as_name();
        let visibility = self.lower_visibility(type_alias);
        let type_ref = type_alias.type_ref().map(|ty| self.lower_type_ref(&ty));
        let ast_id = self.source_ast_id_map.ast_id(type_alias);
        let res = TypeAlias {
            name,
            visibility,
            type_ref,
            ast_id,
        };
        Some(self.data.type_aliases.alloc(res).into())
    }

    /// Lowers a type alias (e.g. `type Foo = Bar`)
    fn lower_const_def(
        &mut self,
        const_def: &ast::ConstDef,
    ) -> Option<LocalItemTreeId<Const>> {
        let name = const_def.name()?.as_name();
        let visibility = self.lower_visibility(const_def);
        let type_ref = const_def.type_ref().map(|ty| self.lower_type_ref(&ty));
        let ast_id = self.source_ast_id_map.ast_id(const_def);
        let res = Const {
            name,
            visibility,
            type_ref,
            ast_id,
        };
        Some(self.data.constants.alloc(res).into())
    }

    /// Lowers an `ast::TypeRef`
    fn lower_type_ref(&self, type_ref: &ast::TypeRef) -> TypeRef {
        TypeRef::from_ast(type_ref.clone())
    }

    /// Lowers an optional `ast::TypeRef`
    fn lower_type_ref_opt(&self, type_ref: Option<ast::TypeRef>) -> TypeRef {
        type_ref
            .map(|ty| self.lower_type_ref(&ty))
            .unwrap_or(TypeRef::Error)
    }

    /// Lowers an `ast::VisibilityOwner`
    fn lower_visibility(&mut self, item: &impl ast::VisibilityOwner) -> RawVisibilityId {
        let vis = RawVisibility::from_ast(item.visibility());
        self.data.visibilities.alloc(vis)
    }

    /// Returns the `Idx` of the next `Field`
    fn next_field_idx(&self) -> Idx<Field> {
        let idx: u32 = self.data.fields.len().try_into().expect("too many fields");
        Idx::from_raw(RawId::from(idx))
    }
}
