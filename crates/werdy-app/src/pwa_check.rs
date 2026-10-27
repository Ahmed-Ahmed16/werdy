//! Checks the install manifest against the shipped image files.

use std::fs;
use std::path::Path;

use serde_json::Value;

#[test]
fn manifest_icons_exist_with_declared_png_sizes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("public");
    let manifest: Value = serde_json::from_str(include_str!("../public/manifest.webmanifest"))
        .expect("manifest must parse as JSON");
    let icons = manifest["icons"]
        .as_array()
        .expect("icons must be an array");
    assert_eq!(icons.len(), 3);

    for icon in icons {
        let source = icon["src"].as_str().expect("icon source must be text");
        let size = icon["sizes"].as_str().expect("icon size must be text");
        let expected = match size {
            "192x192" => 192_u32,
            "512x512" => 512_u32,
            _ => panic!("unexpected icon size: {size}"),
        };
        assert!(source.starts_with("icons/") && !source.contains(".."));
        assert_eq!(icon["type"], "image/png");
        let bytes = fs::read(root.join(source)).expect("manifest icon must exist");
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"), "icon must be PNG");
        assert_eq!(
            u32::from_be_bytes(bytes[16..20].try_into().unwrap()),
            expected
        );
        assert_eq!(
            u32::from_be_bytes(bytes[20..24].try_into().unwrap()),
            expected
        );
    }
    assert_eq!(manifest["icons"][2]["purpose"], "maskable");
}
