/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

//! Minimal out-of-process WebGPU compute + texture backend backed by `wgpu-core`.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use wgpu_core::binding_model::{BindGroupDescriptor, BindGroupEntry, BindingResource, BufferBinding};
use wgpu_core::command::ComputePassDescriptor;
use wgpu_core::device::HostMap;
use wgpu_core::global::Global;
use wgpu_core::id::{
    AdapterId, BindGroupId, BindGroupLayoutId, BufferId, ComputePipelineId, DeviceId, QueueId, ShaderModuleId,
    TextureId, TextureViewId,
};
use wgpu_core::instance::RequestAdapterOptions;
use wgpu_core::pipeline::{ComputePipelineDescriptor, ProgrammableStageDescriptor, ShaderModuleDescriptor, ShaderModuleSource};
use wgpu_core::resource::{BufferDescriptor, BufferMapOperation, TextureDescriptor, TextureViewDescriptor};
use wgpu_types as wgt;

use crate::host::WebContentClientId;
use crate::raster_host::SharedMemory;

#[derive(Default)]
struct ClientResources {
    adapters: HashMap<u64, AdapterId>,
    devices: HashMap<u64, DeviceId>,
    queues: HashMap<u64, QueueId>,
    buffers: HashMap<u64, BufferId>,
    textures: HashMap<u64, TextureId>,
    texture_views: HashMap<u64, TextureViewId>,
    shader_modules: HashMap<u64, ShaderModuleId>,
    compute_pipelines: HashMap<u64, ComputePipelineId>,
    bind_group_layouts: HashMap<u64, BindGroupLayoutId>,
    bind_groups: HashMap<u64, BindGroupId>,
}

#[derive(Default)]
pub struct WebGPUContexts {
    global: Option<Global>,
    next_id: u64,
    clients: HashMap<WebContentClientId, ClientResources>,
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Result<u8, String> {
        let (&v, rest) = self.0.split_first().ok_or("truncated u8")?;
        self.0 = rest;
        Ok(v)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let (b, rest) = self.0.split_at_checked(4).ok_or("truncated u32")?;
        self.0 = rest;
        Ok(u32::from_le_bytes(b.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, String> {
        let (b, rest) = self.0.split_at_checked(8).ok_or("truncated u64")?;
        self.0 = rest;
        Ok(u64::from_le_bytes(b.try_into().unwrap()))
    }
}

fn push_limits(out: &mut Vec<u8>, l: &wgt::Limits) {
    for v in [
        l.max_storage_buffer_binding_size,
        l.max_buffer_size,
        u64::from(l.max_compute_workgroup_storage_size),
        u64::from(l.max_storage_buffers_per_shader_stage),
        u64::from(l.max_texture_dimension_2d),
    ] {
        out.extend_from_slice(&v.to_le_bytes());
    }
}

impl WebGPUContexts {
    fn global(&mut self) -> &Global {
        self.global.get_or_insert_with(|| {
            let mut desc = wgt::InstanceDescriptor::new_without_display_handle();
            desc.backends = wgt::Backends::PRIMARY;
            Global::new("ladybird-webgpu", desc, None)
        })
    }

    fn alloc(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub fn forget_client(&mut self, client: WebContentClientId) {
        let (Some(r), Some(g)) = (self.clients.remove(&client), self.global.as_ref()) else {
            return;
        };
        r.bind_groups.into_values().for_each(|id| g.bind_group_drop(id));
        r.bind_group_layouts.into_values().for_each(|id| g.bind_group_layout_drop(id));
        r.compute_pipelines.into_values().for_each(|id| g.compute_pipeline_drop(id));
        r.shader_modules.into_values().for_each(|id| g.shader_module_drop(id));
        r.texture_views.into_values().for_each(|id| g.texture_view_drop(id));
        for id in r.textures.into_values() {
            g.texture_destroy(id);
            g.texture_drop(id);
        }
        for id in r.buffers.into_values() {
            g.buffer_destroy(id);
            g.buffer_drop(id);
        }
        r.queues.into_values().for_each(|id| g.queue_drop(id));
        r.devices.into_values().for_each(|id| g.device_drop(id));
        r.adapters.into_values().for_each(|id| g.adapter_drop(id));
    }

    pub fn call(
        &mut self,
        client: WebContentClientId,
        op: u8,
        payload: &[u8],
        shm: Option<&SharedMemory>,
    ) -> Result<Vec<u8>, String> {
        let mut r = Reader(payload);
        let id_reply = |id: u64| id.to_le_bytes().to_vec();
        match op {
            1 => {
                let opts = RequestAdapterOptions {
                    power_preference: wgt::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                    apply_limit_buckets: false,
                };
                let adapter = self
                    .global()
                    .request_adapter(&opts, wgt::Backends::PRIMARY, None)
                    .map_err(|e| e.to_string())?;
                let g = self.global();
                let has_f16 = u8::from(g.adapter_features(adapter).contains(wgt::Features::SHADER_F16));
                let limits = g.adapter_limits(adapter);
                let h = self.alloc();
                self.clients.entry(client).or_default().adapters.insert(h, adapter);
                let mut out = Vec::with_capacity(49);
                out.extend_from_slice(&h.to_le_bytes());
                out.push(has_f16);
                push_limits(&mut out, &limits);
                Ok(out)
            }
            2 => {
                let adapter = *self.clients.get(&client).and_then(|c| c.adapters.get(&r.u64().ok()?)).ok_or("unknown adapter")?;
                let al = self.global().adapter_limits(adapter);
                let mut req = wgt::Limits::default();
                let cap32 = |v: u64, m: u32| u32::try_from(v).unwrap_or(u32::MAX).min(m);
                let l = [r.u64()?, r.u64()?, r.u64()?, r.u64()?, r.u64()?];
                if l[0] != 0 { req.max_storage_buffer_binding_size = l[0].min(al.max_storage_buffer_binding_size); }
                if l[1] != 0 { req.max_buffer_size = l[1].min(al.max_buffer_size); }
                if l[2] != 0 { req.max_compute_workgroup_storage_size = cap32(l[2], al.max_compute_workgroup_storage_size); }
                if l[3] != 0 { req.max_storage_buffers_per_shader_stage = cap32(l[3], al.max_storage_buffers_per_shader_stage); }
                if l[4] != 0 { req.max_texture_dimension_2d = cap32(l[4], al.max_texture_dimension_2d); }
                let feats = self.global().adapter_features(adapter) & wgt::Features::SHADER_F16;
                let desc = wgt::DeviceDescriptor {
                    label: None,
                    required_features: feats,
                    required_limits: req,
                    experimental_features: wgt::ExperimentalFeatures::disabled(),
                    memory_hints: wgt::MemoryHints::Performance,
                    trace: wgt::Trace::Off,
                };
                let (dev, q) = self.global().adapter_request_device(adapter, &desc, None, None).map_err(|e| e.to_string())?;
                let dl = self.global().device_limits(dev);
                let (dh, qh) = (self.alloc(), self.alloc());
                let c = self.clients.entry(client).or_default();
                c.devices.insert(dh, dev);
                c.queues.insert(qh, q);
                let mut out = Vec::with_capacity(56);
                out.extend_from_slice(&dh.to_le_bytes());
                out.extend_from_slice(&qh.to_le_bytes());
                push_limits(&mut out, &dl);
                Ok(out)
            }
            3 => {
                let (dh, size, usage) = (r.u64()?, r.u64()?, r.u32()?);
                let dev = *self.clients.get(&client).and_then(|c| c.devices.get(&dh)).ok_or("unknown device")?;
                let (id, err) = self.global().device_create_buffer(
                    dev,
                    &BufferDescriptor { label: None, size, usage: wgt::BufferUsages::from_bits_truncate(usage), mapped_at_creation: false },
                    None,
                );
                if let Some(e) = err { self.global().buffer_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().buffers.insert(h, id);
                Ok(id_reply(h))
            }
            4 => {
                let (dh, width, height, usage) = (r.u64()?, r.u32()?, r.u32()?, r.u32()?);
                let dev = *self.clients.get(&client).and_then(|c| c.devices.get(&dh)).ok_or("unknown device")?;
                let (id, err) = self.global().device_create_texture(
                    dev,
                    &TextureDescriptor {
                        label: None,
                        size: wgt::Extent3d { width, height, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgt::TextureDimension::D2,
                        format: wgt::TextureFormat::Rgba8Unorm,
                        usage: wgt::TextureUsages::from_bits_truncate(usage),
                        view_formats: Vec::new(),
                    },
                    None,
                );
                if let Some(e) = err { self.global().texture_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().textures.insert(h, id);
                Ok(id_reply(h))
            }
            5 => {
                let th = r.u64()?;
                let tex = *self.clients.get(&client).and_then(|c| c.textures.get(&th)).ok_or("unknown texture")?;
                let (id, err) = self.global().texture_create_view(tex, &TextureViewDescriptor::default(), None);
                if let Some(e) = err { self.global().texture_view_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().texture_views.insert(h, id);
                Ok(id_reply(h))
            }
            6 => {
                let dh = r.u64()?;
                let code = std::str::from_utf8(r.0).map_err(|e| e.to_string())?;
                let dev = *self.clients.get(&client).and_then(|c| c.devices.get(&dh)).ok_or("unknown device")?;
                let (id, err) = self.global().device_create_shader_module(
                    dev,
                    &ShaderModuleDescriptor { label: None, runtime_checks: wgt::ShaderRuntimeChecks::checked() },
                    ShaderModuleSource::Wgsl(Cow::Borrowed(code)),
                    None,
                );
                if let Some(e) = err { self.global().shader_module_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().shader_modules.insert(h, id);
                Ok(id_reply(h))
            }
            7 => {
                let (dh, mh) = (r.u64()?, r.u64()?);
                let (dev, module) = {
                    let c = self.clients.get(&client).ok_or("unknown client")?;
                    (*c.devices.get(&dh).ok_or("unknown device")?, *c.shader_modules.get(&mh).ok_or("unknown shader")?)
                };
                let (id, err) = self.global().device_create_compute_pipeline(
                    dev,
                    &ComputePipelineDescriptor {
                        label: None,
                        layout: None,
                        stage: ProgrammableStageDescriptor { module, entry_point: Some(Cow::Borrowed("main")), constants: Default::default(), zero_initialize_workgroup_memory: true },
                        cache: None,
                    },
                    None,
                );
                if let Some(e) = err { self.global().compute_pipeline_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().compute_pipelines.insert(h, id);
                Ok(id_reply(h))
            }
            8 => {
                let (ph, idx) = (r.u64()?, r.u32()?);
                let pipe = *self.clients.get(&client).and_then(|c| c.compute_pipelines.get(&ph)).ok_or("unknown pipeline")?;
                let (id, err) = self.global().compute_pipeline_get_bind_group_layout(pipe, idx, None);
                if let Some(e) = err { self.global().bind_group_layout_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().bind_group_layouts.insert(h, id);
                Ok(id_reply(h))
            }
            9 => {
                let (dh, lh, count) = (r.u64()?, r.u64()?, r.u32()? as usize);
                let (dev, layout, entries) = {
                    let c = self.clients.get(&client).ok_or("unknown client")?;
                    let dev = *c.devices.get(&dh).ok_or("unknown device")?;
                    let layout = *c.bind_group_layouts.get(&lh).ok_or("unknown layout")?;
                    let mut parsed = Vec::with_capacity(count);
                    for _ in 0..count {
                        let kind = r.u8()?;
                        let binding = r.u32()?;
                        let resource = match kind {
                            1 => {
                                let (bh, offset) = (r.u64()?, r.u64()?);
                                BindingResource::Buffer(BufferBinding {
                                    buffer: *c.buffers.get(&bh).ok_or("unknown buffer")?,
                                    offset,
                                    size: None,
                                })
                            }
                            2 => BindingResource::TextureView(*c.texture_views.get(&r.u64()?).ok_or("unknown view")?),
                            _ => return Err("unknown bind resource".into()),
                        };
                        parsed.push(BindGroupEntry { binding, resource });
                    }
                    (dev, layout, parsed)
                };
                let (id, err) = self.global().device_create_bind_group(
                    dev,
                    &BindGroupDescriptor { label: None, layout, entries: Cow::Owned(entries) },
                    None,
                );
                if let Some(e) = err { self.global().bind_group_drop(id); return Err(e.to_string()); }
                let h = self.alloc();
                self.clients.entry(client).or_default().bind_groups.insert(h, id);
                Ok(id_reply(h))
            }
            10 => {
                let (dh, bh, size) = (r.u64()?, r.u64()?, r.u64()?);
                let mem = shm.ok_or("missing shared memory")?;
                let len = usize::try_from(size).map_err(|_| "size overflow")?;
                if len > mem.len() { return Err("shm too small".into()); }
                let (dev, buf) = {
                    let c = self.clients.get(&client).ok_or("unknown client")?;
                    (*c.devices.get(&dh).ok_or("unknown device")?, *c.buffers.get(&bh).ok_or("unknown buffer")?)
                };
                let g = self.global();
                let res = Arc::new(Mutex::new(None));
                let res_cb = Arc::clone(&res);
                g.buffer_map_async(buf, 0, Some(size), BufferMapOperation {
                    host: HostMap::Read,
                    callback: Some(Box::new(move |r| { if let Ok(mut guard) = res_cb.lock() { *guard = Some(r); } })),
                }).map_err(|e| e.to_string())?;
                g.device_poll(dev, wgt::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;
                match res.lock().map_err(|_| "lock poisoned")?.take() {
                    Some(Ok(())) => {}
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Err("map callback missing".into()),
                }
                let (ptr, mapped) = g.buffer_get_mapped_range(buf, 0, Some(size)).map_err(|e| e.to_string())?;
                let copy_len = usize::try_from(mapped).unwrap_or(0).min(len);
                // SAFETY: `ptr` is readable for `copy_len` bytes; `mem` is writable for `len` bytes.
                unsafe { std::ptr::copy_nonoverlapping(ptr.as_ptr(), mem.as_ptr(), copy_len) };
                let _ = g.buffer_unmap(buf);
                Ok(Vec::new())
            }
            _ => Err("unknown webgpu op".into()),
        }
    }

    pub fn post(&mut self, client: WebContentClientId, op: u8, payload: &[u8], shm: Option<&SharedMemory>) {
        let mut r = Reader(payload);
        let _ = match op {
            1 => {
                let (Ok(kind), Ok(h)) = (r.u8(), r.u64()) else { return };
                let (Some(c), Some(g)) = (self.clients.get_mut(&client), self.global.as_ref()) else { return };
                match kind {
                    1 => if let Some(id) = c.devices.remove(&h) { g.device_drop(id); },
                    2 => if let Some(id) = c.buffers.remove(&h) { g.buffer_destroy(id); g.buffer_drop(id); },
                    3 => if let Some(id) = c.textures.remove(&h) { g.texture_destroy(id); g.texture_drop(id); },
                    _ => {}
                }
                Ok::<(), String>(())
            }
            2 => {
                let (Ok(qh), Ok(bh), Ok(off), Ok(sz)) = (r.u64(), r.u64(), r.u64(), r.u64()) else { return };
                let Some(mem) = shm else { return };
                let Some(len) = usize::try_from(sz).ok().filter(|&n| n <= mem.len()) else { return };
                let Some((q, b)) = self.clients.get(&client).and_then(|c| Some((*c.queues.get(&qh)?, *c.buffers.get(&bh)?))) else { return };
                // SAFETY: `mem` is mapped for `mem.len()` bytes and `len <= mem.len()`.
                let bytes = unsafe { std::slice::from_raw_parts(mem.as_ptr(), len) };
                let _ = self.global().queue_write_buffer(q, b, off, bytes);
                Ok(())
            }
            3 => {
                let (Ok(qh), Ok(th), Ok(w), Ok(h)) = (r.u64(), r.u64(), r.u32(), r.u32()) else { return };
                let Some(mem) = shm else { return };
                let bpr = w * 4;
                let len = (bpr as usize) * (h as usize);
                if len > mem.len() { return; }
                let Some((q, t)) = self.clients.get(&client).and_then(|c| Some((*c.queues.get(&qh)?, *c.textures.get(&th)?))) else { return };
                // SAFETY: `mem` is mapped for `mem.len()` bytes and `len <= mem.len()`.
                let bytes = unsafe { std::slice::from_raw_parts(mem.as_ptr(), len) };
                let _ = self.global().queue_write_texture(
                    q,
                    &wgt::TexelCopyTextureInfo { texture: t, mip_level: 0, origin: wgt::Origin3d::ZERO, aspect: wgt::TextureAspect::All },
                    bytes,
                    &wgt::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(h) },
                    &wgt::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                );
                Ok(())
            }
            4 => {
                let (Ok(dh), Ok(qh)) = (r.u64(), r.u64()) else { return };
                let Some((dev, q)) = self.clients.get(&client).and_then(|c| Some((*c.devices.get(&dh)?, *c.queues.get(&qh)?))) else { return };
                let Some(g) = self.global.as_ref() else { return };
                let (enc, err) = g.device_create_command_encoder(dev, &wgt::CommandEncoderDescriptor { label: None }, None);
                if err.is_some() { g.command_encoder_drop(enc); return; }
                let Some(c) = self.clients.get(&client) else { g.command_encoder_drop(enc); return };
                let mut pass = None;
                while let Ok(cmd) = r.u8() {
                    match cmd {
                        1 => if let (p, None) = g.command_encoder_begin_compute_pass(enc, &ComputePassDescriptor { label: None, timestamp_writes: None }) { pass = Some(p); },
                        2 => if let (Ok(ph), Some(p)) = (r.u64(), pass.as_mut()) && let Some(&pid) = c.compute_pipelines.get(&ph) { let _ = g.compute_pass_set_pipeline(p, pid); },
                        3 => if let (Ok(idx), Ok(bgh), Some(p)) = (r.u32(), r.u64(), pass.as_mut()) {
                            let _ = g.compute_pass_set_bind_group(p, idx, (bgh != 0).then(|| c.bind_groups.get(&bgh).copied()).flatten(), &[]);
                        },
                        4 => if let (Ok(x), Ok(y), Ok(z), Some(p)) = (r.u32(), r.u32(), r.u32(), pass.as_mut()) { let _ = g.compute_pass_dispatch_workgroups(p, x, y, z); },
                        5 => if let Some(mut p) = pass.take() { let _ = g.compute_pass_end(&mut p); },
                        6 => if let (Ok(sh), Ok(so), Ok(dh), Ok(doff), Ok(sz)) = (r.u64(), r.u64(), r.u64(), r.u64(), r.u64())
                            && let (Some(&sb), Some(&db)) = (c.buffers.get(&sh), c.buffers.get(&dh))
                        {
                            let _ = g.command_encoder_copy_buffer_to_buffer(enc, sb, so, db, doff, Some(sz));
                        },
                        _ => break,
                    }
                }
                if let Some(mut p) = pass.take() { let _ = g.compute_pass_end(&mut p); }
                let (cb, err) = g.command_encoder_finish(enc, &wgt::CommandBufferDescriptor { label: None }, None);
                if err.is_none() { let _ = g.queue_submit(q, &[cb]); }
                g.command_buffer_drop(cb);
                Ok(())
            }
            _ => Ok(()),
        };
    }
}
