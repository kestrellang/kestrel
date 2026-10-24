//! Existential function tables at mono (existentials plan Phase 10).
//!
//! Every `ImmediateKind::VTable { template, payload }` in a mono body is
//! interned once per `(template, payload)` into `MonoModule::vtables` — sub-
//! tables (ancestors, composition bounds) recursively — and the literal is
//! rewritten to `ImmediateKind::MonoVTable`. Function entries were
//! instantiated at the payload by collection (`CollectionContext::scan_vtable`).

use std::collections::HashMap;

use indexmap::IndexMap;
use kestrel_hecs::Entity;

use crate::immediate::ImmediateKind;
use crate::inst::InstKind;
use crate::item::{VTableEntry, VTableTemplate};
use crate::mono::{InstantiationKey, MonoVTable, MonoVTableEntry};
use crate::ty::TyArena;
use crate::{MonoFuncId, MonoVTableId, TyId};

pub(super) struct VTableInterner<'a> {
    pub templates: &'a IndexMap<Entity, VTableTemplate>,
    pub func_id_map: &'a HashMap<InstantiationKey, MonoFuncId>,
    pub arena: &'a TyArena,
    pub entity_names: &'a IndexMap<Entity, String>,
    pub tables: Vec<MonoVTable>,
    ids: HashMap<(Entity, TyId), MonoVTableId>,
}

impl<'a> VTableInterner<'a> {
    pub fn new(
        templates: &'a IndexMap<Entity, VTableTemplate>,
        func_id_map: &'a HashMap<InstantiationKey, MonoFuncId>,
        arena: &'a TyArena,
        entity_names: &'a IndexMap<Entity, String>,
    ) -> Self {
        Self {
            templates,
            func_id_map,
            arena,
            entity_names,
            tables: Vec::new(),
            ids: HashMap::new(),
        }
    }

    /// Rewrite every table literal in `body` to its interned id.
    pub fn rewrite_body(&mut self, body: &mut crate::body::OssaBody) {
        for block in &mut body.blocks {
            for inst in &mut block.insts {
                if let InstKind::Literal { value, .. } = &mut inst.kind
                    && let ImmediateKind::VTable { template, payload } = value.kind
                {
                    value.kind = ImmediateKind::MonoVTable(self.intern(template, payload));
                }
            }
        }
    }

    /// The table of `template` at `payload`, building it on first use.
    fn intern(&mut self, template: Entity, payload: TyId) -> MonoVTableId {
        if let Some(&id) = self.ids.get(&(template, payload)) {
            return id;
        }
        // Reserve the id first so a (pathological) self-reference terminates.
        let id = MonoVTableId::new(self.tables.len());
        self.ids.insert((template, payload), id);
        let tpl = &self.templates[&template];
        let name = format!(
            "{}${}",
            tpl.name,
            super::mangle::mangle_type_suffix(self.arena, self.entity_names, payload)
        );
        self.tables.push(MonoVTable {
            name,
            entries: Vec::new(),
        });
        let entries = tpl.entries.clone();
        let resolved = entries
            .into_iter()
            .map(|entry| match entry {
                VTableEntry::Func(func) => {
                    let key = InstantiationKey::new(func, vec![payload], None);
                    match self.func_id_map.get(&key) {
                        Some(&f) => MonoVTableEntry::Func(f),
                        None => MonoVTableEntry::Null,
                    }
                },
                VTableEntry::Size => MonoVTableEntry::Size(payload),
                VTableEntry::Align => MonoVTableEntry::Align(payload),
                VTableEntry::Table(sub) => MonoVTableEntry::Table(self.intern(sub, payload)),
                VTableEntry::Null => MonoVTableEntry::Null,
            })
            .collect();
        kestrel_debug::ktrace!("vtable", "table #{} {}", id.index(), self.tables[id.index()].name);
        self.tables[id.index()].entries = resolved;
        id
    }
}
