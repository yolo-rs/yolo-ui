// SPDX-License-Identifier: Apache-2.0
//! Neutral, zero-hardcode Sprite Sheet & Texture Atlas engine (`yolo_gpu::sprite_sheet`).
//!
//! Unifies grid-based and free-form 2D sprite atlases (CSS sprite sheets, SVG `<symbol>` + `<view>`
//! sheets, and JSON metadata manifests) with clean English APIs alongside full backwards-compatible
//! aliases from `aphrody-sprite-sheet`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// Errors returned by [`SpriteSheet`] operations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SpriteSheetError {
    #[error("sheet dimensions must be strictly positive (got {0}x{1})")]
    InvalidDimensions(u32, u32),
    #[error("sprite '{id}' is out of bounds ({x}+{w} > {max_w} or {y}+{h} > {max_h})")]
    SpriteOutOfBounds {
        id: String,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        max_w: u32,
        max_h: u32,
    },
    #[error("grid dimensions (columns={0}, rows={1}) must be non-zero")]
    InvalidGrid(u32, u32),
    #[error("JSON serialization failed: {0}")]
    JsonError(String),
}

/// Backwards-compatible alias for [`SpriteSheetError`].
pub type ErreurSpriteSheet = SpriteSheetError;

/// A single rectangular sprite cell inside a [`SpriteSheet`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprite {
    pub id: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Sprite {
    /// Creates a new [`Sprite`] region.
    #[must_use]
    pub fn new(id: impl Into<String>, x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            id: id.into(),
            x,
            y,
            width,
            height,
        }
    }

    /// Normalized `[u0, v0, u1, v1]` UV coordinates within a sheet of `(sheet_width, sheet_height)`.
    #[must_use]
    pub fn uv_rect(&self, sheet_width: u32, sheet_height: u32) -> [f32; 4] {
        if sheet_width == 0 || sheet_height == 0 {
            return [0.0, 0.0, 1.0, 1.0];
        }
        let sw = sheet_width as f32;
        let sh = sheet_height as f32;
        [
            self.x as f32 / sw,
            self.y as f32 / sh,
            (self.x + self.width) as f32 / sw,
            (self.y + self.height) as f32 / sh,
        ]
    }
}

/// Neutral 2D sprite sheet / texture atlas manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpriteSheet {
    pub image_source: String,
    pub total_width: u32,
    pub total_height: u32,
    pub sprites: BTreeMap<String, Sprite>,
}

impl SpriteSheet {
    /// Creates a new empty [`SpriteSheet`] with the given `image_source` and pixel dimensions.
    pub fn new(
        image_source: impl Into<String>,
        total_width: u32,
        total_height: u32,
    ) -> Result<Self, SpriteSheetError> {
        if total_width == 0 || total_height == 0 {
            return Err(SpriteSheetError::InvalidDimensions(
                total_width,
                total_height,
            ));
        }
        Ok(Self {
            image_source: image_source.into(),
            total_width,
            total_height,
            sprites: BTreeMap::new(),
        })
    }

    /// Backwards-compatible alias for [`Self::new`].
    pub fn nouvelle(
        image_source: impl Into<String>,
        largeur_totale: u32,
        hauteur_totale: u32,
    ) -> Result<Self, SpriteSheetError> {
        Self::new(image_source, largeur_totale, hauteur_totale)
    }

    /// Builds a uniform grid [`SpriteSheet`] (`columns` x `rows`) with `prefix` sprite IDs.
    pub fn from_grid(
        image_source: impl Into<String>,
        total_width: u32,
        total_height: u32,
        columns: u32,
        rows: u32,
        prefix: &str,
    ) -> Result<Self, SpriteSheetError> {
        if columns == 0 || rows == 0 {
            return Err(SpriteSheetError::InvalidGrid(columns, rows));
        }
        let mut sheet = Self::new(image_source, total_width, total_height)?;
        let sprite_w = total_width / columns;
        let sprite_h = total_height / rows;
        if sprite_w == 0 || sprite_h == 0 {
            return Err(SpriteSheetError::InvalidDimensions(sprite_w, sprite_h));
        }
        for r in 0..rows {
            for c in 0..columns {
                let id = format!("{prefix}-r{r}-c{c}");
                sheet.add_sprite(Sprite {
                    id,
                    x: c * sprite_w,
                    y: r * sprite_h,
                    width: sprite_w,
                    height: sprite_h,
                })?;
            }
        }
        Ok(sheet)
    }

    /// Backwards-compatible alias for [`Self::from_grid`].
    pub fn depuis_grille(
        image_source: impl Into<String>,
        largeur_totale: u32,
        hauteur_totale: u32,
        colonnes: u32,
        lignes: u32,
        prefixe: &str,
    ) -> Result<Self, SpriteSheetError> {
        Self::from_grid(
            image_source,
            largeur_totale,
            hauteur_totale,
            colonnes,
            lignes,
            prefixe,
        )
    }

    /// Adds a [`Sprite`] to the sheet after bounds-checking against `total_width` x `total_height`.
    pub fn add_sprite(&mut self, sprite: Sprite) -> Result<(), SpriteSheetError> {
        if sprite.width == 0 || sprite.height == 0 {
            return Err(SpriteSheetError::InvalidDimensions(
                sprite.width,
                sprite.height,
            ));
        }
        if sprite.x.saturating_add(sprite.width) > self.total_width
            || sprite.y.saturating_add(sprite.height) > self.total_height
        {
            return Err(SpriteSheetError::SpriteOutOfBounds {
                id: sprite.id,
                x: sprite.x,
                y: sprite.y,
                w: sprite.width,
                h: sprite.height,
                max_w: self.total_width,
                max_h: self.total_height,
            });
        }
        self.sprites.insert(sprite.id.clone(), sprite);
        Ok(())
    }

    /// Backwards-compatible alias for [`Self::add_sprite`].
    pub fn ajouter_sprite(&mut self, sprite: Sprite) -> Result<(), SpriteSheetError> {
        self.add_sprite(sprite)
    }

    /// Looks up a sprite by `id`.
    #[must_use]
    pub fn get_sprite(&self, id: &str) -> Option<&Sprite> {
        self.sprites.get(id)
    }

    /// Backwards-compatible alias for [`Self::get_sprite`].
    #[must_use]
    pub fn obtenir_sprite(&self, id: &str) -> Option<&Sprite> {
        self.get_sprite(id)
    }

    /// Exports a CSS stylesheet with `.{class_prefix}` and `.{class_prefix}-{id}` rules.
    #[must_use]
    pub fn to_css(&self, class_prefix: &str) -> String {
        let mut css = String::new();
        css.push_str(&format!(
            ".{class_prefix} {{\n  background-image: url('{}');\n  background-repeat: no-repeat;\n  background-size: {}px {}px;\n  display: inline-block;\n}}\n",
            self.image_source, self.total_width, self.total_height
        ));
        for sprite in self.sprites.values() {
            let pos_x = if sprite.x == 0 {
                "0".to_string()
            } else {
                format!("-{}px", sprite.x)
            };
            let pos_y = if sprite.y == 0 {
                "0".to_string()
            } else {
                format!("-{}px", sprite.y)
            };
            css.push_str(&format!(
                ".{class_prefix}-{} {{\n  width: {}px;\n  height: {}px;\n  background-position: {} {};\n}}\n",
                sprite.id, sprite.width, sprite.height, pos_x, pos_y
            ));
        }
        css
    }

    /// Backwards-compatible alias for [`Self::to_css`].
    #[must_use]
    pub fn vers_css(&self, prefixe_classe: &str) -> String {
        self.to_css(prefixe_classe)
    }

    /// Exports an SVG sprite sheet containing `<symbol>` and `<view>` entries for every sprite.
    #[must_use]
    pub fn to_svg_symbols(&self) -> String {
        let mut svg = String::new();
        svg.push_str(&format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">\n",
            self.total_width, self.total_height, self.total_width, self.total_height
        ));
        svg.push_str("  <defs>\n");
        svg.push_str(&format!(
            "    <image id=\"sheet-source\" href=\"{}\" width=\"{}\" height=\"{}\" />\n",
            self.image_source, self.total_width, self.total_height
        ));
        for sprite in self.sprites.values() {
            let offset_x = if sprite.x == 0 {
                "0".to_string()
            } else {
                format!("-{}", sprite.x)
            };
            let offset_y = if sprite.y == 0 {
                "0".to_string()
            } else {
                format!("-{}", sprite.y)
            };
            svg.push_str(&format!(
                "    <symbol id=\"{}\" viewBox=\"0 0 {} {}\">\n      <use href=\"#sheet-source\" x=\"{}\" y=\"{}\" />\n    </symbol>\n",
                sprite.id, sprite.width, sprite.height, offset_x, offset_y
            ));
        }
        svg.push_str("  </defs>\n");
        for sprite in self.sprites.values() {
            svg.push_str(&format!(
                "  <view id=\"view-{}\" viewBox=\"{} {} {} {}\" />\n",
                sprite.id, sprite.x, sprite.y, sprite.width, sprite.height
            ));
        }
        svg.push_str("  <use href=\"#sheet-source\" x=\"0\" y=\"0\" />\n");
        svg.push_str("</svg>\n");
        svg
    }

    /// Backwards-compatible alias for [`Self::to_svg_symbols`].
    #[must_use]
    pub fn vers_svg_symboles(&self) -> String {
        self.to_svg_symbols()
    }

    /// Serializes the sprite sheet manifest as pretty JSON.
    pub fn to_json(&self) -> Result<String, SpriteSheetError> {
        serde_json::to_string_pretty(self).map_err(|e| SpriteSheetError::JsonError(e.to_string()))
    }

    /// Backwards-compatible alias for [`Self::to_json`].
    pub fn vers_json(&self) -> Result<String, SpriteSheetError> {
        self.to_json()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_sprite_sheet_and_exports_work() {
        let sheet = SpriteSheet::from_grid("assets/sprites.png", 1536, 2288, 8, 11, "cell")
            .expect("valid 8x11 grid");
        assert_eq!(sheet.sprites.len(), 88);
        let cell = sheet.get_sprite("cell-r0-c0").expect("r0-c0 exists");
        assert_eq!((cell.width, cell.height), (192, 208));
        assert_eq!(cell.uv_rect(1536, 2288), [0.0, 0.0, 0.125, 208.0 / 2288.0]);

        let css = sheet.to_css("yolo-sprite");
        assert!(css.contains(".yolo-sprite-cell-r1-c2"));
        assert!(css.contains("background-position: -384px -208px;"));

        let svg = sheet.to_svg_symbols();
        assert!(svg.contains("<symbol id=\"cell-r0-c0\" viewBox=\"0 0 192 208\">"));

        let json = sheet.to_json().expect("json");
        assert!(json.contains("\"total_width\": 1536"));
    }

    #[test]
    fn rejects_out_of_bounds_sprites() {
        let mut sheet = SpriteSheet::new("atlas.png", 100, 100).unwrap();
        let res = sheet.add_sprite(Sprite::new("overflow", 80, 80, 30, 30));
        assert!(matches!(res, Err(SpriteSheetError::SpriteOutOfBounds { .. })));
    }
}
