/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUBindGroupLayout {
    #[gc(untraced)]
    layout_id: u64,
    realm: Gc<Realm>,
}

impl Wrappable for GPUBindGroupLayout {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUBindGroupLayout {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, layout_id: u64) -> Ref<Self> { allocate(vm, |_| Self { layout_id, realm }) }
    pub(crate) fn layout_id(&self) -> u64 { self.layout_id }
}
