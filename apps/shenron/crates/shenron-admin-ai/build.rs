//! Resolves the colour roles of `src/tokens.rs` in HCT (yolo-ui-color, CAM16 hue/chroma + L*
//! tone), enforces every declared WCAG contrast pair, and renders the stylesheet into OUT_DIR.
//!
//! A role that misses its ratio is pushed away from its backgrounds one tone at a time; if it
//! cannot reach it inside `[0, 100]` the build fails. The crate therefore cannot ship a palette
//! below 4.5:1 for text or 3:1 for UI components, in either scheme.

use std::{env, fs, path::Path};
use yolo_ui_color::{Argb, Hct};

#[allow(dead_code)]
mod tokens {
    include!("src/tokens.rs");
}

use tokens::{Resolved, Role};

fn ratio(a: Argb, b: Argb) -> f64 {
    let (ya, yb) = (a.luminance_y() / 100.0, b.luminance_y() / 100.0);
    let (hi, lo) = if ya > yb { (ya, yb) } else { (yb, ya) };
    (hi + 0.05) / (lo + 0.05)
}

struct Done {
    name: &'static str,
    argb: Argb,
    tone: f64,
}

fn resolve(scheme: &str, roles: &[Role]) -> Vec<Done> {
    let mut done: Vec<Done> = Vec::new();
    for r in roles {
        let hue = Hct::from_argb(Argb(r.hue.seed())).hue();
        let backs: Vec<&Done> = r
            .against
            .iter()
            .map(|n| {
                done.iter()
                    .find(|d| d.name == *n)
                    .unwrap_or_else(|| panic!("{scheme}: role {} references unknown {n}", r.name))
            })
            .collect();
        let mean_back = if backs.is_empty() {
            50.0
        } else {
            backs.iter().map(|b| b.tone).sum::<f64>() / backs.len() as f64
        };
        let step = if mean_back < 50.0 { 1.0 } else { -1.0 };
        let mut tone = r.tone;
        loop {
            let argb = Hct::new(hue, r.chroma, tone).to_argb();
            let worst = backs
                .iter()
                .map(|b| ratio(argb, b.argb))
                .fold(f64::INFINITY, f64::min);
            if backs.is_empty() || worst >= r.min_ratio {
                done.push(Done { name: r.name, argb, tone });
                break;
            }
            tone += step;
            if !(0.0..=100.0).contains(&tone) {
                panic!("{scheme}: role {} cannot reach {}:1", r.name, r.min_ratio);
            }
        }
    }
    done
}

fn main() {
    println!("cargo:rerun-if-changed=src/tokens.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let dark = resolve("dark", tokens::DARK);
    let light = resolve("light", tokens::LIGHT);
    let to_resolved = |v: &[Done]| -> Vec<Resolved> {
        v.iter()
            .map(|d| Resolved { name: d.name, hex: d.argb.to_hex() })
            .collect()
    };
    let css = tokens::render_css(&to_resolved(&dark), &to_resolved(&light));

    // Contrast table for the unit tests: every declared (fg, bg) pair with its ratio.
    let mut table = String::from(
        "/// (scheme, fg, bg, ratio, required)\npub const CONTRAST: &[(&str, &str, &str, f64, f64)] = &[\n",
    );
    for (scheme, roles, done) in [
        ("dark", tokens::DARK, &dark),
        ("light", tokens::LIGHT, &light),
    ] {
        for r in roles {
            let fg = done.iter().find(|d| d.name == r.name).expect("resolved");
            for b in r.against {
                let bg = done.iter().find(|d| d.name == *b).expect("resolved");
                table.push_str(&format!(
                    "    (\"{scheme}\", \"{}\", \"{}\", {:.3}, {:.1}),\n",
                    r.name,
                    b,
                    ratio(fg.argb, bg.argb),
                    r.min_ratio
                ));
            }
        }
    }
    table.push_str("];\n");

    let out = env::var("OUT_DIR").expect("OUT_DIR");
    let write = |name: &str, body: &str| {
        let path = Path::new(&out).join(name);
        if fs::read_to_string(&path).ok().as_deref() != Some(body) {
            fs::write(&path, body).expect("write generated file");
        }
    };
    write("admin-ai.css", &css);
    write("contrast.rs", &table);
}
