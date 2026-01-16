//! Flex Layout
//!
//! Implements the CSS Flexbox layout algorithm.

use crate::boxtree::LayoutBox;
use crate::block::layout_block;
use crate::inline::layout_inline_children;
use crate::ContainingBlock;
use gugalanna_style::{AlignItems, AlignSelf, Display, FlexDirection, JustifyContent};

/// Simple struct to hold flex item calculations
#[derive(Debug)]
struct FlexItemData {
    index: usize,
    order: i32,
    flex_grow: f32,
    flex_shrink: f32,
    flex_basis: f32,
    main_size: f32,
    cross_size: f32,
    frozen: bool,
}

/// Layout a flex container and its children
pub fn layout_flex(layout_box: &mut LayoutBox, containing_block: ContainingBlock) {
    let style = match layout_box.style() {
        Some(s) => s.clone(),
        None => return,
    };

    // Apply padding/border/margin from style
    layout_box.apply_style_edges();

    // Determine main axis direction
    let flex_direction = style.flex_direction;
    let is_row = matches!(flex_direction, FlexDirection::Row | FlexDirection::RowReverse);
    let is_reversed = matches!(flex_direction, FlexDirection::RowReverse | FlexDirection::ColumnReverse);

    // Calculate container available space
    let container_width = style.width.unwrap_or(
        containing_block.width
            - layout_box.dimensions.margin.horizontal()
            - layout_box.dimensions.border.horizontal()
            - layout_box.dimensions.padding.horizontal()
    );

    let container_height = style.height;

    // Set container content width for now
    layout_box.dimensions.content.width = container_width;

    // Calculate available main space
    let available_main = if is_row {
        container_width
    } else {
        container_height.unwrap_or(f32::MAX)
    };

    let available_cross = if is_row {
        container_height
    } else {
        Some(container_width)
    };

    // Step 1: Collect flex items and compute their base sizes
    let mut flex_items: Vec<FlexItemData> = Vec::new();

    for (index, child) in layout_box.children.iter_mut().enumerate() {
        let child_style = child.style().cloned();

        // Skip items with display: none
        if child_style.as_ref().map(|s| s.display == Display::None).unwrap_or(false) {
            continue;
        }

        let (flex_grow, flex_shrink, flex_basis, order) = if let Some(ref s) = child_style {
            (s.flex_grow, s.flex_shrink, s.flex_basis, s.order)
        } else {
            (0.0, 1.0, None, 0)
        };

        // Apply edges to child for correct margin box calculation
        child.apply_style_edges();

        // Determine base size
        let base_size = if let Some(basis) = flex_basis {
            basis
        } else {
            // Use explicit width/height if specified, otherwise use content-based size
            let explicit_size = if is_row {
                child_style.as_ref().and_then(|s| s.width)
            } else {
                child_style.as_ref().and_then(|s| s.height)
            };

            explicit_size.unwrap_or_else(|| {
                // Estimate based on content - need to do preliminary layout
                compute_intrinsic_main_size(child, is_row, available_main)
            })
        };

        flex_items.push(FlexItemData {
            index,
            order,
            flex_grow,
            flex_shrink,
            flex_basis: base_size,
            main_size: base_size,
            cross_size: 0.0,
            frozen: flex_grow == 0.0 && flex_shrink == 0.0,
        });
    }

    // Step 2: Sort by order property (stable sort preserves original order for equal values)
    flex_items.sort_by_key(|item| item.order);

    // Step 3: Resolve flexible lengths (flex-grow/flex-shrink algorithm)
    resolve_flexible_lengths(&mut flex_items, available_main);

    // Step 4: Layout each child and determine cross sizes
    for item_data in &mut flex_items {
        let child = &mut layout_box.children[item_data.index];

        // Set main size on child before layout
        if is_row {
            child.dimensions.content.width = item_data.main_size
                - child.dimensions.padding.horizontal()
                - child.dimensions.border.horizontal();
        } else {
            child.dimensions.content.height = item_data.main_size
                - child.dimensions.padding.vertical()
                - child.dimensions.border.vertical();
        }

        // Create containing block for child layout
        let child_containing = if is_row {
            ContainingBlock::new(
                item_data.main_size,
                available_cross.unwrap_or(0.0),
            )
        } else {
            ContainingBlock::new(
                available_cross.unwrap_or(container_width),
                item_data.main_size,
            )
        };

        // Layout the child
        layout_flex_item(child, child_containing, is_row, item_data.main_size);

        // Record cross size (margin box)
        item_data.cross_size = if is_row {
            child.dimensions.margin_box_height()
        } else {
            child.dimensions.margin_box_width()
        };
    }

    // Step 5: Determine cross size of flex container
    let max_cross_size: f32 = flex_items.iter()
        .map(|i| i.cross_size)
        .fold(0.0_f32, f32::max);

    let container_cross = available_cross.unwrap_or(max_cross_size);

    // Step 6: Position items along main axis (justify-content)
    let total_main_size: f32 = flex_items.iter().map(|i| i.main_size).sum();
    let free_space = (available_main - total_main_size).max(0.0);

    let (initial_offset, gap) = compute_main_axis_spacing(
        style.justify_content,
        free_space,
        flex_items.len(),
        is_reversed,
    );

    let mut main_cursor = initial_offset;

    // Iterate in correct order based on direction
    let item_indices: Vec<usize> = if is_reversed {
        (0..flex_items.len()).rev().collect()
    } else {
        (0..flex_items.len()).collect()
    };

    for i in item_indices {
        let item_data = &flex_items[i];
        let child = &mut layout_box.children[item_data.index];

        // Position on main axis
        if is_row {
            child.dimensions.content.x = main_cursor + child.dimensions.margin.left;
        } else {
            child.dimensions.content.y = main_cursor + child.dimensions.margin.top;
        }

        // Position on cross axis based on align-items/align-self
        let child_align = child.style()
            .map(|s| s.align_self)
            .unwrap_or(AlignSelf::Auto);

        let effective_align = if child_align == AlignSelf::Auto {
            style.align_items
        } else {
            match child_align {
                AlignSelf::FlexStart => AlignItems::FlexStart,
                AlignSelf::FlexEnd => AlignItems::FlexEnd,
                AlignSelf::Center => AlignItems::Center,
                AlignSelf::Stretch => AlignItems::Stretch,
                AlignSelf::Baseline => AlignItems::Baseline,
                AlignSelf::Auto => style.align_items,
            }
        };

        let child_cross_size = item_data.cross_size;
        let cross_pos = compute_cross_position(
            effective_align,
            child_cross_size,
            container_cross,
        );

        if is_row {
            child.dimensions.content.y = cross_pos + child.dimensions.margin.top;
        } else {
            child.dimensions.content.x = cross_pos + child.dimensions.margin.left;
        }

        // Advance cursor
        main_cursor += item_data.main_size + gap;
    }

    // Step 7: Set container final dimensions
    if is_row {
        layout_box.dimensions.content.width = container_width;
        layout_box.dimensions.content.height = if style.height.is_some() {
            container_height.unwrap()
        } else {
            container_cross
        };
    } else {
        layout_box.dimensions.content.width = container_width;
        layout_box.dimensions.content.height = if style.height.is_some() {
            container_height.unwrap()
        } else {
            total_main_size
        };
    }
}

/// Compute intrinsic main size of a flex item (content-based sizing)
fn compute_intrinsic_main_size(child: &mut LayoutBox, is_row: bool, _available: f32) -> f32 {
    // Apply edges first
    child.apply_style_edges();

    // For text or simple content, estimate based on font metrics
    if child.children.is_empty() {
        // Leaf node - use a reasonable default
        if is_row {
            // Width based on content - return a sensible minimum
            child.style().map(|s| s.font_size * 5.0).unwrap_or(80.0)
        } else {
            // Height based on line height
            child.style().map(|s| s.line_height).unwrap_or(20.0)
        }
    } else {
        // Has children - do a preliminary layout to measure
        // For simplicity, use explicit dimensions if available
        if is_row {
            child.style().and_then(|s| s.width).unwrap_or(100.0)
        } else {
            child.style().and_then(|s| s.height).unwrap_or(
                child.style().map(|s| s.line_height).unwrap_or(20.0)
            )
        }
    }
}

/// Resolve flexible lengths (flex-grow/flex-shrink algorithm)
fn resolve_flexible_lengths(items: &mut [FlexItemData], available_main: f32) {
    if items.is_empty() {
        return;
    }

    let total_basis: f32 = items.iter().map(|i| i.flex_basis).sum();

    if total_basis <= available_main {
        // We have free space - use flex-grow
        let free_space = available_main - total_basis;
        let total_grow: f32 = items.iter()
            .filter(|i| !i.frozen)
            .map(|i| i.flex_grow)
            .sum();

        if total_grow > 0.0 {
            for item in items.iter_mut() {
                if !item.frozen && item.flex_grow > 0.0 {
                    let grow_fraction = item.flex_grow / total_grow;
                    item.main_size = item.flex_basis + (free_space * grow_fraction);
                }
            }
        }
    } else {
        // We need to shrink - use flex-shrink
        let overflow = total_basis - available_main;
        let total_shrink: f32 = items.iter()
            .filter(|i| !i.frozen)
            .map(|i| i.flex_shrink * i.flex_basis)
            .sum();

        if total_shrink > 0.0 {
            for item in items.iter_mut() {
                if !item.frozen && item.flex_shrink > 0.0 {
                    let shrink_fraction = (item.flex_shrink * item.flex_basis) / total_shrink;
                    item.main_size = (item.flex_basis - (overflow * shrink_fraction)).max(0.0);
                }
            }
        }
    }
}

/// Layout a single flex item
fn layout_flex_item(
    child: &mut LayoutBox,
    containing_block: ContainingBlock,
    is_row: bool,
    main_size: f32,
) {
    // Apply edges from style
    child.apply_style_edges();

    // Set the appropriate dimension
    if is_row {
        child.dimensions.content.width = main_size
            - child.dimensions.padding.horizontal()
            - child.dimensions.border.horizontal();
        child.dimensions.content.width = child.dimensions.content.width.max(0.0);
    } else {
        child.dimensions.content.height = main_size
            - child.dimensions.padding.vertical()
            - child.dimensions.border.vertical();
        child.dimensions.content.height = child.dimensions.content.height.max(0.0);
    }

    // Check if this is a block or inline context
    let has_block_children = child.children.iter().any(|c| c.is_block());

    if has_block_children {
        // Use block layout for children
        layout_block(child, containing_block);
    } else if !child.children.is_empty() {
        // Use inline layout for children
        layout_inline_children(child);
    }

    // Ensure dimensions are set
    if child.dimensions.content.height == 0.0 && !child.children.is_empty() {
        let children_height: f32 = child.children
            .iter()
            .map(|c| c.dimensions.margin_box_height())
            .sum();
        child.dimensions.content.height = children_height;
    }

    if child.dimensions.content.width == 0.0 && !child.children.is_empty() {
        let children_width: f32 = child.children
            .iter()
            .map(|c| c.dimensions.margin_box_width())
            .fold(0.0_f32, f32::max);
        child.dimensions.content.width = children_width;
    }

    // For items with no children, ensure minimum size
    if child.children.is_empty() {
        if child.dimensions.content.height == 0.0 {
            child.dimensions.content.height = child.style()
                .map(|s| s.line_height)
                .unwrap_or(20.0);
        }
    }
}

/// Compute spacing for main axis based on justify-content
fn compute_main_axis_spacing(
    justify: JustifyContent,
    free_space: f32,
    item_count: usize,
    is_reversed: bool,
) -> (f32, f32) {
    if item_count == 0 {
        return (0.0, 0.0);
    }

    let (offset, gap) = match justify {
        JustifyContent::FlexStart => (0.0, 0.0),
        JustifyContent::FlexEnd => (free_space, 0.0),
        JustifyContent::Center => (free_space / 2.0, 0.0),
        JustifyContent::SpaceBetween => {
            if item_count == 1 {
                (0.0, 0.0)
            } else {
                (0.0, free_space / (item_count - 1) as f32)
            }
        }
        JustifyContent::SpaceAround => {
            let gap = free_space / item_count as f32;
            (gap / 2.0, gap)
        }
        JustifyContent::SpaceEvenly => {
            let gap = free_space / (item_count + 1) as f32;
            (gap, gap)
        }
    };

    // Adjust for reversed direction
    if is_reversed {
        match justify {
            JustifyContent::FlexStart => (free_space, 0.0),
            JustifyContent::FlexEnd => (0.0, 0.0),
            _ => (offset, gap),
        }
    } else {
        (offset, gap)
    }
}

/// Compute cross axis position based on align-items
fn compute_cross_position(
    align: AlignItems,
    item_cross_size: f32,
    container_cross_size: f32,
) -> f32 {
    match align {
        AlignItems::FlexStart => 0.0,
        AlignItems::FlexEnd => (container_cross_size - item_cross_size).max(0.0),
        AlignItems::Center => ((container_cross_size - item_cross_size) / 2.0).max(0.0),
        AlignItems::Stretch => 0.0, // Item will stretch to fill
        AlignItems::Baseline => 0.0, // Simplified - real impl needs baseline calculation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_axis_spacing_flex_start() {
        let (offset, gap) = compute_main_axis_spacing(JustifyContent::FlexStart, 100.0, 3, false);
        assert_eq!(offset, 0.0);
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn test_main_axis_spacing_flex_end() {
        let (offset, gap) = compute_main_axis_spacing(JustifyContent::FlexEnd, 100.0, 3, false);
        assert_eq!(offset, 100.0);
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn test_main_axis_spacing_center() {
        let (offset, gap) = compute_main_axis_spacing(JustifyContent::Center, 100.0, 3, false);
        assert_eq!(offset, 50.0);
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn test_main_axis_spacing_space_between() {
        let (offset, gap) = compute_main_axis_spacing(JustifyContent::SpaceBetween, 100.0, 3, false);
        assert_eq!(offset, 0.0);
        assert_eq!(gap, 50.0);
    }

    #[test]
    fn test_main_axis_spacing_space_around() {
        let (offset, gap) = compute_main_axis_spacing(JustifyContent::SpaceAround, 120.0, 3, false);
        assert_eq!(offset, 20.0);
        assert_eq!(gap, 40.0);
    }

    #[test]
    fn test_main_axis_spacing_space_evenly() {
        let (offset, gap) = compute_main_axis_spacing(JustifyContent::SpaceEvenly, 100.0, 4, false);
        assert_eq!(offset, 20.0);
        assert_eq!(gap, 20.0);
    }

    #[test]
    fn test_cross_position_flex_start() {
        let pos = compute_cross_position(AlignItems::FlexStart, 50.0, 100.0);
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn test_cross_position_flex_end() {
        let pos = compute_cross_position(AlignItems::FlexEnd, 50.0, 100.0);
        assert_eq!(pos, 50.0);
    }

    #[test]
    fn test_cross_position_center() {
        let pos = compute_cross_position(AlignItems::Center, 50.0, 100.0);
        assert_eq!(pos, 25.0);
    }

    #[test]
    fn test_flexible_lengths_grow() {
        let mut items = vec![
            FlexItemData {
                index: 0, order: 0, flex_grow: 1.0, flex_shrink: 1.0,
                flex_basis: 100.0, main_size: 100.0, cross_size: 0.0, frozen: false,
            },
            FlexItemData {
                index: 1, order: 0, flex_grow: 2.0, flex_shrink: 1.0,
                flex_basis: 100.0, main_size: 100.0, cross_size: 0.0, frozen: false,
            },
        ];

        resolve_flexible_lengths(&mut items, 400.0);

        // 200px free space, distributed 1:2 = 66.67 and 133.33
        assert!((items[0].main_size - 166.67).abs() < 1.0);
        assert!((items[1].main_size - 233.33).abs() < 1.0);
    }

    #[test]
    fn test_flexible_lengths_shrink() {
        let mut items = vec![
            FlexItemData {
                index: 0, order: 0, flex_grow: 0.0, flex_shrink: 1.0,
                flex_basis: 200.0, main_size: 200.0, cross_size: 0.0, frozen: false,
            },
            FlexItemData {
                index: 1, order: 0, flex_grow: 0.0, flex_shrink: 1.0,
                flex_basis: 200.0, main_size: 200.0, cross_size: 0.0, frozen: false,
            },
        ];

        resolve_flexible_lengths(&mut items, 300.0);

        // 100px overflow, each shrinks by 50
        assert_eq!(items[0].main_size, 150.0);
        assert_eq!(items[1].main_size, 150.0);
    }
}
