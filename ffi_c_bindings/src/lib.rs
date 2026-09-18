//! C FFI bindings for [taffy](https://github.com/DioxusLabs/taffy).
//!
//! The crate builds a `staticlib`/`cdylib` exposing a plain C ABI (see
//! `include/taffy.h`) on top of taffy's high-level [`TaffyTree`] API so that
//! the layout engine can be embedded from C++, Kotlin/JNI (Android NDK),
//! ArkTS/N-API (OpenHarmony) and Swift.
//!
//! Modules:
//!   - [`ctypes`]  - `#[repr(C)]` ABI types and numeric constants.
//!   - [`convert`] - conversions between the ABI types and taffy's types.
//!   - [`ffi`]     - the exported `extern "C"` functions.

#![allow(clippy::missing_safety_doc)]

mod convert;
mod ctypes;
mod ffi;
