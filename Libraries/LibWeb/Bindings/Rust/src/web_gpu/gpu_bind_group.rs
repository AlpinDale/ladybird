/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use libjs_rust::api::{Gc, MarkedVec, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};
use crate::web_gpu::gpu_bind_group_layout::GPUBindGroupLayout;
use crate::web_gpu::gpu_buffer::GPUBuffer;
use crate::web_gpu::gpu_texture_view::GPUTextureView;

#[derive(Clone, Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUBufferBinding {
    pub(crate) buffer: Ref<GPUBuffer>,
    #[gc(untraced)]
    pub(crate) offset: u64,
}

#[derive(Clone, Trace)]
#[gc(crate = libjs_rust)]
pub(crate) enum GPUBindingResource {
    GPUBufferBinding(GPUBufferBinding),
    GPUTextureView(Ref<GPUTextureView>),
}

#[derive(Clone, Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUBindGroupEntry {
    #[gc(untraced)]
    pub(crate) binding: u32,
    pub(crate) resource: GPUBindingResource,
}

pub(crate) struct GPUBindGroupDescriptor<'vm> {
    pub(crate) layout: Ref<GPUBindGroupLayout>,
    pub(crate) entries: MarkedVec<'vm, GPUBindGroupEntry>,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUBindGroup {
    #[gc(untraced)]
    bind_group_id: u64,
    realm: Gc<Realm>,
}

impl Wrappable for GPUBindGroup {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUBindGroup {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, bind_group_id: u64) -> Ref<Self> { allocate(vm, |_| Self { bind_group_id, realm }) }
    pub(crate) fn bind_group_id(&self) -> u64 { self.bind_group_id }
}
