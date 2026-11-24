/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use ak::Utf16String;
use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};

pub(crate) struct GPUShaderModuleDescriptor {
    pub(crate) code: Utf16String,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUShaderModule {
    #[gc(untraced)]
    module_id: u64,
    realm: Gc<Realm>,
}

impl Wrappable for GPUShaderModule {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUShaderModule {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, module_id: u64) -> Ref<Self> { allocate(vm, |_| Self { module_id, realm }) }
    pub(crate) fn module_id(&self) -> u64 { self.module_id }
}
