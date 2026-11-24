/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use core::ffi::c_void;
use core::ptr::NonNull;
use std::rc::Rc;

use ak::Utf16String;
use libcompositing_rust::ipc_types::Core_AnonymousBuffer;
use libcompositing_rust::ipc_types::compositor_web_content_server::messages::{WebgpuCall, WebgpuPost};
use libjs_rust::api::{Attribute, Gc, Object, PropertyAttributes, PropertyKey, Realm, Value, Vm};

use crate::bindings::Exception;
use crate::bindings::environment_settings_object::CppEnvironmentSettingsObject;
use crate::compositor_connection::{CompositorConnection, SharedBuffer};

unsafe extern "C" {
    fn web_gpu_compositor_connection(environment: *mut c_void) -> *mut c_void;
}

pub(crate) const LIMIT_NAMES: [&str; 5] = [
    "maxStorageBufferBindingSize",
    "maxBufferSize",
    "maxComputeWorkgroupStorageSize",
    "maxStorageBuffersPerShaderStage",
    "maxTextureDimension2D",
];

pub(crate) struct WebGPUConnection(CompositorConnection);

impl WebGPUConnection {
    pub(crate) fn for_environment(env: &CppEnvironmentSettingsObject) -> Option<Rc<Self>> {
        // SAFETY: `env` is live; C++ returns a newly ref'd `CompositorConnection*` or null.
        let ptr = NonNull::new(unsafe { web_gpu_compositor_connection(env.as_ptr()) })?;
        // SAFETY: Released in `Drop`.
        Some(Rc::new(Self(unsafe { CompositorConnection::lent(ptr) })))
    }

    pub(crate) fn call(&self, op: u8, payload: Vec<u8>, shm: Option<&SharedBuffer>) -> Result<Vec<u8>, String> {
        let buffer = shm.map_or(Core_AnonymousBuffer { shared_memory: None }, SharedBuffer::for_message);
        let reply = self.0.send_sync(WebgpuCall { op, payload, buffer }).ok_or("Compositor connection lost")?;
        if !reply.ok {
            return Err(String::from_utf8_lossy(&reply.payload).into_owned());
        }
        Ok(reply.payload)
    }

    pub(crate) fn call_id(&self, op: u8, payload: Vec<u8>) -> Result<u64, Exception> {
        let out = self.call(op, payload, None).map_err(op_err)?;
        Ok(read_u64(&out, 0))
    }

    pub(crate) fn post(&self, op: u8, payload: Vec<u8>, shm: Option<&SharedBuffer>) {
        let buffer = shm.map_or(Core_AnonymousBuffer { shared_memory: None }, SharedBuffer::for_message);
        self.0.post(WebgpuPost { op, payload, buffer });
    }

    pub(crate) fn destroy(&self, kind: u8, id: u64) {
        if id != 0 {
            let mut p = Vec::with_capacity(9);
            p.push(kind);
            p.extend_from_slice(&id.to_le_bytes());
            self.post(1, p, None);
        }
    }
}

impl Drop for WebGPUConnection {
    fn drop(&mut self) {
        // SAFETY: Matches the `ref()` in `web_gpu_compositor_connection`.
        unsafe { self.0.release() };
    }
}

pub(crate) fn op_err(msg: String) -> Exception {
    Exception::Dom("OperationError", Utf16String::from_utf8(&msg))
}

pub(crate) fn read_u64(b: &[u8], off: usize) -> u64 {
    b.get(off..off + 8).map_or(0, |s| u64::from_le_bytes(s.try_into().unwrap()))
}

pub(crate) fn limits_object(vm: &Vm, realm: Gc<Realm>, bytes: &[u8]) -> Gc<Object> {
    let obj = Object::create(vm, realm, Some(realm.intrinsics().object_prototype(vm)));
    let attrs = PropertyAttributes::new(Attribute::ENUMERABLE);
    for (i, name) in LIMIT_NAMES.iter().enumerate() {
        obj.define_direct_property(vm, &PropertyKey::from_utf8(name), Value::from_f64(read_u64(bytes, i * 8) as f64), attrs);
    }
    obj
}
