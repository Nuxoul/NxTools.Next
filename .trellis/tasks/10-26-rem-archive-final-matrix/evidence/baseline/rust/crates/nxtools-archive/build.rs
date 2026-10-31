fn main() {
    // Bundled UnRAR uses registry/security/crypto APIs but unrar_sys0.5.8 omits
    // advapi32 on MSVC. Link only for the optional isolated worker codec build.
    if std::env::var_os("CARGO_FEATURE_ISOLATED_CODECS").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
    {
        println!("cargo:rustc-link-lib=advapi32");
    }
}
