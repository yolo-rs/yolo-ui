//! Components and generated structure must not carry colour literals: every colour is computed
//! from the active theme bundle and referenced as `var(--yolo-<role>)`.

use std::{fs, path::Path};

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).expect("read_dir").flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn has_hex_color(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b != b'#' {
            continue;
        }
        let run = bytes[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_hexdigit())
            .count();
        let next_is_word = bytes
            .get(i + 1 + run)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_');
        if matches!(run, 3 | 4 | 6 | 8) && !next_is_word {
            return Some(text[i..i + 1 + run].to_owned());
        }
    }
    None
}

#[test]
fn no_hex_color_in_sources() {
    let mut files = Vec::new();
    rust_files(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    assert!(files.len() > 10);
    for file in files {
        let text = fs::read_to_string(&file).expect("read");
        if let Some(hex) = has_hex_color(&text) {
            panic!("{}: hardcoded colour {hex}", file.display());
        }
    }
}

#[test]
fn structure_uses_variables_and_keeps_the_a11y_rules() {
    let structure = shenron_site::structure::render_structure();
    assert!(has_hex_color(&structure).is_none());
    assert!(structure.contains(":focus-visible"));
    assert!(structure.contains("forced-colors: active"));
    assert!(structure.contains("prefers-reduced-motion: reduce"));
    assert!(structure.contains("data-motion='reduce'"));
}
