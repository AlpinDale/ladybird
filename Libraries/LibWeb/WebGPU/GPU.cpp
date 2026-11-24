/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

#include <LibJS/HostClassBuilder.h>
#include <LibJS/Runtime/Realm.h>
#include <LibWeb/Compositor/CompositorHost.h>
#include <LibWeb/HTML/Scripting/Environments.h>
#include <LibWeb/HTML/Window.h>
#include <LibWeb/HTML/WorkerGlobalScope.h>
#include <LibWeb/Page/Page.h>
#include <LibWeb/WebGPU/GPU.h>

namespace Web::WebGPU {

extern "C" {
void* web_rust_web_gpu_create_gpu(JSVM*, void* realm);
void* web_gpu_compositor_connection(HTML::EnvironmentSettingsObject*);
}

GC::Ref<GPU> GPU::create(HTML::EnvironmentSettingsObject& environment)
{
    return *static_cast<GPU*>(web_rust_web_gpu_create_gpu(JS::HostABI::vm_to_abi(environment.realm().vm()), &environment.realm()));
}

void* web_gpu_compositor_connection(HTML::EnvironmentSettingsObject* environment)
{
    if (!environment)
        return nullptr;

    auto& global = environment->global_object();
    Page* page = nullptr;
    if (auto* window = HTML::window_from_global_object(global))
        page = &window->page();
    else if (auto* worker = Bindings::worker_global_scope_from_global_object(global))
        page = worker->page();

    if (!page || !page->has_compositor_host())
        return nullptr;

    return page->compositor_host().ref_compositor_connection();
}

}
