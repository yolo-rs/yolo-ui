//! Chrome layout computed with Taffy 0.15 (flexbox). The Dioxus tree only
//! consumes the resulting rectangles; it never does CSS layout of its own.

use taffy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ChromeLayout {
    pub viewport: Rect,
    pub toolbar: Rect,
    pub canvas: Rect,
    pub strip: Rect,
}

fn rect_of(tree: &TaffyTree<()>, node: NodeId, offset: (f32, f32)) -> Rect {
    let l = tree.layout(node).expect("layout computed");
    Rect {
        x: offset.0 + l.location.x,
        y: offset.1 + l.location.y,
        w: l.size.width,
        h: l.size.height,
    }
}

/// root (column)
///  |- toolbar (fixed height)
///  `- body (row, flex-grow)
///      |- canvas (flex-grow)
///      `- strip (fixed width)
pub fn compute(width: f32, height: f32) -> ChromeLayout {
    let mut tree: TaffyTree<()> = TaffyTree::new();

    let toolbar = tree
        .new_leaf(Style {
            size: Size { width: percent(1.0), height: length(crate::theme::TOOLBAR_H) },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .unwrap();

    let canvas = tree
        .new_leaf(Style {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            min_size: Size { width: length(0.0), height: length(0.0) },
            ..Default::default()
        })
        .unwrap();

    let strip = tree
        .new_leaf(Style {
            size: Size { width: length(crate::theme::STRIP_W), height: percent(1.0) },
            flex_shrink: 0.0,
            ..Default::default()
        })
        .unwrap();

    let body = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                min_size: Size { width: length(0.0), height: length(0.0) },
                ..Default::default()
            },
            &[canvas, strip],
        )
        .unwrap();

    let root = tree
        .new_with_children(
            Style {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                size: Size { width: length(width), height: length(height) },
                ..Default::default()
            },
            &[toolbar, body],
        )
        .unwrap();

    tree.compute_layout(
        root,
        Size { width: AvailableSpace::Definite(width), height: AvailableSpace::Definite(height) },
    )
    .unwrap();

    let body_rect = rect_of(&tree, body, (0.0, 0.0));
    ChromeLayout {
        viewport: rect_of(&tree, root, (0.0, 0.0)),
        toolbar: rect_of(&tree, toolbar, (0.0, 0.0)),
        canvas: rect_of(&tree, canvas, (body_rect.x, body_rect.y)),
        strip: rect_of(&tree, strip, (body_rect.x, body_rect.y)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn splits_viewport() {
        let l = super::compute(1280.0, 800.0);
        assert_eq!(l.toolbar.h, crate::theme::TOOLBAR_H);
        assert_eq!(l.strip.w, crate::theme::STRIP_W);
        assert_eq!(l.canvas.w + l.strip.w, 1280.0);
        assert_eq!(l.canvas.y, crate::theme::TOOLBAR_H);
        assert_eq!(l.canvas.h, 800.0 - crate::theme::TOOLBAR_H);
    }
}
