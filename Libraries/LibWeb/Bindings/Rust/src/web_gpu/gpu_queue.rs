/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use std::rc::Rc;

use libjs_rust::api::{Gc, MarkedVec, Realm, Trace, Value, Vm};

use crate::bindings::conversions::BufferSource;
use crate::bindings::interface::{Ref, Wrappable, allocate};
use crate::bindings::promise::{WebIdlPromise, create_resolved_promise};
use crate::bindings::{CallContext, WebIdlResult};
use crate::compositor_connection::SharedBuffer;
use crate::html::image_bitmap::ImageBitmap;
use crate::web_gpu::gpu_buffer::GPUBuffer;
use crate::web_gpu::gpu_command_buffer::GPUCommandBuffer;
use crate::web_gpu::gpu_texture::GPUTexture;
use crate::web_gpu::host::{WebGPUConnection, op_err};

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUCopyExternalImageSourceInfo {
    pub(crate) source: Ref<ImageBitmap>,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUCopyExternalImageDestInfo {
    pub(crate) texture: Ref<GPUTexture>,
}

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUQueue {
    #[gc(untraced)]
    conn: Rc<WebGPUConnection>,
    #[gc(untraced)]
    device_id: u64,
    #[gc(untraced)]
    queue_id: u64,
    realm: Gc<Realm>,
}

impl Wrappable for GPUQueue {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUQueue {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, conn: Rc<WebGPUConnection>, device_id: u64, queue_id: u64) -> Ref<Self> {
        allocate(vm, |_| Self { conn, device_id, queue_id, realm })
    }

    pub(crate) fn submit(&self, _: &mut CallContext, command_buffers: MarkedVec<'_, Ref<GPUCommandBuffer>>) -> WebIdlResult<()> {
        let mut p = Vec::new();
        p.extend_from_slice(&self.device_id.to_le_bytes());
        p.extend_from_slice(&self.queue_id.to_le_bytes());
        for cb in command_buffers.values().iter() {
            p.extend_from_slice(&cb.take_commands());
        }
        self.conn.post(4, p, None);
        Ok(())
    }

    pub(crate) fn on_submitted_work_done(&self, cx: &mut CallContext) -> WebIdlResult<WebIdlPromise> {
        Ok(create_resolved_promise(cx.vm(), self.realm, Value::UNDEFINED))
    }

    pub(crate) fn write_buffer(&self, _: &mut CallContext, buffer: Ref<GPUBuffer>, buffer_offset: u64, data: BufferSource) -> WebIdlResult<()> {
        let len = data.extent().byte_length;
        if len == 0 { return Ok(()); }
        let shm = SharedBuffer::create(len).ok_or_else(|| op_err("shm alloc failed".into()))?;
        // SAFETY: No script execution or GC allocation occurs while borrowing `data`.
        unsafe { data.with_bytes(|b| core::ptr::copy_nonoverlapping(b.as_ptr(), shm.as_ptr(), len)) };
        let mut p = Vec::with_capacity(32);
        for v in [self.queue_id, buffer.buffer_id(), buffer_offset, len as u64] { p.extend_from_slice(&v.to_le_bytes()); }
        self.conn.post(2, p, Some(&shm));
        Ok(())
    }

    pub(crate) fn copy_external_image_to_texture(
        &self,
        _: &mut CallContext,
        source: GPUCopyExternalImageSourceInfo,
        destination: GPUCopyExternalImageDestInfo,
        copy_size: Vec<u32>,
    ) -> WebIdlResult<()> {
        let frame = source.source.frame().ok_or_else(|| op_err("ImageBitmap detached".into()))?;
        let (w, h) = (copy_size.first().copied().unwrap_or(0), copy_size.get(1).copied().unwrap_or(0));
        if w == 0 || h == 0 { return Ok(()); }
        let total = (w as usize) * (h as usize) * 4;
        let shm = SharedBuffer::create(total).ok_or_else(|| op_err("shm alloc failed".into()))?;
        // SAFETY: `frame` keeps pixels mapped; `shm` is writable for `total` bytes.
        let src = unsafe { ak::slice_from_raw_parts(frame.pixels().as_ptr(), frame.size_in_bytes()) };
        let dst = unsafe { core::slice::from_raw_parts_mut(shm.as_ptr(), total) };
        let (pitch, fmt, premul) = (frame.pitch(), frame.bitmap_format_value(), frame.alpha_type_value() == 0);
        let (bgra, opaque) = (fmt == 1 || fmt == 2, fmt == 1 || fmt == 3);
        for row in 0..h as usize {
            for col in 0..w as usize {
                let (si, di) = (row * pitch + col * 4, (row * (w as usize) + col) * 4);
                let a = if opaque { 255 } else { src[si + 3] };
                let (mut r, mut g, mut b) = if bgra { (src[si + 2], src[si + 1], src[si]) } else { (src[si], src[si + 1], src[si + 2]) };
                if premul && a < 255 {
                    let un = |c: u8| if a == 0 { 0 } else { ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8 };
                    (r, g, b) = (un(r), un(g), un(b));
                }
                dst[di..di + 4].copy_from_slice(&[r, g, b, a]);
            }
        }
        let mut p = Vec::with_capacity(24);
        p.extend_from_slice(&self.queue_id.to_le_bytes());
        p.extend_from_slice(&destination.texture.texture_id().to_le_bytes());
        p.extend_from_slice(&w.to_le_bytes());
        p.extend_from_slice(&h.to_le_bytes());
        self.conn.post(3, p, Some(&shm));
        Ok(())
    }
}
