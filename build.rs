fn main() {
    pkg_config::Config::new()
        .atleast_version("3.8.7")
        .probe("sqlite3")
        .unwrap();

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .allowlist_function("sqlite3_aggregate_context")
        .allowlist_function("sqlite3_auto_extension")
        .allowlist_function("sqlite3_errstr")
        .allowlist_function("sqlite3_result_blob")
        .allowlist_function("sqlite3_result_double")
        .allowlist_function("sqlite3_result_error")
        .allowlist_function("sqlite3_result_error_nomem")
        .allowlist_function("sqlite3_value_blob")
        .allowlist_function("sqlite3_value_bytes")
        .allowlist_function("sqlite3_value_double")
        .allowlist_function("sqlite3_value_int64")
        .allowlist_function("sqlite3_value_text")
        .allowlist_function("sqlite3_value_type")
        .allowlist_type("sqlite3_context")
        .allowlist_var("SQLITE_BLOB")
        .allowlist_var("SQLITE_FLOAT")
        .allowlist_var("SQLITE_INTEGER")
        .allowlist_var("SQLITE_NULL")
        .allowlist_var("SQLITE_OK")
        .allowlist_var("SQLITE_TEXT")
        .generate()
        .expect("Unable to generate bindings");
    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    cc::Build::new().file("src/shim.c").compile("shim")
}
