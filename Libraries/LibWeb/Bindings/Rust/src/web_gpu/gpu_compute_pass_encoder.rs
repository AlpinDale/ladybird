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
use crate::web_gpu::gpu_bind_group::GPUBindGroup;
use crate::web_gpu::gpu_compute_pipeline::GPUComputePipeline;

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUComputePassEncoder {
    #[gc(untraced)]
    commands: Rc<RefCell<Vec<u8>>>,
    realm: Gc<Realm>,
}

impl Wrappable for GPUComputePassEncoder {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUComputePassEncoder {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, commands: Rc<RefCell<Vec<u8>>>) -> Ref<Self> {
        allocate(vm, |_| Self { commands, realm })
    }

    pub(crate) fn set_pipeline(&self, _: &mut CallContext, pipeline: Ref<GPUComputePipeline>) -> WebIdlResult<()> {
        let mut c = self.commands.borrow_mut();
        c.push(2);
        c.extend_from_slice(&pipeline.pipeline_id().to_le_bytes());
        Ok(())
    }

    pub(crate) fn set_bind_group(&self, _: &mut CallContext, index: u32, bind_group: Option<Ref<GPUBindGroup>>) -> WebIdlResult<()> {
        let mut c = self.commands.borrow_mut();
        c.push(3);
        c.extend_from_slice(&index.to_le_bytes());
        c.extend_from_slice(&bind_group.map_or(0, |bg| bg.bind_group_id()).to_le_bytes());
        Ok(())
    }

    pub(crate) fn dispatch_workgroups(&self, _: &mut CallContext, x: u32, y: u32, z: u32) -> WebIdlResult<()> {
        let mut c = self.commands.borrow_mut();
        c.push(4);
        c.extend_from_slice(&x.to_le_bytes());
        c.extend_from_slice(&y.to_le_bytes());
        c.extend_from_slice(&z.to_le_bytes());
        Ok(())
    }

    pub(crate) fn end(&self, _: &mut CallContext) -> WebIdlResult<()> {
        self.commands.borrow_mut().push(5);
        Ok(())
    }
}
