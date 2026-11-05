//! Generated wallpaper: an SVG painted from the resolved colour roles.
//!
//! Used when a pack ships no wallpaper, so a theme always has one and it always matches the
//! scheme, including after the user changes the seed. The composition (positions and sizes of the
//! colour fields) is derived from the seed, so two seeds give two different images, and the same
//! seed always gives the same image. Nothing is downloaded and there is no licence to track.

use std::fmt::Write as _;

fn role<'a>(roles: &'a [(String, String)], name: &str, fallback: &'a str) -> &'a str {
    roles
        .iter()
        .find(|(n, _)| n == name)
        .map_or(fallback, |(_, v)| v.as_str())
}

/// Small deterministic generator (SplitMix64) seeded from the colour seed.
struct Mix(u64);

impl Mix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform value in `lo..hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + u32::try_from(self.next() % u64::from(hi - lo)).unwrap_or(0)
    }
}

/// SVG wallpaper (2560x1600 view box, scales with `preserveAspectRatio="xMidYMid slice"`).
#[must_use]
pub fn generated_wallpaper_svg(roles: &[(String, String)], seed: u32) -> String {
    let base = role(roles, "surface", "#1c1b1f");
    let deep = role(roles, "surface-container-lowest", base);
    let fields = [
        role(roles, "primary-container", "#4f378b"),
        role(roles, "tertiary-container", "#633b48"),
        role(roles, "secondary-container", "#4a4458"),
        role(roles, "primary", "#d0bcff"),
    ];
    let mut mix = Mix(u64::from(seed));
    let mut out = String::with_capacity(1400);
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 2560 1600\" preserveAspectRatio=\"xMidYMid slice\">\
<defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\"><stop offset=\"0\" stop-color=\"{base}\"/><stop offset=\"1\" stop-color=\"{deep}\"/></linearGradient>\
<filter id=\"b\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"180\"/></filter></defs>\
<rect width=\"2560\" height=\"1600\" fill=\"url(#g)\"/><g filter=\"url(#b)\">"
    );
    for (i, color) in fields.iter().enumerate() {
        let cx = mix.range(200, 2360);
        let cy = mix.range(150, 1450);
        let rx = mix.range(380, 900);
        let ry = mix.range(260, 700);
        // The accent field stays small and faint so text over the wallpaper keeps its contrast.
        let opacity = if i == fields.len() - 1 { 0.35 } else { 0.85 };
        let _ = write!(
            out,
            "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{}\" ry=\"{}\" fill=\"{color}\" opacity=\"{opacity}\"/>",
            if i == fields.len() - 1 { rx / 2 } else { rx },
            if i == fields.len() - 1 { ry / 2 } else { ry }
        );
    }
    out.push_str("</g></svg>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roles() -> Vec<(String, String)> {
        [
            ("surface", "#101418"),
            ("primary-container", "#004f58"),
            ("primary", "#82d3e0"),
        ]
        .iter()
        .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
        .collect()
    }

    #[test]
    fn deterministic_per_seed_and_different_across_seeds() {
        let a = generated_wallpaper_svg(&roles(), 1);
        assert_eq!(a, generated_wallpaper_svg(&roles(), 1));
        assert_ne!(a, generated_wallpaper_svg(&roles(), 2));
    }

    #[test]
    fn uses_scheme_colours_and_is_well_formed() {
        let svg = generated_wallpaper_svg(&roles(), 7);
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(svg.contains("#101418") && svg.contains("#004f58"));
        assert_eq!(svg.matches("<ellipse").count(), 4);
    }
}
