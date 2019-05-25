fn main() {
    println!("cargo:rustc-link-lib=sqlite3");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .whitelist_function("sqlite3_aggregate_context")
        .whitelist_function("sqlite3_auto_extension")
        .whitelist_function("sqlite3_result_blob")
        .whitelist_function("sqlite3_result_double")
        .whitelist_function("sqlite3_result_error")
        .whitelist_function("sqlite3_result_error_nomem")
        .whitelist_function("sqlite3_value_blob")
        .whitelist_function("sqlite3_value_bytes")
        .whitelist_function("sqlite3_value_double")
        .whitelist_function("sqlite3_value_int64")
        .whitelist_function("sqlite3_value_text")
        .whitelist_function("sqlite3_value_type")
        .whitelist_type("sqlite3_context")
        .whitelist_var("SQLITE_BLOB")
        .whitelist_var("SQLITE_FLOAT")
        .whitelist_var("SQLITE_INTEGER")
        .whitelist_var("SQLITE_NULL")
        .whitelist_var("SQLITE_OK")
        .whitelist_var("SQLITE_TEXT")
        .generate()
        .expect("Unable to generate bindings");
    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    cc::Build::new().file("src/shim.c").compile("shim")
}
