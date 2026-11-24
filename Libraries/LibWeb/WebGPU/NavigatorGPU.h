/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

#pragma once

#include <LibGC/Ptr.h>
#include <LibWeb/Bindings/Wrappable.h>
#include <LibWeb/Forward.h>

namespace Web::WebGPU {

class NavigatorGPU {
public:
    virtual ~NavigatorGPU() = default;

    GC::Ref<GPU> gpu();

protected:
    virtual Bindings::Wrappable const& this_navigator_gpu_object() const = 0;
    virtual HTML::EnvironmentSettingsObject& this_navigator_gpu_settings_object() const = 0;
};

}
