use crate::primitive_type::PrimitiveType;
use crate::{ids::ItemDefinitionId, visibility::Visibility, Name, PerNs};
use once_cell::sync::Lazy;
use rustc_hash::FxHashMap;

/// Holds all items that are visible from an item as well as by which name and under which
/// visibility they are accessible.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct ItemScope {
    /// All types visible in this scope
    types: FxHashMap<Name, (ItemDefinitionId, Visibility)>,

    /// All values visible in this scope
    values: FxHashMap<Name, (ItemDefinitionId, Visibility)>,

    /// All items that are defined in this scope
    defs: Vec<ItemDefinitionId>,
}

pub(crate) static BUILTIN_SCOPE: Lazy<FxHashMap<Name, PerNs<(ItemDefinitionId, Visibility)>>> =
    Lazy::new(|| {
        PrimitiveType::ALL
            .iter()
            .map(|(name, ty)| {
                (
                    name.clone(),
                    PerNs::types((ty.clone().into(), Visibility::Public)),
                )
            })
            .collect()
    });

impl ItemScope {
    /// Returns an iterator over all declarations with this scope
    pub fn declarations(&self) -> impl Iterator<Item = ItemDefinitionId> + '_ {
        self.defs.iter().copied()
    }

    /// Adds an item definition to the list of definitions
    pub(crate) fn add_definition(&mut self, def: ItemDefinitionId) {
        self.defs.push(def)
    }

    /// Adds a named item resolution into the scope
    pub(crate) fn add_resolution(
        &mut self,
        name: Name,
        def: PerNs<(ItemDefinitionId, Visibility)>,
    ) {
        if let Some((types, visibility)) = def.types {
            self.types.insert(name.clone(), (types, visibility));
        }
        if let Some((values, visibility)) = def.values {
            self.values.insert(name, (values, visibility));
        }
    }

    /// Gets a name from the current module scope
    pub(crate) fn get(&self, name: &Name) -> PerNs<(ItemDefinitionId, Visibility)> {
        PerNs {
            types: self.types.get(name).copied(),
            values: self.values.get(name).copied(),
        }
    }
}

impl PerNs<(ItemDefinitionId, Visibility)> {
    pub(crate) fn from_definition(
        def: ItemDefinitionId,
        vis: Visibility,
        has_constructor: bool,
    ) -> PerNs<(ItemDefinitionId, Visibility)> {
        match def {
            ItemDefinitionId::FunctionId(_) => PerNs::values((def, vis)),
            ItemDefinitionId::StructId(_) => {
                if has_constructor {
                    PerNs::both((def, vis), (def, vis))
                } else {
                    PerNs::types((def, vis))
                }
            }
            ItemDefinitionId::TypeAliasId(_) => PerNs::types((def, vis)),
            ItemDefinitionId::ConstDefId(_) => PerNs::types((def, vis)),
            ItemDefinitionId::PrimitiveType(_) => PerNs::types((def, vis)),
            ItemDefinitionId::ModuleId(_) => PerNs::types((def, vis)),
        }
    }
}
