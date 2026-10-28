use std::{fs, path::PathBuf};

#[expect(clippy::expect_used, reason = "Missing fixtures must fail the test.")]
fn app_file(path: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path))
        .expect("branding file must exist")
}

#[test]
fn android_launcher_inherits_one_brand_label() {
    let config = app_file("Dioxus.toml");
    assert!(config.contains("android_manifest = \"assets/icon/AndroidManifest.xml\""));
    let manifest = app_file("assets/icon/AndroidManifest.xml");
    assert_eq!(manifest.matches("android:label=").count(), 1);
    assert!(manifest.contains("android:label=\"werdy\""));
    assert!(manifest.contains("android:icon=\"@mipmap/werdy_launcher\""));
    assert!(manifest.contains("android:name=\"android.app.lib_name\" android:value=\"main\""));
    assert!(manifest.contains("android.intent.category.LAUNCHER"));
}

#[test]
fn adaptive_icon_uses_our_foreground_and_background() {
    let adaptive = app_file("src/android/src/main/res/mipmap-anydpi-v26/werdy_launcher.xml");
    assert!(adaptive.contains("@drawable/werdy_foreground"));
    assert!(adaptive.contains("@color/werdy_icon_background"));
    let foreground = app_file("src/android/src/main/res/drawable/werdy_foreground.xml");
    assert!(foreground.contains("android:viewportWidth=\"108\""));
    assert!(!foreground.contains("ic_launcher"));
}

#[test]
fn foreground_artwork_fits_android_safe_circle() {
    let source = app_file("assets/icon/icon.svg");
    let attribute = |element: &str, name: &str| -> f64 {
        element
            .split(&format!("{name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    for circle in source.lines().filter(|line| line.contains("<circle ")) {
        let x = attribute(circle, "cx") - 54.0;
        let y = attribute(circle, "cy") - 54.0;
        assert!(x.hypot(y) + attribute(circle, "r") <= 33.0);
    }
    let check = source
        .lines()
        .find(|line| line.contains("<polyline "))
        .unwrap();
    let points = check
        .split("points=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    for point in points.split_whitespace() {
        let (x, y) = point.split_once(',').unwrap();
        let x = x.parse::<f64>().unwrap() - 54.0;
        let y = y.parse::<f64>().unwrap() - 54.0;
        assert!(x.hypot(y) + attribute(check, "stroke-width") / 2.0 <= 33.0);
    }
}

#[test]
fn favicon_and_legacy_launcher_images_have_declared_sizes() {
    assert!(app_file("index.html").contains("/{base_path}/icons/favicon.svg"));
    assert_eq!(
        app_file("public/icons/favicon.svg"),
        app_file("assets/icon/icon.svg")
    );
    for (density, size) in [
        ("mdpi", 48_u32),
        ("hdpi", 72),
        ("xhdpi", 96),
        ("xxhdpi", 144),
        ("xxxhdpi", 192),
    ] {
        let path = format!("src/android/src/main/res/mipmap-{density}/werdy_launcher.png");
        let bytes = fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), size);
        assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), size);
    }
}
