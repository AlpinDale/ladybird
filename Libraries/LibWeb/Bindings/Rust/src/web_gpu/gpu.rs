/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use core::ffi::c_void;

use libjs_rust::api::{Gc, Realm, Trace, Value, Vm};

use crate::bindings::environment_settings_object::{CppEnvironmentSettingsObject, relevant_settings_object};
use crate::bindings::interface::{Ref, Wrappable, allocate, cell_into_abi, realm_arguments, wrap};
use crate::bindings::promise::{WebIdlPromise, create_resolved_promise};
use crate::bindings::{CallContext, WebIdlResult};
use crate::web_gpu::gpu_adapter::GPUAdapter;
use crate::web_gpu::host::{WebGPUConnection, read_u64};

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPU {
    environment: CppEnvironmentSettingsObject,
    realm: Gc<Realm>,
}

impl Wrappable for GPU {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPU {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>) -> Ref<Self> {
        allocate(vm, |_| Self { environment: relevant_settings_object(realm.global_object()), realm })
    }

    pub(crate) fn request_adapter(&self, cx: &mut CallContext) -> WebIdlResult<WebIdlPromise> {
        let (vm, realm) = (cx.vm(), self.realm);
        let Some(conn) = WebGPUConnection::for_environment(&self.environment) else {
            return Ok(create_resolved_promise(vm, realm, Value::NULL));
        };
        let Ok(out) = conn.call(1, Vec::new(), None) else {
            return Ok(create_resolved_promise(vm, realm, Value::NULL));
        };
        let adapter_id = read_u64(&out, 0);
        if adapter_id == 0 || out.len() < 49 {
            return Ok(create_resolved_promise(vm, realm, Value::NULL));
        }
        let adapter = GPUAdapter::create(vm, realm, conn, adapter_id, out[8] != 0, &out[9..49]);
        Ok(create_resolved_promise(vm, realm, Value::from_object(wrap(&CallContext::new(vm, realm), adapter))))
    }
}

/// # Safety
///
/// The pointers must be the live VM and a live realm.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn web_rust_web_gpu_create_gpu(vm: *mut c_void, realm: *mut c_void) -> *mut c_void {
    // SAFETY: The caller passes the live VM and a live realm.
    let (vm, realm) = unsafe { realm_arguments(vm, realm) };
    cell_into_abi(GPU::create(vm, realm))
}
