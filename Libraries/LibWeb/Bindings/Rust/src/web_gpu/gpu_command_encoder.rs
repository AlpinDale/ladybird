/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use core::cell::RefCell;
use std::rc::Rc;

use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};
use crate::bindings::{CallContext, WebIdlResult};
use crate::web_gpu::gpu_buffer::GPUBuffer;
use crate::web_gpu::gpu_command_buffer::GPUCommandBuffer;
use crate::web_gpu::gpu_compute_pass_encoder::GPUComputePassEncoder;

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUCommandEncoder {
    #[gc(untraced)]
    commands: Rc<RefCell<Vec<u8>>>,
    realm: Gc<Realm>,
}

impl Wrappable for GPUCommandEncoder {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUCommandEncoder {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>) -> Ref<Self> {
        allocate(vm, |_| Self { commands: Rc::new(RefCell::new(Vec::new())), realm })
    }

    pub(crate) fn begin_compute_pass(&self, cx: &mut CallContext) -> WebIdlResult<Ref<GPUComputePassEncoder>> {
        self.commands.borrow_mut().push(1);
        Ok(GPUComputePassEncoder::create(cx.vm(), self.realm, Rc::clone(&self.commands)))
    }

    pub(crate) fn copy_buffer_to_buffer(
        &self,
        _: &mut CallContext,
        source: Ref<GPUBuffer>,
        source_offset: u64,
        destination: Ref<GPUBuffer>,
        destination_offset: u64,
        size: u64,
    ) -> WebIdlResult<()> {
        let mut c = self.commands.borrow_mut();
        c.push(6);
        for v in [source.buffer_id(), source_offset, destination.buffer_id(), destination_offset, size] {
            c.extend_from_slice(&v.to_le_bytes());
        }
        Ok(())
    }

    pub(crate) fn finish(&self, cx: &mut CallContext) -> WebIdlResult<Ref<GPUCommandBuffer>> {
        Ok(GPUCommandBuffer::create(cx.vm(), self.realm, core::mem::take(&mut *self.commands.borrow_mut())))
    }
}
