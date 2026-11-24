// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Pure Rust WebAssembly entry point for AnimaX Web (`platform/web/core`).
//! Re-exports all `animax-ffi` C ABI symbols plus linear-memory allocation helpers
//! so JS/TS can load `animax_wasm.wasm` directly without Emscripten or C++ Skity.

pub use animax_ffi::*;
use std::alloc::{alloc, dealloc, Layout};

#[no_mangle]
pub extern "C" fn animax_wasm_alloc(len: usize) -> *mut u8 {
    let size = len.max(1);
    let Ok(layout) = Layout::from_size_align(size, 8) else {
        return std::ptr::null_mut();
    };
    unsafe { alloc(layout) }
}

#[no_mangle]
pub unsafe extern "C" fn animax_wasm_free(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let size = len.max(1);
    if let Ok(layout) = Layout::from_size_align(size, 8) {
        dealloc(ptr, layout);
    }
}
