fn main() {
    generate_schemas();
    #[cfg(feature = "cbindgen")]
    generate_header();
}

fn generate_schemas() {
    use mote_api::messages::{host_to_mote, mote_to_host};
    use schemars::schema_for;
    use std::path::PathBuf;

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let schemas_dir = PathBuf::from(&crate_dir).join("schemas");
    std::fs::create_dir_all(&schemas_dir).unwrap();

    let host_to_mote = serde_json::to_string_pretty(&schema_for!(host_to_mote::Message)).unwrap();
    let mote_to_host = serde_json::to_string_pretty(&schema_for!(mote_to_host::Message)).unwrap();

    std::fs::write(schemas_dir.join("host_to_mote.json"), host_to_mote).unwrap();
    std::fs::write(schemas_dir.join("mote_to_host.json"), mote_to_host).unwrap();

    println!("cargo:rerun-if-changed=../mote-api/src/messages");
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
