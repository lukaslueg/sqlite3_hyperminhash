static AUTOLOAD: std::sync::Once = std::sync::Once::new();

pub fn init_db() -> rusqlite::Result<rusqlite::Connection> {
    AUTOLOAD.call_once(|| {
        // https://sqlite.org/c3ref/auto_extension.html
        let ptr = sqlite3_hyperminhash::sqlite3_sqlitehyperminhash_init
            as unsafe extern "C" fn(
                *mut std::ffi::c_void,
                *const std::ffi::c_void,
                *const std::ffi::c_void,
            ) -> i32;
        let rc = unsafe {
            sqlite3_hyperminhash::testutil::sqlite3_auto_extension(Some(std::mem::transmute(ptr)))
        };
        if rc as u32 != sqlite3_hyperminhash::testutil::SQLITE_OK {
            panic!("sqlite3_auto_extension failed");
        }
    });
    rusqlite::Connection::open_in_memory()
}
