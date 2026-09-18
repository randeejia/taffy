//! Conversions between the C ABI types (see `ctypes.rs`) and taffy's own
//! types.
//!
//! Invalid enum discriminators are reported as `TAFFY_ERR_INVALID_ARGUMENT`
//! rather than causing a panic across the FFI boundary.

use taffy::geometry::{Point, Rect, Size};
use taffy::style::{
    AlignContent, AlignItems, AvailableSpace, BoxSizing, Dimension, Direction, Display, FlexDirection, FlexWrap,
    LengthPercentage, LengthPercentageAuto, Overflow, Position, Style,
};
use taffy::tree::Layout;

use crate::ctypes::*;

/// Error produced while converting an ABI style into a taffy [`Style`].
pub struct ConvertError(pub u32);

// ---------------------------------------------------------------------------
// Dimensions
// ---------------------------------------------------------------------------

/// ABI dim -> [`Dimension`] (used for `size` / `flex_basis`; all intrinsic
/// sizing keywords are supported).
pub(crate) fn dim_to_dimension(d: CTaffyDim) -> Dimension {
    match d.dim_type {
        TAFFY_DIM_LENGTH => Dimension::length(d.value),
        TAFFY_DIM_PERCENT => Dimension::percent(d.value),
        TAFFY_DIM_MIN_CONTENT => Dimension::min_content(),
        TAFFY_DIM_MAX_CONTENT => Dimension::max_content(),
        TAFFY_DIM_FIT_CONTENT_LENGTH => Dimension::fit_content_px(d.value),
        TAFFY_DIM_FIT_CONTENT_PERCENT => Dimension::fit_content_percent(d.value),
        TAFFY_DIM_FIT_CONTENT => Dimension::fit_content(),
        TAFFY_DIM_STRETCH => Dimension::stretch(),
        // TAFFY_DIM_AUTO and any unknown tag
        _ => Dimension::auto(),
    }
}

/// ABI dim -> [`LengthPercentageAuto`] (used for `inset` / `margin` /
/// `min_size` / `max_size`). Intrinsic sizing keywords other than auto are
/// not legal here and collapse to `auto`.
pub(crate) fn dim_to_length_percentage_auto(d: CTaffyDim) -> LengthPercentageAuto {
    match d.dim_type {
        TAFFY_DIM_LENGTH => LengthPercentageAuto::length(d.value),
        TAFFY_DIM_PERCENT => LengthPercentageAuto::percent(d.value),
        _ => LengthPercentageAuto::auto(),
    }
}

/// ABI dim -> [`LengthPercentage`] (used for `padding` / `border` / `gap`).
/// Anything that is not a length/percentage collapses to zero (these
/// properties do not accept `auto` or sizing keywords in CSS).
pub(crate) fn dim_to_length_percentage(d: CTaffyDim) -> LengthPercentage {
    match d.dim_type {
        TAFFY_DIM_LENGTH => LengthPercentage::length(d.value),
        TAFFY_DIM_PERCENT => LengthPercentage::percent(d.value),
        _ => LengthPercentage::length(0.0),
    }
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

fn map_display(v: u8) -> Result<Display, ConvertError> {
    match v {
        TAFFY_DISPLAY_NONE => Ok(Display::None),
        TAFFY_DISPLAY_FLEX => Ok(Display::Flex),
        TAFFY_DISPLAY_BLOCK => Ok(Display::Block),
        TAFFY_DISPLAY_FLOW_ROOT => Ok(Display::FlowRoot),
        TAFFY_DISPLAY_GRID => Ok(Display::Grid),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

fn map_position(v: u8) -> Result<Position, ConvertError> {
    match v {
        TAFFY_POSITION_RELATIVE => Ok(Position::Relative),
        TAFFY_POSITION_ABSOLUTE => Ok(Position::Absolute),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

fn map_direction(v: u8) -> Result<Direction, ConvertError> {
    match v {
        TAFFY_DIRECTION_LTR => Ok(Direction::Ltr),
        TAFFY_DIRECTION_RTL => Ok(Direction::Rtl),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

fn map_flex_direction(v: u8) -> Result<FlexDirection, ConvertError> {
    match v {
        TAFFY_FLEX_DIRECTION_ROW => Ok(FlexDirection::Row),
        TAFFY_FLEX_DIRECTION_COLUMN => Ok(FlexDirection::Column),
        TAFFY_FLEX_DIRECTION_ROW_REVERSE => Ok(FlexDirection::RowReverse),
        TAFFY_FLEX_DIRECTION_COLUMN_REVERSE => Ok(FlexDirection::ColumnReverse),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

fn map_flex_wrap(v: u8) -> Result<FlexWrap, ConvertError> {
    match v {
        TAFFY_FLEX_WRAP_NO_WRAP => Ok(FlexWrap::NoWrap),
        TAFFY_FLEX_WRAP_WRAP => Ok(FlexWrap::Wrap),
        TAFFY_FLEX_WRAP_WRAP_REVERSE => Ok(FlexWrap::WrapReverse),
        TAFFY_FLEX_WRAP_BALANCE => Ok(FlexWrap::Balance),
        TAFFY_FLEX_WRAP_BALANCE_REVERSE => Ok(FlexWrap::BalanceReverse),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

fn map_overflow(v: u8) -> Result<Overflow, ConvertError> {
    match v {
        TAFFY_OVERFLOW_VISIBLE => Ok(Overflow::Visible),
        TAFFY_OVERFLOW_CLIP => Ok(Overflow::Clip),
        TAFFY_OVERFLOW_HIDDEN => Ok(Overflow::Hidden),
        TAFFY_OVERFLOW_SCROLL => Ok(Overflow::Scroll),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

fn map_box_sizing(v: u8) -> Result<BoxSizing, ConvertError> {
    match v {
        TAFFY_BOX_SIZING_BORDER_BOX => Ok(BoxSizing::BorderBox),
        TAFFY_BOX_SIZING_CONTENT_BOX => Ok(BoxSizing::ContentBox),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

/// align-items / align-self / justify-items / justify-self. `AUTO` maps to
/// `None` (CSS "not set"; for align-self this is the `auto` keyword).
fn map_align_items(v: u8) -> Result<Option<AlignItems>, ConvertError> {
    match v {
        TAFFY_ALIGN_AUTO => Ok(None),
        TAFFY_ALIGN_START => Ok(Some(AlignItems::START)),
        TAFFY_ALIGN_END => Ok(Some(AlignItems::END)),
        TAFFY_ALIGN_FLEX_START => Ok(Some(AlignItems::FLEX_START)),
        TAFFY_ALIGN_FLEX_END => Ok(Some(AlignItems::FLEX_END)),
        TAFFY_ALIGN_SELF_START => Ok(Some(AlignItems::SELF_START)),
        TAFFY_ALIGN_SELF_END => Ok(Some(AlignItems::SELF_END)),
        TAFFY_ALIGN_CENTER => Ok(Some(AlignItems::CENTER)),
        TAFFY_ALIGN_BASELINE => Ok(Some(AlignItems::BASELINE)),
        TAFFY_ALIGN_STRETCH => Ok(Some(AlignItems::STRETCH)),
        TAFFY_ALIGN_SAFE_START => Ok(Some(AlignItems::SAFE_START)),
        TAFFY_ALIGN_SAFE_END => Ok(Some(AlignItems::SAFE_END)),
        TAFFY_ALIGN_SAFE_FLEX_START => Ok(Some(AlignItems::SAFE_FLEX_START)),
        TAFFY_ALIGN_SAFE_FLEX_END => Ok(Some(AlignItems::SAFE_FLEX_END)),
        TAFFY_ALIGN_SAFE_SELF_START => Ok(Some(AlignItems::SAFE_SELF_START)),
        TAFFY_ALIGN_SAFE_SELF_END => Ok(Some(AlignItems::SAFE_SELF_END)),
        TAFFY_ALIGN_SAFE_CENTER => Ok(Some(AlignItems::SAFE_CENTER)),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

/// align-content / justify-content. `AUTO` maps to `None`.
fn map_align_content(v: u8) -> Result<Option<AlignContent>, ConvertError> {
    match v {
        TAFFY_CONTENT_ALIGN_AUTO => Ok(None),
        TAFFY_CONTENT_ALIGN_START => Ok(Some(AlignContent::START)),
        TAFFY_CONTENT_ALIGN_END => Ok(Some(AlignContent::END)),
        TAFFY_CONTENT_ALIGN_FLEX_START => Ok(Some(AlignContent::FLEX_START)),
        TAFFY_CONTENT_ALIGN_FLEX_END => Ok(Some(AlignContent::FLEX_END)),
        TAFFY_CONTENT_ALIGN_CENTER => Ok(Some(AlignContent::CENTER)),
        TAFFY_CONTENT_ALIGN_STRETCH => Ok(Some(AlignContent::STRETCH)),
        TAFFY_CONTENT_ALIGN_SPACE_BETWEEN => Ok(Some(AlignContent::SPACE_BETWEEN)),
        TAFFY_CONTENT_ALIGN_SPACE_EVENLY => Ok(Some(AlignContent::SPACE_EVENLY)),
        TAFFY_CONTENT_ALIGN_SPACE_AROUND => Ok(Some(AlignContent::SPACE_AROUND)),
        TAFFY_CONTENT_ALIGN_SAFE_START => Ok(Some(AlignContent::SAFE_START)),
        TAFFY_CONTENT_ALIGN_SAFE_END => Ok(Some(AlignContent::SAFE_END)),
        TAFFY_CONTENT_ALIGN_SAFE_FLEX_START => Ok(Some(AlignContent::SAFE_FLEX_START)),
        TAFFY_CONTENT_ALIGN_SAFE_FLEX_END => Ok(Some(AlignContent::SAFE_FLEX_END)),
        TAFFY_CONTENT_ALIGN_SAFE_CENTER => Ok(Some(AlignContent::SAFE_CENTER)),
        _ => Err(ConvertError(TAFFY_ERR_INVALID_ARGUMENT)),
    }
}

// ---------------------------------------------------------------------------
// Available space
// ---------------------------------------------------------------------------

pub(crate) fn map_available_space(s: CTaffyAvailableSpace) -> AvailableSpace {
    match s.space_type {
        TAFFY_AVAILABLE_MIN_CONTENT => AvailableSpace::MinContent,
        TAFFY_AVAILABLE_MAX_CONTENT => AvailableSpace::MaxContent,
        // Definite and any unknown value
        _ => AvailableSpace::Definite(s.value),
    }
}

pub(crate) fn available_space_to_c(s: AvailableSpace) -> CTaffyAvailableSpace {
    match s {
        AvailableSpace::Definite(v) => CTaffyAvailableSpace { space_type: TAFFY_AVAILABLE_DEFINITE, value: v },
        AvailableSpace::MinContent => CTaffyAvailableSpace { space_type: TAFFY_AVAILABLE_MIN_CONTENT, value: 0.0 },
        AvailableSpace::MaxContent => CTaffyAvailableSpace { space_type: TAFFY_AVAILABLE_MAX_CONTENT, value: 0.0 },
    }
}

// ---------------------------------------------------------------------------
// Style / Layout
// ---------------------------------------------------------------------------

/// Convert an ABI style into taffy's [`Style`].
#[allow(clippy::field_reassign_with_default)] // incremental build with `?` validation
pub(crate) fn style_from_c(c: &CTaffyStyle) -> Result<Style, ConvertError> {
    // Grid template / placement fields and other advanced properties keep
    // their taffy defaults; they can be exposed in a later (ABI-additive)
    // revision.
    let mut style = Style::default();

    style.display = map_display(c.display)?;
    style.box_sizing = map_box_sizing(c.box_sizing)?;
    style.position = map_position(c.position_type)?;
    style.direction = map_direction(c.direction)?;

    style.flex_direction = map_flex_direction(c.flex_direction)?;
    style.flex_wrap = map_flex_wrap(c.flex_wrap)?;
    style.overflow = Point { x: map_overflow(c.overflow_x)?, y: map_overflow(c.overflow_y)? };
    style.scrollbar_width = c.scrollbar_width;

    style.align_items = map_align_items(c.align_items)?;
    style.align_self = map_align_items(c.align_self)?;
    style.justify_items = map_align_items(c.justify_items)?;
    style.justify_self = map_align_items(c.justify_self)?;
    style.align_content = map_align_content(c.align_content)?;
    style.justify_content = map_align_content(c.justify_content)?;

    style.flex_line_count = c.flex_line_count.max(1);

    style.flex_grow = c.flex_grow;
    style.flex_shrink = c.flex_shrink;
    style.aspect_ratio = if c.aspect_ratio.is_nan() { None } else { Some(c.aspect_ratio) };

    // Edge order on both sides is [left, right, top, bottom].
    style.inset = Rect {
        left: dim_to_length_percentage_auto(c.inset[0]),
        right: dim_to_length_percentage_auto(c.inset[1]),
        top: dim_to_length_percentage_auto(c.inset[2]),
        bottom: dim_to_length_percentage_auto(c.inset[3]),
    };
    style.margin = Rect {
        left: dim_to_length_percentage_auto(c.margin[0]),
        right: dim_to_length_percentage_auto(c.margin[1]),
        top: dim_to_length_percentage_auto(c.margin[2]),
        bottom: dim_to_length_percentage_auto(c.margin[3]),
    };
    style.padding = Rect {
        left: dim_to_length_percentage(c.padding[0]),
        right: dim_to_length_percentage(c.padding[1]),
        top: dim_to_length_percentage(c.padding[2]),
        bottom: dim_to_length_percentage(c.padding[3]),
    };
    style.border = Rect {
        left: dim_to_length_percentage(c.border[0]),
        right: dim_to_length_percentage(c.border[1]),
        top: dim_to_length_percentage(c.border[2]),
        bottom: dim_to_length_percentage(c.border[3]),
    };

    style.size = Size { width: dim_to_dimension(c.size[0]), height: dim_to_dimension(c.size[1]) };
    style.min_size = Size {
        width: dim_to_length_percentage_auto(c.min_size[0]),
        height: dim_to_length_percentage_auto(c.min_size[1]),
    };
    style.max_size = Size {
        width: dim_to_length_percentage_auto(c.max_size[0]),
        height: dim_to_length_percentage_auto(c.max_size[1]),
    };
    style.flex_basis = dim_to_dimension(c.flex_basis);
    style.gap = Size { width: dim_to_length_percentage(c.gap[0]), height: dim_to_length_percentage(c.gap[1]) };

    Ok(style)
}

/// Produce the ABI encoding of taffy's default [`Style`]. This is the single
/// source of truth for `taffy_style_default()` and guarantees the C defaults
/// never drift from the Rust defaults.
pub(crate) fn default_style_to_c() -> CTaffyStyle {
    style_to_c(&Style::default())
}

/// Convert a taffy [`Style`] to its ABI representation.
pub(crate) fn style_to_c(s: &Style) -> CTaffyStyle {
    let dim_from_dimension = |d: Dimension| -> CTaffyDim {
        use taffy::style::ExpandedDimension;
        match d.expand() {
            ExpandedDimension::Length(v) => CTaffyDim { dim_type: TAFFY_DIM_LENGTH, value: v },
            ExpandedDimension::Percent(v) => CTaffyDim { dim_type: TAFFY_DIM_PERCENT, value: v },
            ExpandedDimension::MinContent => CTaffyDim { dim_type: TAFFY_DIM_MIN_CONTENT, value: 0.0 },
            ExpandedDimension::MaxContent => CTaffyDim { dim_type: TAFFY_DIM_MAX_CONTENT, value: 0.0 },
            ExpandedDimension::FitContentPx(v) => CTaffyDim { dim_type: TAFFY_DIM_FIT_CONTENT_LENGTH, value: v },
            ExpandedDimension::FitContentPercent(v) => CTaffyDim { dim_type: TAFFY_DIM_FIT_CONTENT_PERCENT, value: v },
            ExpandedDimension::FitContent => CTaffyDim { dim_type: TAFFY_DIM_FIT_CONTENT, value: 0.0 },
            ExpandedDimension::Stretch => CTaffyDim { dim_type: TAFFY_DIM_STRETCH, value: 0.0 },
            ExpandedDimension::Auto | ExpandedDimension::Content | ExpandedDimension::Calc(_) => {
                CTaffyDim { dim_type: TAFFY_DIM_AUTO, value: 0.0 }
            }
        }
    };
    let dim_from_lpa = |d: LengthPercentageAuto| -> CTaffyDim {
        use taffy::style::ExpandedLengthPercentageAuto;
        match d.expand() {
            ExpandedLengthPercentageAuto::Length(v) => CTaffyDim { dim_type: TAFFY_DIM_LENGTH, value: v },
            ExpandedLengthPercentageAuto::Percent(v) => CTaffyDim { dim_type: TAFFY_DIM_PERCENT, value: v },
            ExpandedLengthPercentageAuto::Auto | ExpandedLengthPercentageAuto::Calc(_) => {
                CTaffyDim { dim_type: TAFFY_DIM_AUTO, value: 0.0 }
            }
        }
    };
    let dim_from_lp = |d: LengthPercentage| -> CTaffyDim {
        use taffy::style::ExpandedLengthPercentage;
        match d.expand() {
            ExpandedLengthPercentage::Length(v) => CTaffyDim { dim_type: TAFFY_DIM_LENGTH, value: v },
            ExpandedLengthPercentage::Percent(v) => CTaffyDim { dim_type: TAFFY_DIM_PERCENT, value: v },
            ExpandedLengthPercentage::Calc(_) => CTaffyDim { dim_type: TAFFY_DIM_AUTO, value: 0.0 },
        }
    };

    let align_items_to_c = |v: Option<AlignItems>| -> u8 {
        match v {
            None => TAFFY_ALIGN_AUTO,
            Some(a) => match (a.keyword, a.safety) {
                (taffy::style::AlignItemsKeyword::Start, taffy::style::AlignmentSafety::Safe) => TAFFY_ALIGN_SAFE_START,
                (taffy::style::AlignItemsKeyword::End, taffy::style::AlignmentSafety::Safe) => TAFFY_ALIGN_SAFE_END,
                (taffy::style::AlignItemsKeyword::FlexStart, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_ALIGN_SAFE_FLEX_START
                }
                (taffy::style::AlignItemsKeyword::FlexEnd, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_ALIGN_SAFE_FLEX_END
                }
                (taffy::style::AlignItemsKeyword::SelfStart, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_ALIGN_SAFE_SELF_START
                }
                (taffy::style::AlignItemsKeyword::SelfEnd, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_ALIGN_SAFE_SELF_END
                }
                (taffy::style::AlignItemsKeyword::Center, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_ALIGN_SAFE_CENTER
                }
                (taffy::style::AlignItemsKeyword::Start, _) => TAFFY_ALIGN_START,
                (taffy::style::AlignItemsKeyword::End, _) => TAFFY_ALIGN_END,
                (taffy::style::AlignItemsKeyword::FlexStart, _) => TAFFY_ALIGN_FLEX_START,
                (taffy::style::AlignItemsKeyword::FlexEnd, _) => TAFFY_ALIGN_FLEX_END,
                (taffy::style::AlignItemsKeyword::SelfStart, _) => TAFFY_ALIGN_SELF_START,
                (taffy::style::AlignItemsKeyword::SelfEnd, _) => TAFFY_ALIGN_SELF_END,
                (taffy::style::AlignItemsKeyword::Center, _) => TAFFY_ALIGN_CENTER,
                (taffy::style::AlignItemsKeyword::Baseline, _) => TAFFY_ALIGN_BASELINE,
                (taffy::style::AlignItemsKeyword::Stretch, _) => TAFFY_ALIGN_STRETCH,
            },
        }
    };

    let align_content_to_c = |v: Option<AlignContent>| -> u8 {
        match v {
            None => TAFFY_CONTENT_ALIGN_AUTO,
            Some(a) => match (a.keyword, a.safety) {
                (taffy::style::AlignContentKeyword::Start, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_CONTENT_ALIGN_SAFE_START
                }
                (taffy::style::AlignContentKeyword::End, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_CONTENT_ALIGN_SAFE_END
                }
                (taffy::style::AlignContentKeyword::FlexStart, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_CONTENT_ALIGN_SAFE_FLEX_START
                }
                (taffy::style::AlignContentKeyword::FlexEnd, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_CONTENT_ALIGN_SAFE_FLEX_END
                }
                (taffy::style::AlignContentKeyword::Center, taffy::style::AlignmentSafety::Safe) => {
                    TAFFY_CONTENT_ALIGN_SAFE_CENTER
                }
                (taffy::style::AlignContentKeyword::Start, _) => TAFFY_CONTENT_ALIGN_START,
                (taffy::style::AlignContentKeyword::End, _) => TAFFY_CONTENT_ALIGN_END,
                (taffy::style::AlignContentKeyword::FlexStart, _) => TAFFY_CONTENT_ALIGN_FLEX_START,
                (taffy::style::AlignContentKeyword::FlexEnd, _) => TAFFY_CONTENT_ALIGN_FLEX_END,
                (taffy::style::AlignContentKeyword::Center, _) => TAFFY_CONTENT_ALIGN_CENTER,
                (taffy::style::AlignContentKeyword::Stretch, _) => TAFFY_CONTENT_ALIGN_STRETCH,
                (taffy::style::AlignContentKeyword::SpaceBetween, _) => TAFFY_CONTENT_ALIGN_SPACE_BETWEEN,
                (taffy::style::AlignContentKeyword::SpaceEvenly, _) => TAFFY_CONTENT_ALIGN_SPACE_EVENLY,
                (taffy::style::AlignContentKeyword::SpaceAround, _) => TAFFY_CONTENT_ALIGN_SPACE_AROUND,
            },
        }
    };

    let display_to_c = |d: Display| -> u8 {
        match d {
            Display::None => TAFFY_DISPLAY_NONE,
            Display::Flex => TAFFY_DISPLAY_FLEX,
            Display::Block => TAFFY_DISPLAY_BLOCK,
            Display::FlowRoot => TAFFY_DISPLAY_FLOW_ROOT,
            Display::Grid => TAFFY_DISPLAY_GRID,
        }
    };

    CTaffyStyle {
        display: display_to_c(s.display),
        box_sizing: match s.box_sizing {
            BoxSizing::BorderBox => TAFFY_BOX_SIZING_BORDER_BOX,
            BoxSizing::ContentBox => TAFFY_BOX_SIZING_CONTENT_BOX,
        },
        position_type: match s.position {
            Position::Relative => TAFFY_POSITION_RELATIVE,
            Position::Absolute => TAFFY_POSITION_ABSOLUTE,
        },
        direction: match s.direction {
            Direction::Ltr => TAFFY_DIRECTION_LTR,
            Direction::Rtl => TAFFY_DIRECTION_RTL,
        },
        flex_direction: match s.flex_direction {
            FlexDirection::Row => TAFFY_FLEX_DIRECTION_ROW,
            FlexDirection::Column => TAFFY_FLEX_DIRECTION_COLUMN,
            FlexDirection::RowReverse => TAFFY_FLEX_DIRECTION_ROW_REVERSE,
            FlexDirection::ColumnReverse => TAFFY_FLEX_DIRECTION_COLUMN_REVERSE,
        },
        flex_wrap: match s.flex_wrap {
            FlexWrap::NoWrap => TAFFY_FLEX_WRAP_NO_WRAP,
            FlexWrap::Wrap => TAFFY_FLEX_WRAP_WRAP,
            FlexWrap::WrapReverse => TAFFY_FLEX_WRAP_WRAP_REVERSE,
            FlexWrap::Balance => TAFFY_FLEX_WRAP_BALANCE,
            FlexWrap::BalanceReverse => TAFFY_FLEX_WRAP_BALANCE_REVERSE,
        },
        overflow_x: match s.overflow.x {
            Overflow::Visible => TAFFY_OVERFLOW_VISIBLE,
            Overflow::Clip => TAFFY_OVERFLOW_CLIP,
            Overflow::Hidden => TAFFY_OVERFLOW_HIDDEN,
            Overflow::Scroll => TAFFY_OVERFLOW_SCROLL,
        },
        overflow_y: match s.overflow.y {
            Overflow::Visible => TAFFY_OVERFLOW_VISIBLE,
            Overflow::Clip => TAFFY_OVERFLOW_CLIP,
            Overflow::Hidden => TAFFY_OVERFLOW_HIDDEN,
            Overflow::Scroll => TAFFY_OVERFLOW_SCROLL,
        },
        align_items: align_items_to_c(s.align_items),
        align_self: align_items_to_c(s.align_self),
        align_content: align_content_to_c(s.align_content),
        justify_content: align_content_to_c(s.justify_content),
        justify_items: align_items_to_c(s.justify_items),
        justify_self: align_items_to_c(s.justify_self),
        flex_line_count: s.flex_line_count,
        scrollbar_width: s.scrollbar_width,
        flex_grow: s.flex_grow,
        flex_shrink: s.flex_shrink,
        aspect_ratio: s.aspect_ratio.unwrap_or(f32::NAN),
        inset: [
            dim_from_lpa(s.inset.left),
            dim_from_lpa(s.inset.right),
            dim_from_lpa(s.inset.top),
            dim_from_lpa(s.inset.bottom),
        ],
        margin: [
            dim_from_lpa(s.margin.left),
            dim_from_lpa(s.margin.right),
            dim_from_lpa(s.margin.top),
            dim_from_lpa(s.margin.bottom),
        ],
        padding: [
            dim_from_lp(s.padding.left),
            dim_from_lp(s.padding.right),
            dim_from_lp(s.padding.top),
            dim_from_lp(s.padding.bottom),
        ],
        border: [
            dim_from_lp(s.border.left),
            dim_from_lp(s.border.right),
            dim_from_lp(s.border.top),
            dim_from_lp(s.border.bottom),
        ],
        size: [dim_from_dimension(s.size.width), dim_from_dimension(s.size.height)],
        min_size: [dim_from_lpa(s.min_size.width), dim_from_lpa(s.min_size.height)],
        max_size: [dim_from_lpa(s.max_size.width), dim_from_lpa(s.max_size.height)],
        flex_basis: dim_from_dimension(s.flex_basis),
        gap: [dim_from_lp(s.gap.width), dim_from_lp(s.gap.height)],
    }
}

/// Convert a computed [`Layout`] to its ABI representation.
pub(crate) fn layout_to_c(l: &Layout) -> CTaffyLayout {
    CTaffyLayout {
        order: l.order,
        x: l.location.x,
        y: l.location.y,
        width: l.size.width,
        height: l.size.height,
        content_width: l.content_box_width(),
        content_height: l.content_box_height(),
        border: [l.border.left, l.border.right, l.border.top, l.border.bottom],
        padding: [l.padding.left, l.padding.right, l.padding.top, l.padding.bottom],
        margin: [l.margin.left, l.margin.right, l.margin.top, l.margin.bottom],
        scroll_width: l.scroll_width(),
        scroll_height: l.scroll_height(),
    }
}
