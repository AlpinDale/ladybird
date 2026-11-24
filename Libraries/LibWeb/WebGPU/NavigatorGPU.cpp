/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

#include <LibWeb/HTML/Scripting/Environments.h>
#include <LibWeb/WebGPU/GPU.h>
#include <LibWeb/WebGPU/NavigatorGPU.h>

namespace Web::WebGPU {

// https://gpuweb.github.io/gpuweb/#dom-navigatorgpu-gpu
GC::Ref<GPU> NavigatorGPU::gpu()
{
    return this_navigator_gpu_settings_object().gpu();
}

}
