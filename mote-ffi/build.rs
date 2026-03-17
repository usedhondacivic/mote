fn main() {
    #[cfg(feature = "cbindgen")]
    generate_header();
}

#[cfg(feature = "cbindgen")]
fn generate_header() {
    use std::path::PathBuf;

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = PathBuf::from(&crate_dir)
        .join("include")
        .join("mote_link.h");

    std::fs::create_dir_all(out.parent().unwrap()).unwrap();

    let config = cbindgen::Config::from_file(format!("{crate_dir}/cbindgen.toml"))
        .expect("Failed to read cbindgen.toml");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(config)
        .with_language(cbindgen::Language::C)
        .with_include_guard("MOTE_LINK_H")
        .with_cpp_compat(true)
        .with_parse_deps(false)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(out);
}
