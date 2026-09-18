//! The exported C ABI functions. Every declaration here has an exactly
//! matching prototype in `include/taffy.h` (the "interface closed loop":
//! header declaration -> Rust definition -> C-side call sites).

#![allow(clippy::missing_safety_doc)]
// The exported functions mention the private `FfiState` wrapper only through
// raw, type-erased pointers on the C ABI; the warning is a false positive for
// an FFI crate whose module is itself private.
#![allow(private_interfaces)]

use core::ffi::c_void;
use core::ptr;
use core::slice;
use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use taffy::geometry::Size;
use taffy::style::AvailableSpace;
use taffy::tree::{LayoutInput, LayoutOutput, NodeId, TaffyError, TaffyTree};
use taffy::{compute_leaf_layout, TraversePartialTree};

use crate::convert;
use crate::ctypes::*;

/// Concrete tree type used inside [`FfiState`].
///
/// The `NodeContext` is a raw host pointer (native peer object), handed back
/// to the C measure callback for leaf measurement. Raw pointers are neither
/// `Send` nor `Sync`: a tree must only be accessed from one thread at a time.
type FfiTree = TaffyTree<*mut c_void>;

/// State behind the opaque `taffy_tree_t`.
///
/// taffy 0.14 panics on slotmap indexing when given an unknown [`NodeId`]
/// (even its own `TaffyResult`-returning methods do), so the FFI layer keeps
/// its own set of live ids and validates every node argument up front. This
/// turns invalid-id programmer errors into plain
/// `TAFFY_ERR_INVALID_NODE` status codes instead of caught panics.
struct FfiState {
    tree: FfiTree,
    /// Ids of every node currently owned by the tree. Membership changes only
    /// on node creation, `taffy_node_remove()` and `taffy_tree_clear()`
    /// (hierarchy detach operations keep the node alive).
    live: HashSet<u64>,
}

impl FfiState {
    /// Returns `true` when `node` is currently owned by the tree.
    #[inline]
    fn is_live(&self, node: u64) -> bool {
        self.live.contains(&node)
    }
}

const TAFFY_C_VERSION: &[u8] = b"0.1.0\0";

/// Run `f` while catching any Rust panic so it never unwinds across the FFI
/// boundary (which would be undefined behaviour).
#[inline]
fn guard<F>(f: F) -> u32
where
    F: FnOnce() -> u32,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(code) => code,
        Err(_) => TAFFY_ERR_UNKNOWN,
    }
}

/// Map a taffy error onto its C status code.
#[inline]
fn map_err(e: TaffyError) -> u32 {
    match e {
        TaffyError::ChildIndexOutOfBounds { .. } => TAFFY_ERR_INDEX_OUT_OF_BOUNDS,
        TaffyError::InvalidParentNode(_) | TaffyError::InvalidChildNode(_) | TaffyError::InvalidInputNode(_) => {
            TAFFY_ERR_INVALID_NODE
        }
    }
}

/// Convert a raw id into a taffy [`NodeId`].
#[inline]
fn nid(id: u64) -> NodeId {
    NodeId::from(id)
}

// ===========================================================================
// Lifecycle / configuration
// ===========================================================================

#[no_mangle]
pub extern "C" fn taffy_c_version() -> *const core::ffi::c_char {
    TAFFY_C_VERSION.as_ptr() as *const core::ffi::c_char
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_new() -> *mut FfiState {
    Box::into_raw(Box::new(FfiState { tree: FfiTree::new(), live: HashSet::new() }))
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_new_with_capacity(capacity: usize) -> *mut FfiState {
    Box::into_raw(Box::new(FfiState { tree: FfiTree::with_capacity(capacity), live: HashSet::with_capacity(capacity) }))
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_free(tree: *mut FfiState) {
    if !tree.is_null() {
        drop(Box::from_raw(tree));
    }
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_clear(tree: *mut FfiState) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        (*tree).tree.clear();
        (*tree).live.clear();
        TAFFY_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_enable_rounding(tree: *mut FfiState) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        (*tree).tree.enable_rounding();
        TAFFY_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_disable_rounding(tree: *mut FfiState) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        (*tree).tree.disable_rounding();
        TAFFY_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_total_node_count(tree: *const FfiState, out_count: *mut usize) -> u32 {
    guard(|| {
        if tree.is_null() || out_count.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        *out_count = (*tree).tree.total_node_count();
        TAFFY_OK
    })
}

// ===========================================================================
// Style
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn taffy_style_default(out_style: *mut CTaffyStyle) -> u32 {
    guard(|| {
        if out_style.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        *out_style = convert::default_style_to_c();
        TAFFY_OK
    })
}

// ===========================================================================
// Node creation / removal / styling
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn taffy_node_new_leaf(
    tree: *mut FfiState,
    style: *const CTaffyStyle,
    out_node: *mut u64,
) -> u32 {
    guard(|| {
        if tree.is_null() || style.is_null() || out_node.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        let taffy_style = match convert::style_from_c(&*style) {
            Ok(s) => s,
            Err(e) => return e.0,
        };
        match (*tree).tree.new_leaf(taffy_style) {
            Ok(id) => {
                (*tree).live.insert(u64::from(id));
                *out_node = u64::from(id);
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_new_with_children(
    tree: *mut FfiState,
    style: *const CTaffyStyle,
    children: *const u64,
    child_count: usize,
    out_node: *mut u64,
) -> u32 {
    guard(|| {
        if tree.is_null() || style.is_null() || out_node.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if child_count > 0 && children.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        let raw_ids: Vec<u64> =
            if child_count == 0 { Vec::new() } else { slice::from_raw_parts(children, child_count).to_vec() };
        // taffy panics when the child list contains an unknown id, so validate
        // every reference before mutating anything.
        if raw_ids.iter().any(|id| !(*tree).is_live(*id)) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let taffy_style = match convert::style_from_c(&*style) {
            Ok(s) => s,
            Err(e) => return e.0,
        };
        let kids: Vec<NodeId> = raw_ids.iter().copied().map(nid).collect();
        match (*tree).tree.new_with_children(taffy_style, &kids) {
            Ok(id) => {
                (*tree).live.insert(u64::from(id));
                *out_node = u64::from(id);
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_remove(tree: *mut FfiState, node: u64) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.remove(nid(node)) {
            Ok(_) => {
                (*tree).live.remove(&node);
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_set_style(tree: *mut FfiState, node: u64, style: *const CTaffyStyle) -> u32 {
    guard(|| {
        if tree.is_null() || style.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let taffy_style = match convert::style_from_c(&*style) {
            Ok(s) => s,
            Err(e) => return e.0,
        };
        match (*tree).tree.set_style(nid(node), taffy_style) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_set_context(tree: *mut FfiState, node: u64, context: *mut c_void) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.set_node_context(nid(node), Some(context)) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_get_context(
    tree: *const FfiState,
    node: u64,
    out_context: *mut *mut c_void,
) -> u32 {
    guard(|| {
        if tree.is_null() || out_context.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        // Nodes without a context report NULL.
        *out_context = (*tree).tree.get_node_context(nid(node)).copied().unwrap_or(ptr::null_mut());
        TAFFY_OK
    })
}

// ===========================================================================
// Hierarchy mutation
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn taffy_node_add_child(tree: *mut FfiState, parent: u64, child: u64) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) || !(*tree).is_live(child) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.add_child(nid(parent), nid(child)) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_insert_child_at_index(
    tree: *mut FfiState,
    parent: u64,
    index: usize,
    child: u64,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) || !(*tree).is_live(child) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.insert_child_at_index(nid(parent), index, nid(child)) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_set_children(
    tree: *mut FfiState,
    parent: u64,
    children: *const u64,
    child_count: usize,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if child_count > 0 && children.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let raw_ids: Vec<u64> =
            if child_count == 0 { Vec::new() } else { slice::from_raw_parts(children, child_count).to_vec() };
        if raw_ids.iter().any(|id| !(*tree).is_live(*id)) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let kids: Vec<NodeId> = raw_ids.iter().copied().map(nid).collect();
        match (*tree).tree.set_children(nid(parent), &kids) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_replace_child_at_index(
    tree: *mut FfiState,
    parent: u64,
    index: usize,
    new_child: u64,
    out_old_child: *mut u64,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) || !(*tree).is_live(new_child) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.replace_child_at_index(nid(parent), index, nid(new_child)) {
            Ok(old) => {
                if !out_old_child.is_null() {
                    *out_old_child = u64::from(old);
                }
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_remove_child(
    tree: *mut FfiState,
    parent: u64,
    child: u64,
    out_removed_child: *mut u64,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) || !(*tree).is_live(child) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.remove_child(nid(parent), nid(child)) {
            Ok(removed) => {
                if !out_removed_child.is_null() {
                    *out_removed_child = u64::from(removed);
                }
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_remove_child_at_index(
    tree: *mut FfiState,
    parent: u64,
    index: usize,
    out_removed_child: *mut u64,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.remove_child_at_index(nid(parent), index) {
            Ok(removed) => {
                if !out_removed_child.is_null() {
                    *out_removed_child = u64::from(removed);
                }
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

// ===========================================================================
// Queries
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn taffy_node_child_at_index(
    tree: *const FfiState,
    parent: u64,
    index: usize,
    out_child: *mut u64,
) -> u32 {
    guard(|| {
        if tree.is_null() || out_child.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(parent) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.child_at_index(nid(parent), index) {
            Ok(id) => {
                *out_child = u64::from(id);
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_child_count(tree: *const FfiState, node: u64, out_count: *mut usize) -> u32 {
    guard(|| {
        if tree.is_null() || out_count.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        *out_count = (*tree).tree.child_count(nid(node));
        TAFFY_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_children(
    tree: *const FfiState,
    node: u64,
    out_children: *mut u64,
    capacity: usize,
    out_count: *mut usize,
) -> u32 {
    guard(|| {
        if tree.is_null() || out_count.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let kids = match (*tree).tree.children(nid(node)) {
            Ok(kids) => kids,
            Err(e) => return map_err(e),
        };
        *out_count = kids.len();
        if out_children.is_null() || capacity < kids.len() {
            return TAFFY_ERR_BUFFER_TOO_SMALL;
        }
        ptr::copy_nonoverlapping(kids.as_ptr() as *const u64, out_children, kids.len());
        TAFFY_OK
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_parent(tree: *const FfiState, node: u64, out_parent: *mut u64) -> u32 {
    guard(|| {
        if tree.is_null() || out_parent.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.parent(nid(node)) {
            Some(parent) => {
                *out_parent = u64::from(parent);
                TAFFY_OK
            }
            None => TAFFY_ERR_NO_PARENT,
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_dirty(tree: *const FfiState, node: u64, out_dirty: *mut core::ffi::c_int) -> u32 {
    guard(|| {
        if tree.is_null() || out_dirty.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.dirty(nid(node)) {
            Ok(dirty) => {
                *out_dirty = dirty as core::ffi::c_int;
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_mark_dirty(tree: *mut FfiState, node: u64) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.mark_dirty(nid(node)) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

// ===========================================================================
// Layout computation & output
// ===========================================================================

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_compute_layout(
    tree: *mut FfiState,
    root: u64,
    available_width: CTaffyAvailableSpace,
    available_height: CTaffyAvailableSpace,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(root) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let available = Size {
            width: convert::map_available_space(available_width),
            height: convert::map_available_space(available_height),
        };
        match (*tree).tree.compute_layout(nid(root), available) {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_compute_layout_with_measure(
    tree: *mut FfiState,
    root: u64,
    available_width: CTaffyAvailableSpace,
    available_height: CTaffyAvailableSpace,
    callback: Option<CTaffyMeasureCallback>,
    measure_context: *mut c_void,
) -> u32 {
    guard(|| {
        if tree.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(root) {
            return TAFFY_ERR_INVALID_NODE;
        }
        let available = Size {
            width: convert::map_available_space(available_width),
            height: convert::map_available_space(available_height),
        };

        let result = (*tree).tree.compute_layout_with_measure(
            nid(root),
            available,
            move |inputs: LayoutInput,
                  node: NodeId,
                  node_context: Option<&mut *mut c_void>,
                  style: &taffy::Style|
                  -> LayoutOutput {
                // Mirrors taffy's built-in measure path in compute_layout(),
                // except the final leaf-size closure dispatches to the host
                // callback so text/image widgets can report intrinsic sizes.
                compute_leaf_layout(
                    inputs,
                    style,
                    |_calc: *const (), _basis: f32| 0.0,
                    |known_dimensions: Size<Option<f32>>, available_space: Size<AvailableSpace>| -> Size<f32> {
                        match callback {
                            Some(cb) => {
                                let known = [
                                    known_dimensions.width.unwrap_or(f32::NAN),
                                    known_dimensions.height.unwrap_or(f32::NAN),
                                ];
                                let avail = [
                                    convert::available_space_to_c(available_space.width),
                                    convert::available_space_to_c(available_space.height),
                                ];
                                let ctx = node_context.map(|p| *p).unwrap_or(ptr::null_mut());
                                let mut out = [0.0f32; 2];
                                cb(
                                    u64::from(node),
                                    known.as_ptr(),
                                    avail.as_ptr(),
                                    ctx,
                                    measure_context,
                                    out.as_mut_ptr(),
                                );
                                Size { width: out[0], height: out[1] }
                            }
                            // No callback supplied: leaves resolve to a 0x0 box,
                            // mirroring taffy's own compute_layout() behaviour.
                            None => Size::ZERO,
                        }
                    },
                )
            },
        );

        match result {
            Ok(_) => TAFFY_OK,
            Err(e) => map_err(e),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn taffy_node_layout(tree: *const FfiState, node: u64, out_layout: *mut CTaffyLayout) -> u32 {
    guard(|| {
        if tree.is_null() || out_layout.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(node) {
            return TAFFY_ERR_INVALID_NODE;
        }
        match (*tree).tree.layout(nid(node)) {
            Ok(layout) => {
                *out_layout = convert::layout_to_c(layout);
                TAFFY_OK
            }
            Err(e) => map_err(e),
        }
    })
}

/// Recursively append `[x, y, width, height, child_count, order]` for `node`
/// and every descendant, depth-first pre-order.
fn dfs_append_layouts(tree: &FfiTree, node: NodeId, out: &mut Vec<f32>) -> Result<(), TaffyError> {
    let layout = tree.layout(node)?;
    // child_count() comes from the TraversePartialTree trait.
    let child_count = tree.child_count(node);
    out.extend_from_slice(&[
        layout.location.x,
        layout.location.y,
        layout.size.width,
        layout.size.height,
        child_count as f32,
        layout.order as f32,
    ]);
    for child in tree.child_ids(node) {
        dfs_append_layouts(tree, child, out)?;
    }
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn taffy_tree_write_layouts_dfs(
    tree: *const FfiState,
    root: u64,
    out_buffer: *mut f32,
    capacity: usize,
    out_required: *mut usize,
) -> u32 {
    guard(|| {
        if tree.is_null() || out_required.is_null() {
            return TAFFY_ERR_NULL_POINTER;
        }
        if !(*tree).is_live(root) {
            return TAFFY_ERR_INVALID_NODE;
        }

        let mut records: Vec<f32> = Vec::new();
        if let Err(e) = dfs_append_layouts(&(*tree).tree, nid(root), &mut records) {
            return map_err(e);
        }

        *out_required = records.len();
        if out_buffer.is_null() || capacity < records.len() {
            return TAFFY_ERR_BUFFER_TOO_SMALL;
        }
        ptr::copy_nonoverlapping(records.as_ptr(), out_buffer, records.len());
        TAFFY_OK
    })
}
