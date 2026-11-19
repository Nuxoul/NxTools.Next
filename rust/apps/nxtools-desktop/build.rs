use std::{env, fs, path::PathBuf, process::Command};

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
    emit_build_metadata();

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
    let version = env::var("CARGO_PKG_VERSION").expect("Cargo package version");
    let major = env::var("CARGO_PKG_VERSION_MAJOR").expect("Cargo major version");
    let minor = env::var("CARGO_PKG_VERSION_MINOR").expect("Cargo minor version");
    let patch = env::var("CARGO_PKG_VERSION_PATCH").expect("Cargo patch version");
    resource.push_str(&format!(
        r#"
1 VERSIONINFO
FILEVERSION {major},{minor},{patch},0
PRODUCTVERSION {major},{minor},{patch},0
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x40004L
FILETYPE 0x1L
FILESUBTYPE 0
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "CompanyName", "NxTools"
            VALUE "FileDescription", "NxTools"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "NxTools"
            VALUE "OriginalFilename", "NxTools.exe"
            VALUE "ProductName", "NxTools"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    ));
    fs::write(&resource_path, resource).expect("write desktop icon resources");

    embed_resource::compile_for(&resource_path, ["nxtools-desktop"], embed_resource::NONE)
        .manifest_optional()
        .expect("compile desktop icon resource");
}

fn emit_build_metadata() {
    fn git(args: &[&str]) -> Option<String> {
        let output = Command::new("git").args(args).output().ok()?;
        output.status.success().then_some(())?;
        Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
    }

    println!("cargo:rerun-if-env-changed=RUSTC");
    if let Some(target) = env::var_os("TARGET") {
        println!(
            "cargo:rustc-env=NXTOOLS_BUILD_TARGET={}",
            target.to_string_lossy()
        );
    }
    if let Some(output) = env::var_os("RUSTC")
        .and_then(|rustc| Command::new(rustc).arg("--version").output().ok())
        .filter(|output| output.status.success())
    {
        println!(
            "cargo:rustc-env=NXTOOLS_RUSTC={}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
    }

    if let Some(head) = git(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"])
        .and_then(|reference| git(&["rev-parse", "--git-path", &reference]))
        .filter(|reference| std::path::Path::new(reference).exists())
    {
        println!("cargo:rerun-if-changed={reference}");
    }
    if let Some(commit) = git(&["rev-parse", "HEAD"]) {
        println!("cargo:rustc-env=NXTOOLS_BUILD_COMMIT={commit}");
    }
}
