/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use core::cell::Cell;
use std::rc::Rc;

use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};
use crate::bindings::{CallContext, WebIdlResult};
use crate::web_gpu::gpu_texture_view::GPUTextureView;
use crate::web_gpu::host::WebGPUConnection;

pub(crate) struct GPUTextureDescriptor {
    pub(crate) size: Vec<u32>,
    pub(crate) usage: u32,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUTexture {
    #[gc(untraced)]
    conn: Rc<WebGPUConnection>,
    #[gc(untraced)]
    texture_id: u64,
    #[gc(untraced)]
    destroyed: Cell<bool>,
    realm: Gc<Realm>,
}

impl Wrappable for GPUTexture {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl Drop for GPUTexture {
    fn drop(&mut self) { if !self.destroyed.get() { self.conn.destroy(3, self.texture_id); } }
}

impl GPUTexture {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, conn: Rc<WebGPUConnection>, texture_id: u64) -> Ref<Self> {
        allocate(vm, |_| Self { conn, texture_id, destroyed: Cell::new(false), realm })
    }

    pub(crate) fn texture_id(&self) -> u64 { self.texture_id }

    pub(crate) fn create_view(&self, cx: &mut CallContext) -> WebIdlResult<Ref<GPUTextureView>> {
        let id = self.conn.call_id(5, self.texture_id.to_le_bytes().to_vec())?;
        Ok(GPUTextureView::create(cx.vm(), self.realm, id))
    }

    pub(crate) fn destroy(&self, _: &mut CallContext) -> WebIdlResult<()> {
        if !self.destroyed.replace(true) { self.conn.destroy(3, self.texture_id); }
        Ok(())
    }
}
