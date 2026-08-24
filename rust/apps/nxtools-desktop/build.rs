#[path = "src/windows_icon.rs"]
mod windows_icon;

use std::{env, fs, path::PathBuf};

const BRAND_ICONS: &[(u8, &str)] = &[
    (
        16,
        "../../../browser_extension/nxtools-interceptor/icons/icon-16.png",
    ),
    (
        32,
        "../../../browser_extension/nxtools-interceptor/icons/icon-32.png",
    ),
    (
        48,
        "../../../browser_extension/nxtools-interceptor/icons/icon-48.png",
    ),
    (
        128,
        "../../../browser_extension/nxtools-interceptor/icons/icon-128.png",
    ),
];

fn main() {
    for (_, path) in BRAND_ICONS {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/windows_icon.rs");

    if env::var_os("CARGO_CFG_TARGET_OS").as_deref() != Some(std::ffi::OsStr::new("windows")) {
        return;
    }

    let images = BRAND_ICONS
        .iter()
        .map(|(size, path)| {
            fs::read(path)
                .map(|bytes| (*size, bytes))
                .unwrap_or_else(|error| panic!("failed to read brand icon {path}: {error}"))
        })
        .collect::<Vec<_>>();
    let image_refs = images
        .iter()
        .map(|(size, bytes)| (*size, bytes.as_slice()))
        .collect::<Vec<_>>();
    let icon = windows_icon::encode_png_ico(&image_refs)
        .unwrap_or_else(|error| panic!("failed to assemble desktop icon: {error}"));

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let icon_path = output.join("nxtools.ico");
    fs::write(&icon_path, icon).expect("write generated desktop icon");
    let resource_path = output.join("nxtools-icon.rc");
    let resource_icon_path = icon_path.to_string_lossy().replace('\\', "/");
    fs::write(&resource_path, format!("1 ICON \"{resource_icon_path}\"\n"))
        .expect("write generated desktop icon resource");

    embed_resource::compile_for(&resource_path, ["nxtools-desktop"], embed_resource::NONE)
        .manifest_optional()
        .expect("compile desktop icon resource");
}
