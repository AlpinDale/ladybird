/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUTextureView {
    #[gc(untraced)]
    view_id: u64,
    realm: Gc<Realm>,
}

impl Wrappable for GPUTextureView {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUTextureView {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, view_id: u64) -> Ref<Self> { allocate(vm, |_| Self { view_id, realm }) }
    pub(crate) fn view_id(&self) -> u64 { self.view_id }
}
