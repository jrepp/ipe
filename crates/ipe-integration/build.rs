fn main() {
    println!("cargo:rerun-if-changed=adapters/servicenow-metadata.wat");
    let bytes = wat::parse_file("adapters/servicenow-metadata.wat").expect("valid bundled adapter");
    std::fs::write(
        std::path::Path::new(&std::env::var_os("OUT_DIR").unwrap()).join("metadata.wasm"),
        bytes,
    )
    .unwrap();
}
