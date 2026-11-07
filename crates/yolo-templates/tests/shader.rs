// SPDX-License-Identifier: Apache-2.0
//! The starfield WGSL parses and validates with naga, and its uniform block matches the Rust
//! struct the host writes.

use yolo_templates::motion::{STARFIELD_WGSL, StarfieldClock};

#[test]
fn starfield_wgsl_validates() {
    let module = naga::front::wgsl::parse_str(STARFIELD_WGSL).expect("WGSL parses");
    let mut validator = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
    validator.validate(&module).expect("WGSL validates");
    let entry: Vec<_> = module.entry_points.iter().map(|e| e.name.as_str()).collect();
    assert!(entry.contains(&"vs_main") && entry.contains(&"fs_main"));
}

#[test]
fn uniform_block_is_sixteen_bytes() {
    let module = naga::front::wgsl::parse_str(STARFIELD_WGSL).unwrap();
    let uniforms = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("Uniforms"))
        .expect("Uniforms struct");
    match &uniforms.1.inner {
        naga::TypeInner::Struct { span, members } => {
            assert_eq!(*span, 16);
            let names: Vec<_> = members.iter().map(|m| m.name.as_deref().unwrap_or("")).collect();
            assert_eq!(names, ["resolution", "time", "density"]);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(StarfieldClock::new(false).uniforms(1.0, 1.0, 0.5).to_bytes().len(), 16);
}
