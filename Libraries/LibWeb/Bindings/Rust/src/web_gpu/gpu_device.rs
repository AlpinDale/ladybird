/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::rc::Rc;

use libjs_rust::api::{Gc, Object, Realm, Trace, Value, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate, wrap};
use crate::bindings::promise::{WebIdlPromise, create_promise, create_rejected_promise_from_exception, create_resolved_promise};
use crate::bindings::{CallContext, WebIdlResult};
use crate::web_gpu::gpu_bind_group::{GPUBindGroup, GPUBindGroupDescriptor, GPUBindingResource};
use crate::web_gpu::gpu_buffer::GPUBuffer;
use crate::web_gpu::gpu_command_encoder::GPUCommandEncoder;
use crate::web_gpu::gpu_compute_pipeline::{GPUComputePipeline, GPUComputePipelineDescriptor};
use crate::web_gpu::gpu_queue::GPUQueue;
use crate::web_gpu::gpu_shader_module::{GPUShaderModule, GPUShaderModuleDescriptor};
use crate::web_gpu::gpu_texture::{GPUTexture, GPUTextureDescriptor};
use crate::web_gpu::host::{WebGPUConnection, limits_object};

pub(crate) struct GPUBufferDescriptor {
    pub(crate) size: u64,
    pub(crate) usage: u32,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUDevice {
    #[gc(untraced)]
    conn: Rc<WebGPUConnection>,
    #[gc(untraced)]
    device_id: u64,
    limits: Gc<Object>,
    queue: Ref<GPUQueue>,
    lost: WebIdlPromise,
    realm: Gc<Realm>,
}

impl Wrappable for GPUDevice {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl Drop for GPUDevice {
    fn drop(&mut self) { self.conn.destroy(1, self.device_id); }
}

impl GPUDevice {
    pub(crate) fn create(
        vm: &Vm,
        realm: Gc<Realm>,
        conn: Rc<WebGPUConnection>,
        device_id: u64,
        queue_id: u64,
        limits_bytes: &[u8],
    ) -> Ref<Self> {
        let limits = limits_object(vm, realm, limits_bytes);
        let queue = GPUQueue::create(vm, realm, Rc::clone(&conn), device_id, queue_id);
        let lost = create_promise(vm, realm);
        allocate(vm, |_| Self { conn, device_id, limits, queue, lost, realm })
    }

    pub(crate) fn limits(&self, _: &mut CallContext) -> WebIdlResult<Gc<Object>> { Ok(self.limits) }
    pub(crate) fn queue(&self, _: &mut CallContext) -> WebIdlResult<Ref<GPUQueue>> { Ok(self.queue) }
    pub(crate) fn lost(&self, _: &mut CallContext) -> WebIdlResult<WebIdlPromise> { Ok(self.lost) }

    pub(crate) fn create_buffer(&self, cx: &mut CallContext, desc: GPUBufferDescriptor) -> WebIdlResult<Ref<GPUBuffer>> {
        let mut p = Vec::with_capacity(20);
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(&desc.size.to_le_bytes());
        p.extend_from_slice(&desc.usage.to_le_bytes());
        let id = self.conn.call_id(3, p)?;
        Ok(GPUBuffer::create(cx.vm(), self.realm, Rc::clone(&self.conn), self.device_id, id, desc.size))
    }

    pub(crate) fn create_texture(&self, cx: &mut CallContext, desc: GPUTextureDescriptor) -> WebIdlResult<Ref<GPUTexture>> {
        let (w, h) = (desc.size.first().copied().unwrap_or(1), desc.size.get(1).copied().unwrap_or(1));
        let mut p = Vec::with_capacity(20);
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(&w.to_le_bytes());
        p.extend_from_slice(&h.to_le_bytes());
        p.extend_from_slice(&desc.usage.to_le_bytes());
        let id = self.conn.call_id(4, p)?;
        Ok(GPUTexture::create(cx.vm(), self.realm, Rc::clone(&self.conn), id))
    }

    pub(crate) fn create_shader_module(&self, cx: &mut CallContext, desc: GPUShaderModuleDescriptor) -> WebIdlResult<Ref<GPUShaderModule>> {
        let code = crate::web_gpu::utf16_to_string(&desc.code);
        let mut p = Vec::with_capacity(8 + code.len());
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(code.as_bytes());
        let id = self.conn.call_id(6, p)?;
        Ok(GPUShaderModule::create(cx.vm(), self.realm, id))
    }

    pub(crate) fn create_compute_pipeline(&self, cx: &mut CallContext, desc: GPUComputePipelineDescriptor) -> WebIdlResult<Ref<GPUComputePipeline>> {
        let mut p = Vec::with_capacity(16);
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(&desc.compute.module.module_id().to_le_bytes());
        let id = self.conn.call_id(7, p)?;
        Ok(GPUComputePipeline::create(cx.vm(), self.realm, Rc::clone(&self.conn), id))
    }

    pub(crate) fn create_compute_pipeline_async(&self, cx: &mut CallContext, desc: GPUComputePipelineDescriptor) -> WebIdlResult<WebIdlPromise> {
        let (vm, realm) = (cx.vm(), self.realm);
        match self.create_compute_pipeline(cx, desc) {
            Ok(pipe) => Ok(create_resolved_promise(vm, realm, Value::from_object(wrap(&CallContext::new(vm, realm), pipe)))),
            Err(e) => Ok(create_rejected_promise_from_exception(vm, realm, e)),
        }
    }

    pub(crate) fn create_bind_group(&self, cx: &mut CallContext, desc: GPUBindGroupDescriptor<'_>) -> WebIdlResult<Ref<GPUBindGroup>> {
        let entries = desc.entries.values();
        let mut p = Vec::with_capacity(20 + entries.len() * 21);
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(&desc.layout.layout_id().to_le_bytes());
        p.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for e in entries.iter() {
            match &e.resource {
                GPUBindingResource::GPUBufferBinding(b) => {
                    p.push(1);
                    p.extend_from_slice(&e.binding.to_le_bytes());
                    p.extend_from_slice(&b.buffer.buffer_id().to_le_bytes());
                    p.extend_from_slice(&b.offset.to_le_bytes());
                }
                GPUBindingResource::GPUTextureView(v) => {
                    p.push(2);
                    p.extend_from_slice(&e.binding.to_le_bytes());
                    p.extend_from_slice(&v.view_id().to_le_bytes());
                }
            }
        }
        let id = self.conn.call_id(9, p)?;
        Ok(GPUBindGroup::create(cx.vm(), self.realm, id))
    }

    pub(crate) fn create_command_encoder(&self, cx: &mut CallContext) -> WebIdlResult<Ref<GPUCommandEncoder>> {
        Ok(GPUCommandEncoder::create(cx.vm(), self.realm))
    }
}
