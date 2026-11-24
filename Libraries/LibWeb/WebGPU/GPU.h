/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

#pragma once

#include <LibJS/Forward.h>
#include <LibWeb/Bindings/GPU.h>
#include <LibWeb/Bindings/RustInterface.h>
#include <LibWeb/Forward.h>

namespace Web::WebGPU {

class GPU : public Bindings::RustInterfaceCell {
    WEB_RUST_INTERFACE(GPU, gpu)

public:
    static GC::Ref<GPU> create(HTML::EnvironmentSettingsObject&);
};

}
