// SPDX-License-Identifier: Apache-2.0
//! Canonical M3 shape tokens mapped to egui corner radii.

use egui::CornerRadius;

/// 8 dp small corner radius for interactive controls.
pub const CONTROL: CornerRadius = CornerRadius::same(yolo_tokens::shape::SMALL.dp as u8);
/// 4 dp extra-small corner radius for menus and popovers.
pub const MENU: CornerRadius = CornerRadius::same(yolo_tokens::shape::EXTRA_SMALL.dp as u8);
/// 12 dp medium corner radius for windows and dialogs.
pub const WINDOW: CornerRadius = CornerRadius::same(yolo_tokens::shape::MEDIUM.dp as u8);
