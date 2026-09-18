//! `#[repr(C)]` counterparts of the types declared in `include/taffy.h`.
//!
//! The field order, widths and padding here MUST stay byte-identical to the C
//! declarations. Both sides use natural alignment, so no packed repr is needed.

#![allow(non_camel_case_types)]

use core::ffi::c_void;

// ---------------------------------------------------------------------------
// Status codes (taffy_status_t)
// ---------------------------------------------------------------------------

pub const TAFFY_OK: u32 = 0;
pub const TAFFY_ERR_NULL_POINTER: u32 = 1;
pub const TAFFY_ERR_INVALID_NODE: u32 = 2;
pub const TAFFY_ERR_INDEX_OUT_OF_BOUNDS: u32 = 3;
pub const TAFFY_ERR_BUFFER_TOO_SMALL: u32 = 4;
pub const TAFFY_ERR_INVALID_ARGUMENT: u32 = 5;
pub const TAFFY_ERR_NO_PARENT: u32 = 6;
pub const TAFFY_ERR_UNKNOWN: u32 = 99;

// ---------------------------------------------------------------------------
// Enum discriminators
// ---------------------------------------------------------------------------

// taffy_display_t
pub const TAFFY_DISPLAY_NONE: u8 = 0;
pub const TAFFY_DISPLAY_FLEX: u8 = 1;
pub const TAFFY_DISPLAY_BLOCK: u8 = 2;
pub const TAFFY_DISPLAY_FLOW_ROOT: u8 = 3;
pub const TAFFY_DISPLAY_GRID: u8 = 4;

// taffy_position_type_t
pub const TAFFY_POSITION_RELATIVE: u8 = 0;
pub const TAFFY_POSITION_ABSOLUTE: u8 = 1;

// taffy_direction_t
pub const TAFFY_DIRECTION_LTR: u8 = 0;
pub const TAFFY_DIRECTION_RTL: u8 = 1;

// taffy_flex_direction_t
pub const TAFFY_FLEX_DIRECTION_ROW: u8 = 0;
pub const TAFFY_FLEX_DIRECTION_COLUMN: u8 = 1;
pub const TAFFY_FLEX_DIRECTION_ROW_REVERSE: u8 = 2;
pub const TAFFY_FLEX_DIRECTION_COLUMN_REVERSE: u8 = 3;

// taffy_flex_wrap_t
pub const TAFFY_FLEX_WRAP_NO_WRAP: u8 = 0;
pub const TAFFY_FLEX_WRAP_WRAP: u8 = 1;
pub const TAFFY_FLEX_WRAP_WRAP_REVERSE: u8 = 2;
pub const TAFFY_FLEX_WRAP_BALANCE: u8 = 3;
pub const TAFFY_FLEX_WRAP_BALANCE_REVERSE: u8 = 4;

// taffy_overflow_t
pub const TAFFY_OVERFLOW_VISIBLE: u8 = 0;
pub const TAFFY_OVERFLOW_CLIP: u8 = 1;
pub const TAFFY_OVERFLOW_HIDDEN: u8 = 2;
pub const TAFFY_OVERFLOW_SCROLL: u8 = 3;

// taffy_box_sizing_t
pub const TAFFY_BOX_SIZING_BORDER_BOX: u8 = 0;
pub const TAFFY_BOX_SIZING_CONTENT_BOX: u8 = 1;

// taffy_dim_type_t
pub const TAFFY_DIM_AUTO: u8 = 0;
pub const TAFFY_DIM_LENGTH: u8 = 1;
pub const TAFFY_DIM_PERCENT: u8 = 2;
pub const TAFFY_DIM_MIN_CONTENT: u8 = 3;
pub const TAFFY_DIM_MAX_CONTENT: u8 = 4;
pub const TAFFY_DIM_FIT_CONTENT_LENGTH: u8 = 5;
pub const TAFFY_DIM_FIT_CONTENT_PERCENT: u8 = 6;
pub const TAFFY_DIM_STRETCH: u8 = 7;
pub const TAFFY_DIM_FIT_CONTENT: u8 = 8;

// taffy_align_items_t
pub const TAFFY_ALIGN_AUTO: u8 = 0;
pub const TAFFY_ALIGN_START: u8 = 1;
pub const TAFFY_ALIGN_END: u8 = 2;
pub const TAFFY_ALIGN_FLEX_START: u8 = 3;
pub const TAFFY_ALIGN_FLEX_END: u8 = 4;
pub const TAFFY_ALIGN_SELF_START: u8 = 5;
pub const TAFFY_ALIGN_SELF_END: u8 = 6;
pub const TAFFY_ALIGN_CENTER: u8 = 7;
pub const TAFFY_ALIGN_BASELINE: u8 = 8;
pub const TAFFY_ALIGN_STRETCH: u8 = 9;
pub const TAFFY_ALIGN_SAFE_START: u8 = 10;
pub const TAFFY_ALIGN_SAFE_END: u8 = 11;
pub const TAFFY_ALIGN_SAFE_FLEX_START: u8 = 12;
pub const TAFFY_ALIGN_SAFE_FLEX_END: u8 = 13;
pub const TAFFY_ALIGN_SAFE_SELF_START: u8 = 14;
pub const TAFFY_ALIGN_SAFE_SELF_END: u8 = 15;
pub const TAFFY_ALIGN_SAFE_CENTER: u8 = 16;

// taffy_align_content_t
pub const TAFFY_CONTENT_ALIGN_AUTO: u8 = 0;
pub const TAFFY_CONTENT_ALIGN_START: u8 = 1;
pub const TAFFY_CONTENT_ALIGN_END: u8 = 2;
pub const TAFFY_CONTENT_ALIGN_FLEX_START: u8 = 3;
pub const TAFFY_CONTENT_ALIGN_FLEX_END: u8 = 4;
pub const TAFFY_CONTENT_ALIGN_CENTER: u8 = 5;
pub const TAFFY_CONTENT_ALIGN_STRETCH: u8 = 6;
pub const TAFFY_CONTENT_ALIGN_SPACE_BETWEEN: u8 = 7;
pub const TAFFY_CONTENT_ALIGN_SPACE_EVENLY: u8 = 8;
pub const TAFFY_CONTENT_ALIGN_SPACE_AROUND: u8 = 9;
pub const TAFFY_CONTENT_ALIGN_SAFE_START: u8 = 10;
pub const TAFFY_CONTENT_ALIGN_SAFE_END: u8 = 11;
pub const TAFFY_CONTENT_ALIGN_SAFE_FLEX_START: u8 = 12;
pub const TAFFY_CONTENT_ALIGN_SAFE_FLEX_END: u8 = 13;
pub const TAFFY_CONTENT_ALIGN_SAFE_CENTER: u8 = 14;

// taffy_available_space_type_t
pub const TAFFY_AVAILABLE_DEFINITE: u8 = 0;
pub const TAFFY_AVAILABLE_MIN_CONTENT: u8 = 1;
pub const TAFFY_AVAILABLE_MAX_CONTENT: u8 = 2;

// ---------------------------------------------------------------------------
// ABI structs
// ---------------------------------------------------------------------------

/// C ABI `taffy_dim_t`: a tagged length / percentage.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTaffyDim {
    /// One of the `TAFFY_DIM_*` constants.
    pub dim_type: u8,
    pub value: f32,
}

/// C ABI `taffy_available_space_t`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTaffyAvailableSpace {
    /// One of the `TAFFY_AVAILABLE_*` constants.
    pub space_type: u8,
    pub value: f32,
}

/// C ABI `taffy_style_t`. Field order mirrors `include/taffy.h` exactly.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTaffyStyle {
    // Core
    pub display: u8,
    pub box_sizing: u8,
    pub position_type: u8,
    pub direction: u8,

    // Flexbox container
    pub flex_direction: u8,
    pub flex_wrap: u8,
    pub overflow_x: u8,
    pub overflow_y: u8,

    // Alignment
    pub align_items: u8,
    pub align_self: u8,
    pub align_content: u8,
    pub justify_content: u8,
    pub justify_items: u8,
    pub justify_self: u8,

    pub flex_line_count: u16,

    pub scrollbar_width: f32,

    // Flexbox item
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub aspect_ratio: f32,

    // Rect edges are [left, right, top, bottom]; sizes/gaps are [width, height].
    pub inset: [CTaffyDim; 4],
    pub margin: [CTaffyDim; 4],
    pub padding: [CTaffyDim; 4],
    pub border: [CTaffyDim; 4],
    pub size: [CTaffyDim; 2],
    pub min_size: [CTaffyDim; 2],
    pub max_size: [CTaffyDim; 2],
    pub flex_basis: CTaffyDim,
    pub gap: [CTaffyDim; 2],
}

/// C ABI `taffy_layout_t`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTaffyLayout {
    pub order: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub content_width: f32,
    pub content_height: f32,
    pub border: [f32; 4],
    pub padding: [f32; 4],
    pub margin: [f32; 4],
    pub scroll_width: f32,
    pub scroll_height: f32,
}

/// C ABI `taffy_measure_callback_t`.
pub type CTaffyMeasureCallback = extern "C" fn(
    node: u64,
    known_dimensions: *const f32,
    available_space: *const CTaffyAvailableSpace,
    node_context: *mut c_void,
    measure_context: *mut c_void,
    out_size: *mut f32,
);
