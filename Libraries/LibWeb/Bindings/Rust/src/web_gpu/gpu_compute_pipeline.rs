/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::rc::Rc;

use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};
use crate::bindings::{CallContext, WebIdlResult};
use crate::web_gpu::gpu_bind_group_layout::GPUBindGroupLayout;
use crate::web_gpu::gpu_shader_module::GPUShaderModule;
use crate::web_gpu::host::WebGPUConnection;

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUProgrammableStage {
    pub(crate) module: Ref<GPUShaderModule>,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUComputePipelineDescriptor {
    pub(crate) compute: GPUProgrammableStage,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUComputePipeline {
    #[gc(untraced)]
    conn: Rc<WebGPUConnection>,
    #[gc(untraced)]
    pipeline_id: u64,
    realm: Gc<Realm>,
}

impl Wrappable for GPUComputePipeline {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUComputePipeline {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, conn: Rc<WebGPUConnection>, pipeline_id: u64) -> Ref<Self> {
        allocate(vm, |_| Self { conn, pipeline_id, realm })
    }

    pub(crate) fn pipeline_id(&self) -> u64 { self.pipeline_id }

    pub(crate) fn get_bind_group_layout(&self, cx: &mut CallContext, index: u32) -> WebIdlResult<Ref<GPUBindGroupLayout>> {
        let mut p = Vec::with_capacity(12);
        p.extend_from_slice(&self.pipeline_id.to_le_bytes());
        p.extend_from_slice(&index.to_le_bytes());
        let id = self.conn.call_id(8, p)?;
        Ok(GPUBindGroupLayout::create(cx.vm(), self.realm, id))
    }
}
