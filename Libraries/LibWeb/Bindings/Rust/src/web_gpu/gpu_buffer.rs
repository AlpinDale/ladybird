/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use core::cell::Cell;
use std::rc::Rc;

use libjs_rust::api::{ArrayBuffer, ArrayBufferShared, Gc, GcRefCell, Realm, Trace, Value, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};
use crate::bindings::promise::{WebIdlPromise, create_rejected_promise_from_exception, create_resolved_promise};
use crate::bindings::{CallContext, WebIdlResult};
use crate::compositor_connection::SharedBuffer;
use crate::web_gpu::host::{WebGPUConnection, op_err};

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUBuffer {
    #[gc(untraced)]
    conn: Rc<WebGPUConnection>,
    #[gc(untraced)]
    device_id: u64,
    #[gc(untraced)]
    buffer_id: u64,
    #[gc(untraced)]
    size: u64,
    #[gc(untraced)]
    destroyed: Cell<bool>,
    mapped: GcRefCell<Option<Gc<ArrayBuffer>>>,
    realm: Gc<Realm>,
}

impl Wrappable for GPUBuffer {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl Drop for GPUBuffer {
    fn drop(&mut self) { if !self.destroyed.get() { self.conn.destroy(2, self.buffer_id); } }
}

impl GPUBuffer {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, conn: Rc<WebGPUConnection>, device_id: u64, buffer_id: u64, size: u64) -> Ref<Self> {
        allocate(vm, |_| Self { conn, device_id, buffer_id, size, destroyed: Cell::new(false), mapped: GcRefCell::new(None), realm })
    }

    pub(crate) fn buffer_id(&self) -> u64 { self.buffer_id }

    pub(crate) fn map_async(&self, cx: &mut CallContext) -> WebIdlResult<WebIdlPromise> {
        let (vm, realm) = (cx.vm(), self.realm);
        let len = usize::try_from(self.size).unwrap_or(0);
        let Some(shm) = SharedBuffer::create(len) else {
            return Ok(create_rejected_promise_from_exception(vm, realm, op_err("shm alloc failed".into())));
        };
        let mut p = Vec::with_capacity(24);
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(&self.buffer_id.to_le_bytes());
        p.extend_from_slice(&self.size.to_le_bytes());
        if let Err(e) = self.conn.call(10, p, Some(&shm)) {
            return Ok(create_rejected_promise_from_exception(vm, realm, op_err(e)));
        }
        // SAFETY: `shm` is mapped for `len` bytes written by the compositor.
        let slice = unsafe { core::slice::from_raw_parts(shm.as_ptr(), len) };
        *self.mapped.borrow_mut(vm) = Some(ArrayBuffer::create_from_bytes(vm, realm, slice, ArrayBufferShared::No));
        Ok(create_resolved_promise(vm, realm, Value::UNDEFINED))
    }

    pub(crate) fn get_mapped_range(&self, _: &mut CallContext) -> WebIdlResult<Gc<ArrayBuffer>> {
        self.mapped.borrow().ok_or_else(|| op_err("GPUBuffer is not mapped".into()))
    }

    pub(crate) fn unmap(&self, cx: &mut CallContext) -> WebIdlResult<()> {
        *self.mapped.borrow_mut(cx.vm()) = None;
        Ok(())
    }

    pub(crate) fn destroy(&self, cx: &mut CallContext) -> WebIdlResult<()> {
        *self.mapped.borrow_mut(cx.vm()) = None;
        if !self.destroyed.replace(true) { self.conn.destroy(2, self.buffer_id); }
        Ok(())
    }
}
