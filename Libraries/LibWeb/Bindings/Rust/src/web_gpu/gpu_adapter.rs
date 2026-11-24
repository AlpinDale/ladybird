/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::rc::Rc;

use ak::Utf16String;
use libjs_rust::api::{Gc, Object, PrimitiveString, Realm, Set, Trace, Value, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate, wrap};
use crate::bindings::promise::{WebIdlPromise, create_rejected_promise_from_exception, create_resolved_promise};
use crate::bindings::{CallContext, WebIdlResult};
use crate::web_gpu::gpu_device::GPUDevice;
use crate::web_gpu::host::{LIMIT_NAMES, WebGPUConnection, limits_object, op_err, read_u64};

pub(crate) struct GPUDeviceDescriptor {
    pub(crate) required_limits: Option<Vec<(Utf16String, Option<u64>)>>,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUAdapter {
    #[gc(untraced)]
    conn: Rc<WebGPUConnection>,
    #[gc(untraced)]
    adapter_id: u64,
    features: Gc<Object>,
    limits: Gc<Object>,
    info: Gc<Object>,
    realm: Gc<Realm>,
}

impl Wrappable for GPUAdapter {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUAdapter {
    pub(crate) fn create(
        vm: &Vm,
        realm: Gc<Realm>,
        conn: Rc<WebGPUConnection>,
        adapter_id: u64,
        has_f16: bool,
        limits_bytes: &[u8],
    ) -> Ref<Self> {
        let set = Set::create(vm, realm);
        if has_f16 {
            set.set_add(vm, Value::from_string(PrimitiveString::create(vm, Utf16String::from_utf8("shader-f16"))));
        }
        let limits = limits_object(vm, realm, limits_bytes);
        let info = Object::create(vm, realm, Some(realm.intrinsics().object_prototype(vm)));
        allocate(vm, |_| Self { conn, adapter_id, features: set.upcast::<Object>(), limits, info, realm })
    }

    pub(crate) fn features(&self, _: &mut CallContext) -> WebIdlResult<Gc<Object>> { Ok(self.features) }
    pub(crate) fn limits(&self, _: &mut CallContext) -> WebIdlResult<Gc<Object>> { Ok(self.limits) }
    pub(crate) fn info(&self, _: &mut CallContext) -> WebIdlResult<Gc<Object>> { Ok(self.info) }

    pub(crate) fn request_device(&self, cx: &mut CallContext, descriptor: GPUDeviceDescriptor) -> WebIdlResult<WebIdlPromise> {
        let (vm, realm) = (cx.vm(), self.realm);
        let mut req = [0u64; 5];
        if let Some(map) = descriptor.required_limits {
            for (k, v) in map {
                if let Some(v) = v && let Some(i) = LIMIT_NAMES.iter().position(|&n| n == crate::web_gpu::utf16_to_string(&k)) {
                    req[i] = v;
                }
            }
        }
        let mut p = Vec::with_capacity(48);
        p.extend_from_slice(&self.adapter_id.to_le_bytes());
        for v in req { p.extend_from_slice(&v.to_le_bytes()); }
        match self.conn.call(2, p, None) {
            Ok(out) if out.len() >= 56 => {
                let device = GPUDevice::create(vm, realm, Rc::clone(&self.conn), read_u64(&out, 0), read_u64(&out, 8), &out[16..56]);
                Ok(create_resolved_promise(vm, realm, Value::from_object(wrap(&CallContext::new(vm, realm), device))))
            }
            Ok(_) => Ok(create_rejected_promise_from_exception(vm, realm, op_err("truncated device reply".into()))),
            Err(e) => Ok(create_rejected_promise_from_exception(vm, realm, op_err(e))),
        }
    }
}
