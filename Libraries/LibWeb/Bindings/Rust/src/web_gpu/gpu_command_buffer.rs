/*
 * Copyright (c) 2026-present, the Ladybird developers.
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use core::cell::RefCell;

use libjs_rust::api::{Gc, Realm, Trace, Vm};

use crate::bindings::interface::{Ref, Wrappable, allocate};

#[derive(Trace)]
#[gc(crate = libjs_rust)]
pub(crate) struct GPUCommandBuffer {
    #[gc(untraced)]
    commands: RefCell<Vec<u8>>,
    realm: Gc<Realm>,
}

impl Wrappable for GPUCommandBuffer {
    fn relevant_realm(&self) -> Option<Gc<Realm>> { Some(self.realm) }
}

impl GPUCommandBuffer {
    pub(crate) fn create(vm: &Vm, realm: Gc<Realm>, commands: Vec<u8>) -> Ref<Self> {
        allocate(vm, |_| Self { commands: RefCell::new(commands), realm })
    }
    pub(crate) fn take_commands(&self) -> Vec<u8> { core::mem::take(&mut *self.commands.borrow_mut()) }
}
