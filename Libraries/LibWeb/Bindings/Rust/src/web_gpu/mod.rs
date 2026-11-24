/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

pub(crate) mod gpu;
pub(crate) mod gpu_adapter;
pub(crate) mod gpu_bind_group;
pub(crate) mod gpu_bind_group_layout;
pub(crate) mod gpu_buffer;
pub(crate) mod gpu_command_buffer;
pub(crate) mod gpu_command_encoder;
pub(crate) mod gpu_compute_pass_encoder;
pub(crate) mod gpu_compute_pipeline;
pub(crate) mod gpu_device;
pub(crate) mod gpu_queue;
pub(crate) mod gpu_shader_module;
pub(crate) mod gpu_texture;
pub(crate) mod gpu_texture_view;
pub(crate) mod host;

use ak::{Utf16String, Utf16StringUnits};

pub(crate) fn utf16_to_string(s: &Utf16String) -> String {
    match s.as_units() {
        Utf16StringUnits::Ascii(u) => String::from_utf8_lossy(u).into_owned(),
        Utf16StringUnits::Latin1(u) => u.iter().map(|&b| char::from(b)).collect(),
        Utf16StringUnits::Utf16(u) => String::from_utf16_lossy(u),
    }
}
