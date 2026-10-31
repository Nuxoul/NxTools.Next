use std::{env, fs, path::PathBuf};

// Stable RT_GROUP_ICON IDs: GPUI/tray/shortcuts use 1; archive DefaultIcon uses -2.
// Embed both in the already signed/approved Desktop: no unsigned runtime sibling.
const BRAND_ICONS: &[(u16, &str)] = &[
    (1, "assets/brand/nxtools.ico"),
    (2, "assets/brand/nxtools-archive.ico"),
];

fn main() {
    for (_, path) in BRAND_ICONS {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-changed=build.rs");

    if env::var_os("CARGO_CFG_TARGET_OS").as_deref() != Some(std::ffi::OsStr::new("windows")) {
        return;
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo must provide OUT_DIR"));
    let resource_path = output.join("nxtools-icon.rc");
    let manifest =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("Cargo manifest directory"));
    let mut resource = String::new();
    for (id, path) in BRAND_ICONS {
        let icon_path = manifest.join(path);
        assert!(icon_path.is_file(), "missing approved icon {path}");
        let resource_icon_path = icon_path.to_string_lossy().replace('\\', "/");
        resource.push_str(&format!("{id} ICON \"{resource_icon_path}\"\n"));
    }
    fs::write(&resource_path, resource).expect("write desktop icon resources");

    embed_resource::compile_for(&resource_path, ["nxtools-desktop"], embed_resource::NONE)
        .manifest_optional()
        .expect("compile desktop icon resource");
}
