fn main() {
    // `include_dir!` embeds the ros_model directory at macro-expansion time but, on
    // stable Rust, does not register per-file cargo dependencies for it. Without this,
    // adding/editing a `ros_model/*.txt` file silently does not trigger a rebuild of
    // this crate (and therefore not of anything using `known_entities()`), so `cargo
    // check`/`build` can succeed against stale generated code.
    println!("cargo:rerun-if-changed=ros_model");
}
