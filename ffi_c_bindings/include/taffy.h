/*
 * taffy.h - C FFI bindings for the Taffy layout engine.
 *
 * Taffy implements the CSS Flexbox, CSS Grid and Block layout algorithms.
 * This header exposes a stable C ABI on top of the Rust implementation so that
 * it can be consumed from C++, Kotlin/JNI (Android NDK), ArkTS/N-API
 * (OpenHarmony) and Swift (via a bridging header / module map).
 *
 * Version: 0.1.0 (built against taffy 0.14.x)
 *
 * ---------------------------------------------------------------------------
 * DESIGN NOTES
 * ---------------------------------------------------------------------------
 *
 * 1. Handles & ids
 *    - A layout tree is owned by the library and is referenced through an
 *      opaque `taffy_tree_t *` pointer.
 *    - Nodes are referenced by a 64 bit `taffy_node_id` value (analogous to
 *      the `jlong` node pointers used by the old stretch bindings). Ids are
 *      owned by the tree; call `taffy_node_remove()` (or
 *      `taffy_tree_free()` / `taffy_tree_clear()`) to release them.
 *
 * 2. Ownership
 *    - `taffy_tree_t *` returned by `taffy_tree_new*()` must be released with
 *      `taffy_tree_free()`.
 *    - All style/input structs are copied by value during the call; the
 *      library never retains pointers to them.
 *    - Per-node user context (`void *`, typically the native peer of a text /
 *      image widget) is stored opaquely and handed back to the measure
 *      callback. The library never frees it.
 *
 * 3. Measure functions
 *    Unlike stretch (which attached a measure function to every leaf node),
 *    taffy takes a SINGLE global measure callback at compute time. The
 *    callback is invoked for every childless node and receives the per-node
 *    context, allowing the host platform to dispatch to its own text / image
 *    measurement code.
 *
 * 4. Units & conventions
 *    - All lengths are plain `float` values in abstract units (usually
 *      logical pixels).
 *    - Percentages use the CSS convention: a fraction in the range [0.0, 1.0],
 *      NOT [0.0, 100.0].
 *    - "unknown" float values (e.g. an unknown known-dimension passed to a
 *      measure callback) are signalled with NaN.
 *    - Rect arrays are physical / absolute in order
 *      `[left, right, top, bottom]`. Size arrays are `[width, height]`.
 *      Logical start/end (as used by stretch) are resolved by taffy from the
 *      node's `direction` (LTR/RTL).
 *
 * 5. Threading
 *    A `taffy_tree_t` instance is NOT thread safe. External synchronisation
 *    is required if it is accessed from multiple threads.
 *
 * 6. Error handling
 *    Every fallible call returns a `taffy_status_t` (TAFFY_OK == 0). Invalid
 *    nodes / indices never cause undefined behaviour: panics originating in
 *    the Rust core are caught at the FFI boundary and reported as
 *    TAFFY_ERR_UNKNOWN.
 */

#ifndef TAFFY_H
#define TAFFY_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ===========================================================================
 * Status codes
 * ========================================================================= */

typedef enum taffy_status_t {
    TAFFY_OK = 0,
    /* A required output / object pointer was NULL */
    TAFFY_ERR_NULL_POINTER = 1,
    /* The referenced node does not exist in the tree */
    TAFFY_ERR_INVALID_NODE = 2,
    /* A child index was out of range */
    TAFFY_ERR_INDEX_OUT_OF_BOUNDS = 3,
    /* The supplied buffer was too small; the required size is written out */
    TAFFY_ERR_BUFFER_TOO_SMALL = 4,
    /* An enum / argument value was not recognised */
    TAFFY_ERR_INVALID_ARGUMENT = 5,
    /* taffy_node_parent(): the node currently has no parent */
    TAFFY_ERR_NO_PARENT = 6,
    /* Anything else (including a caught Rust panic) */
    TAFFY_ERR_UNKNOWN = 99,
} taffy_status_t;

/* ===========================================================================
 * Enumerations
 *
 * Numeric values are part of the ABI and MUST stay stable. New members may be
 * appended, but existing values must not be renumbered.
 * ========================================================================= */

/* CSS `display` (which layout algorithm lays out the children) */
typedef enum taffy_display_t {
    TAFFY_DISPLAY_NONE = 0,
    TAFFY_DISPLAY_FLEX = 1,
    TAFFY_DISPLAY_BLOCK = 2,
    TAFFY_DISPLAY_FLOW_ROOT = 3,
    TAFFY_DISPLAY_GRID = 4,
} taffy_display_t;

/* CSS `position` */
typedef enum taffy_position_type_t {
    TAFFY_POSITION_RELATIVE = 0,
    TAFFY_POSITION_ABSOLUTE = 1,
    TAFFY_POSITION_STATIC = 2,
    TAFFY_POSITION_FIXED = 3,
    TAFFY_POSITION_STICKY = 4,
} taffy_position_type_t;

/* CSS `direction`. Note: modern taffy has no "inherit" mode (unlike stretch). */
typedef enum taffy_direction_t {
    TAFFY_DIRECTION_LTR = 0,
    TAFFY_DIRECTION_RTL = 1,
} taffy_direction_t;

/* CSS `flex-direction` */
typedef enum taffy_flex_direction_t {
    TAFFY_FLEX_DIRECTION_ROW = 0,
    TAFFY_FLEX_DIRECTION_COLUMN = 1,
    TAFFY_FLEX_DIRECTION_ROW_REVERSE = 2,
    TAFFY_FLEX_DIRECTION_COLUMN_REVERSE = 3,
} taffy_flex_direction_t;

/* CSS `flex-wrap` (Balance/BalanceReverse require flexbox level 2 support) */
typedef enum taffy_flex_wrap_t {
    TAFFY_FLEX_WRAP_NO_WRAP = 0,
    TAFFY_FLEX_WRAP_WRAP = 1,
    TAFFY_FLEX_WRAP_WRAP_REVERSE = 2,
    TAFFY_FLEX_WRAP_BALANCE = 3,
    TAFFY_FLEX_WRAP_BALANCE_REVERSE = 4,
} taffy_flex_wrap_t;

/* CSS `overflow` for a single axis */
typedef enum taffy_overflow_t {
    TAFFY_OVERFLOW_VISIBLE = 0,
    TAFFY_OVERFLOW_CLIP = 1,
    TAFFY_OVERFLOW_HIDDEN = 2,
    TAFFY_OVERFLOW_SCROLL = 3,
} taffy_overflow_t;

/* CSS `box-sizing` */
typedef enum taffy_box_sizing_t {
    TAFFY_BOX_SIZING_BORDER_BOX = 0,
    TAFFY_BOX_SIZING_CONTENT_BOX = 1,
} taffy_box_sizing_t;

/*
 * Discriminator for `taffy_dim_t`.
 *
 * The LENGTH / PERCENT / AUTO members map 1:1 onto the stretch binding's
 * Points / Percent / Undefined-or-Auto encoding. The remaining members expose
 * taffy's intrinsic sizing keywords (min-content / max-content / fit-content /
 * stretch).
 *
 * Applicability per style property:
 *   size / flex_basis : LENGTH, PERCENT, AUTO, MIN_CONTENT, MAX_CONTENT,
 *                       FIT_CONTENT, FIT_CONTENT_LENGTH, FIT_CONTENT_PERCENT,
 *                       STRETCH
 *   min_size/max_size, margin, inset : LENGTH, PERCENT, AUTO (other keywords
 *                       are treated as AUTO)
 *   padding / border / gap : LENGTH, PERCENT (AUTO / keywords treated as 0)
 */
typedef enum taffy_dim_type_t {
    TAFFY_DIM_AUTO = 0,
    TAFFY_DIM_LENGTH = 1,
    TAFFY_DIM_PERCENT = 2,
    TAFFY_DIM_MIN_CONTENT = 3,
    TAFFY_DIM_MAX_CONTENT = 4,
    TAFFY_DIM_FIT_CONTENT_LENGTH = 5,
    TAFFY_DIM_FIT_CONTENT_PERCENT = 6,
    TAFFY_DIM_STRETCH = 7,
    TAFFY_DIM_FIT_CONTENT = 8,
} taffy_dim_type_t;

/*
 * Alignment on the cross axis / block axis: align-items, align-self,
 * justify-items, justify-self.
 * TAFFY_ALIGN_AUTO means "not set" (None); for align-self this is CSS `auto`.
 */
typedef enum taffy_align_items_t {
    TAFFY_ALIGN_AUTO = 0,
    TAFFY_ALIGN_START = 1,
    TAFFY_ALIGN_END = 2,
    TAFFY_ALIGN_FLEX_START = 3,
    TAFFY_ALIGN_FLEX_END = 4,
    TAFFY_ALIGN_SELF_START = 5,
    TAFFY_ALIGN_SELF_END = 6,
    TAFFY_ALIGN_CENTER = 7,
    TAFFY_ALIGN_BASELINE = 8,
    TAFFY_ALIGN_STRETCH = 9,
    TAFFY_ALIGN_SAFE_START = 10,
    TAFFY_ALIGN_SAFE_END = 11,
    TAFFY_ALIGN_SAFE_FLEX_START = 12,
    TAFFY_ALIGN_SAFE_FLEX_END = 13,
    TAFFY_ALIGN_SAFE_SELF_START = 14,
    TAFFY_ALIGN_SAFE_SELF_END = 15,
    TAFFY_ALIGN_SAFE_CENTER = 16,
} taffy_align_items_t;

/*
 * Content / main axis distribution: align-content, justify-content.
 * TAFFY_CONTENT_ALIGN_AUTO means "not set" (None).
 */
typedef enum taffy_align_content_t {
    TAFFY_CONTENT_ALIGN_AUTO = 0,
    TAFFY_CONTENT_ALIGN_START = 1,
    TAFFY_CONTENT_ALIGN_END = 2,
    TAFFY_CONTENT_ALIGN_FLEX_START = 3,
    TAFFY_CONTENT_ALIGN_FLEX_END = 4,
    TAFFY_CONTENT_ALIGN_CENTER = 5,
    TAFFY_CONTENT_ALIGN_STRETCH = 6,
    TAFFY_CONTENT_ALIGN_SPACE_BETWEEN = 7,
    TAFFY_CONTENT_ALIGN_SPACE_EVENLY = 8,
    TAFFY_CONTENT_ALIGN_SPACE_AROUND = 9,
    TAFFY_CONTENT_ALIGN_SAFE_START = 10,
    TAFFY_CONTENT_ALIGN_SAFE_END = 11,
    TAFFY_CONTENT_ALIGN_SAFE_FLEX_START = 12,
    TAFFY_CONTENT_ALIGN_SAFE_FLEX_END = 13,
    TAFFY_CONTENT_ALIGN_SAFE_CENTER = 14,
} taffy_align_content_t;

/* Amount of space offered to the root node when computing layout */
typedef enum taffy_available_space_type_t {
    /* value holds a definite size in abstract units */
    TAFFY_AVAILABLE_DEFINITE = 0,
    /* indefinite: lay out under a min-content constraint */
    TAFFY_AVAILABLE_MIN_CONTENT = 1,
    /* indefinite: lay out under a max-content constraint */
    TAFFY_AVAILABLE_MAX_CONTENT = 2,
} taffy_available_space_type_t;

/* ===========================================================================
 * Structs
 * ========================================================================= */

/* Opaque layout tree handle */
typedef struct taffy_tree_t taffy_tree_t;

/* Node identifier. Stable for the lifetime of the node in the tree. */
typedef uint64_t taffy_node_id;

/*
 * A length-like style value (see taffy_dim_type_t for legal discriminators).
 * Layout is identical on all platforms: one byte discriminator followed by a
 * 32 bit float (8 bytes with natural padding).
 */
typedef struct taffy_dim_t {
    uint8_t type;
    float value;
} taffy_dim_t;

/* Available space offered on one axis (see taffy_available_space_type_t) */
typedef struct taffy_available_space_t {
    uint8_t type;
    float value; /* used only when type == TAFFY_AVAILABLE_DEFINITE */
} taffy_available_space_t;

/*
 * The full settable style of a single node. Mirrors taffy's `Style` struct
 * (core + flexbox properties; CSS Grid template tracks are configured with
 * the taffy defaults for now and can be added in a later ABI revision).
 *
 * ALWAYS initialise with taffy_style_default() before setting fields - the
 * all-zero bit pattern does NOT represent taffy's defaults (notably
 * flex_shrink defaults to 1.0, margin/padding/border default to 0 and
 * size/inset default to auto).
 */
typedef struct taffy_style_t {
    /* Core */
    uint8_t display;          /* taffy_display_t */
    uint8_t box_sizing;       /* taffy_box_sizing_t */
    uint8_t position_type;    /* taffy_position_type_t */
    uint8_t direction;        /* taffy_direction_t */

    /* Flexbox container */
    uint8_t flex_direction;   /* taffy_flex_direction_t */
    uint8_t flex_wrap;        /* taffy_flex_wrap_t */
    uint8_t overflow_x;       /* taffy_overflow_t */
    uint8_t overflow_y;       /* taffy_overflow_t */

    /* Alignment (0 / *_AUTO means "not set") */
    uint8_t align_items;      /* taffy_align_items_t */
    uint8_t align_self;       /* taffy_align_items_t */
    uint8_t align_content;    /* taffy_align_content_t */
    uint8_t justify_content;  /* taffy_align_content_t */
    uint8_t justify_items;    /* taffy_align_items_t (grid) */
    uint8_t justify_self;     /* taffy_align_items_t (grid) */

    uint16_t flex_line_count; /* >= 1, used when flex_wrap is Balance* */

    float scrollbar_width;

    /* Flexbox item */
    float flex_grow;
    float flex_shrink;
    float aspect_ratio;       /* NaN = unset; otherwise width / height */

    /*
     * Rect edges are [left, right, top, bottom] (physical).
     * Sizes / gaps are [width, height].
     */
    taffy_dim_t inset[4];
    taffy_dim_t margin[4];
    taffy_dim_t padding[4];
    taffy_dim_t border[4];
    taffy_dim_t size[2];
    taffy_dim_t min_size[2];
    taffy_dim_t max_size[2];
    taffy_dim_t flex_basis;
    taffy_dim_t gap[2];
} taffy_style_t;

/* Computed layout of a single node, relative to its parent's border box. */
typedef struct taffy_layout_t {
    uint32_t order; /* paint order (topological; higher = on top) */
    float x;
    float y;
    float width;
    float height;
    float content_width;
    float content_height;
    float border[4];  /* [left, right, top, bottom] */
    float padding[4]; /* [left, right, top, bottom] */
    float margin[4];  /* [left, right, top, bottom] */
    float scroll_width;
    float scroll_height;
} taffy_layout_t;

/*
 * Number of floats written per node by taffy_tree_write_layouts_dfs().
 * Each record is: [x, y, width, height, child_count, order], produced in
 * depth-first pre-order, exactly like the float-array encoding used by the
 * stretch JNI binding (one additional trailing `order` float per node).
 */
#define TAFFY_LAYOUT_FLOATS_PER_NODE 6u

/*
 * Callback invoked to measure a childless (leaf) node during layout.
 *
 *  node             - id of the node being measured
 *  known_dimensions - pointer to 2 floats [width, height]; NaN when a
 *                     dimension is unknown
 *  available_space  - pointer to 2 taffy_available_space_t, [width, height]
 *  node_context     - the per-node context set via taffy_node_set_context
 *                     (NULL if none was set)
 *  measure_context  - the context pointer passed to the compute call
 *  out_size         - mandatory output, write measured [width, height] here
 *
 * The callback must not mutate the tree (it is invoked re-entrantly while a
 * layout computation is in progress).
 */
typedef void (*taffy_measure_callback_t)(taffy_node_id node,
                                         const float *known_dimensions,
                                         const taffy_available_space_t *available_space,
                                         void *node_context,
                                         void *measure_context,
                                         float *out_size);

/* ===========================================================================
 * Lifecycle / configuration
 * ========================================================================= */

/* Version string of this binding (semver, NUL terminated, static storage). */
const char *taffy_c_version(void);

/* Create a new, empty tree (default internal capacity). Free with taffy_tree_free. */
taffy_tree_t *taffy_tree_new(void);

/* Create a new, empty tree pre-sized for at least `capacity` nodes. */
taffy_tree_t *taffy_tree_new_with_capacity(size_t capacity);

/* Destroy a tree and all of its nodes. `tree` may be NULL (no-op). */
void taffy_tree_free(taffy_tree_t *tree);

/* Remove every node from the tree (the tree itself remains usable). */
taffy_status_t taffy_tree_clear(taffy_tree_t *tree);

/* Control rounding of layout values to the nearest integer (enabled by default). */
taffy_status_t taffy_tree_enable_rounding(taffy_tree_t *tree);
taffy_status_t taffy_tree_disable_rounding(taffy_tree_t *tree);

/* Total number of nodes currently stored in the tree. */
taffy_status_t taffy_tree_total_node_count(const taffy_tree_t *tree, size_t *out_count);

/* ===========================================================================
 * Style helpers
 * =========================================================================== */

/* Fill `out_style` with taffy's default style values. */
taffy_status_t taffy_style_default(taffy_style_t *out_style);

/* ===========================================================================
 * Node creation / removal / styling
 * ========================================================================= */

/* Create a leaf node (no children). Its id is written to *out_node. */
taffy_status_t taffy_node_new_leaf(taffy_tree_t *tree,
                                  const taffy_style_t *style,
                                  taffy_node_id *out_node);

/* Create a node with `child_count` existing children. */
taffy_status_t taffy_node_new_with_children(taffy_tree_t *tree,
                                            const taffy_style_t *style,
                                            const taffy_node_id *children,
                                            size_t child_count,
                                            taffy_node_id *out_node);

/* Remove a node (and detach it from its parent) from the tree entirely. */
taffy_status_t taffy_node_remove(taffy_tree_t *tree, taffy_node_id node);

/* Replace the style of an existing node. */
taffy_status_t taffy_node_set_style(taffy_tree_t *tree,
                                    taffy_node_id node,
                                    const taffy_style_t *style);

/* Attach / clear (NULL) an opaque host-side context pointer to a node. */
taffy_status_t taffy_node_set_context(taffy_tree_t *tree,
                                      taffy_node_id node,
                                      void *context);
taffy_status_t taffy_node_get_context(const taffy_tree_t *tree,
                                      taffy_node_id node,
                                      void **out_context);

/* ===========================================================================
 * Tree / hierarchy mutation
 * ========================================================================= */

taffy_status_t taffy_node_add_child(taffy_tree_t *tree,
                                    taffy_node_id parent,
                                    taffy_node_id child);

/* Insert at index; valid range is 0..=current_child_count. */
taffy_status_t taffy_node_insert_child_at_index(taffy_tree_t *tree,
                                                 taffy_node_id parent,
                                                 size_t index,
                                                 taffy_node_id child);

/* Replace the complete children list of `parent`. */
taffy_status_t taffy_node_set_children(taffy_tree_t *tree,
                                       taffy_node_id parent,
                                       const taffy_node_id *children,
                                       size_t child_count);

/* Replace a child by index. The removed child id is written to *out_old_child
 * when out_old_child is not NULL. */
taffy_status_t taffy_node_replace_child_at_index(taffy_tree_t *tree,
                                                 taffy_node_id parent,
                                                 size_t index,
                                                 taffy_node_id new_child,
                                                 taffy_node_id *out_old_child);

/* Detach `child` from `parent` (the child node itself stays in the tree). */
taffy_status_t taffy_node_remove_child(taffy_tree_t *tree,
                                       taffy_node_id parent,
                                       taffy_node_id child,
                                       taffy_node_id *out_removed_child);

taffy_status_t taffy_node_remove_child_at_index(taffy_tree_t *tree,
                                                taffy_node_id parent,
                                                size_t index,
                                                taffy_node_id *out_removed_child);

/* ===========================================================================
 * Tree queries
 * ========================================================================= */

taffy_status_t taffy_node_child_at_index(const taffy_tree_t *tree,
                                         taffy_node_id parent,
                                         size_t index,
                                         taffy_node_id *out_child);

taffy_status_t taffy_node_child_count(const taffy_tree_t *tree,
                                      taffy_node_id node,
                                      size_t *out_count);

/*
 * Copy the ids of all children into `out_children` (capacity entries).
 * The exact number of children is always written to *out_count. When the
 * buffer is NULL or too small, TAFFY_ERR_BUFFER_TOO_SMALL is returned (and
 * *out_count still holds the required count).
 */
taffy_status_t taffy_node_children(const taffy_tree_t *tree,
                                   taffy_node_id node,
                                   taffy_node_id *out_children,
                                   size_t capacity,
                                   size_t *out_count);

/* Returns TAFFY_ERR_NO_PARENT when the node is unattached. */
taffy_status_t taffy_node_parent(const taffy_tree_t *tree,
                                 taffy_node_id node,
                                 taffy_node_id *out_parent);

/* Dirty flag: true when the cached layout of the node is stale. */
taffy_status_t taffy_node_dirty(const taffy_tree_t *tree,
                                taffy_node_id node,
                                int *out_dirty);

/* Mark a node (and its ancestors) as requiring relayout. */
taffy_status_t taffy_node_mark_dirty(taffy_tree_t *tree, taffy_node_id node);

/* ===========================================================================
 * Layout computation & output
 * =========================================================================== */

/*
 * Compute the layout of `root` and its descendants.
 *
 * Leaf nodes without a measure callback are sized to 0x0 (like stretch's
 * non-measured leaves). Use taffy_tree_compute_layout_with_measure() if the
 * tree contains text/image leaves that need intrinsic measurement.
 */
taffy_status_t taffy_tree_compute_layout(taffy_tree_t *tree,
                                         taffy_node_id root,
                                         taffy_available_space_t available_width,
                                         taffy_available_space_t available_height);

/*
 * Same as taffy_tree_compute_layout(), but invokes `callback` for every
 * childless node so the embedder can measure its intrinsic content size.
 * `measure_context` is passed through to every callback unmodified.
 */
taffy_status_t taffy_tree_compute_layout_with_measure(
    taffy_tree_t *tree,
    taffy_node_id root,
    taffy_available_space_t available_width,
    taffy_available_space_t available_height,
    taffy_measure_callback_t callback,
    void *measure_context);

/* Read back the computed layout of a single node. */
taffy_status_t taffy_node_layout(const taffy_tree_t *tree,
                                 taffy_node_id node,
                                 taffy_layout_t *out_layout);

/*
 * Flatten the layouts of `root` and all of its descendants into a float
 * buffer in depth-first pre-order, one TAFFY_LAYOUT_FLOATS_PER_NODE record per
 * node: [x, y, width, height, child_count, order].
 *
 * The required number of floats is always written to *out_required. When
 * `out_buffer` is NULL or `capacity` is insufficient the function returns
 * TAFFY_ERR_BUFFER_TOO_SMALL without writing anything, allowing callers to
 * allocate an exactly sized buffer and retry.
 */
taffy_status_t taffy_tree_write_layouts_dfs(const taffy_tree_t *tree,
                                            taffy_node_id root,
                                            float *out_buffer,
                                            size_t capacity,
                                            size_t *out_required);

/* ===========================================================================
 * Inline ergonomic helpers (header-only, no ABI impact)
 * ========================================================================= */

static inline taffy_dim_t taffy_dim_length(float value) {
    taffy_dim_t d;
    d.type = TAFFY_DIM_LENGTH;
    d.value = value;
    return d;
}

static inline taffy_dim_t taffy_dim_percent(float value) {
    taffy_dim_t d;
    d.type = TAFFY_DIM_PERCENT;
    d.value = value;
    return d;
}

static inline taffy_dim_t taffy_dim_auto(void) {
    taffy_dim_t d;
    d.type = TAFFY_DIM_AUTO;
    d.value = 0.0f;
    return d;
}

static inline taffy_available_space_t taffy_available_definite(float value) {
    taffy_available_space_t a;
    a.type = TAFFY_AVAILABLE_DEFINITE;
    a.value = value;
    return a;
}

static inline taffy_available_space_t taffy_available_min_content(void) {
    taffy_available_space_t a;
    a.type = TAFFY_AVAILABLE_MIN_CONTENT;
    a.value = 0.0f;
    return a;
}

static inline taffy_available_space_t taffy_available_max_content(void) {
    taffy_available_space_t a;
    a.type = TAFFY_AVAILABLE_MAX_CONTENT;
    a.value = 0.0f;
    return a;
}

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* TAFFY_H */
