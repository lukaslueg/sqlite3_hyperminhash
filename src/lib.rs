mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;
use std::ffi;
use std::mem;
use std::os::raw;
use std::slice;

use hyperminhash::Sketch;

#[derive(Hash)]
enum RawValue<'a> {
    Null,
    Int(i64),
    Float(u64), // Bit-representation of a double to make it hashable
    Text(&'a str),
    Blob(&'a [u8]),
}

impl<'a> RawValue<'a> {
    unsafe fn new(value: *mut sqlite3_value) -> Self {
        match sqlite3_value_type(value) as u32 {
            SQLITE_NULL => RawValue::Null,
            SQLITE_INTEGER => RawValue::Int(sqlite3_value_int64(value)),
            SQLITE_FLOAT => RawValue::Float({
                // Hold my beer and watch this!
                std::mem::transmute(sqlite3_value_double(value))
            }),
            SQLITE_TEXT => {
                let s = sqlite3_value_text(value);
                assert!(!s.is_null());
                let s = std::ffi::CStr::from_ptr(s as *const raw::c_char);
                // We explicitely told sqlite3 that we want UTF8-data in shim.c!
                RawValue::Text(std::str::from_utf8_unchecked(s.to_bytes()))
            }
            SQLITE_BLOB => {
                let blob = sqlite3_value_blob(value);
                let len = sqlite3_value_bytes(value);
                if len > 0 {
                    RawValue::Blob(std::slice::from_raw_parts(blob as *const u8, len as usize))
                } else {
                    RawValue::Blob(&[])
                }
            }
            _ => panic!("Unknown return value from sqlite3_value_type"),
        }
    }
}

/// Used by tests to auto-load itself into sqlite
#[doc(hidden)]
pub mod testutil {
    pub use super::bindings::sqlite3_auto_extension;
    pub use super::bindings::SQLITE_OK;
}

/// Initialization shim provided by shim.c
extern "C" {
    pub fn init_shim(
        db: *mut ffi::c_void,
        pzErrMsg: *const ffi::c_void,
        pApi: *const ffi::c_void,
    ) -> raw::c_int;
}

/// Public initialization function, called by sqlite
#[no_mangle]
#[allow(non_snake_case)]
pub unsafe extern "C" fn sqlite3_sqlitehyperminhash_init(
    db: *mut ffi::c_void,
    pzErrMsg: *const ffi::c_void,
    pApi: *const ffi::c_void,
) -> raw::c_int {
    init_shim(db, pzErrMsg, pApi)
}

/// The step-function, called for each row
#[no_mangle]
pub unsafe extern "C" fn hyperminhash_step(
    ctx: *mut sqlite3_context,
    num_values: raw::c_int,
    values: *mut *mut sqlite3_value,
) {
    // The pointer to a Box<Sketch> is stored in the context
    let p = sqlite3_aggregate_context(ctx, mem::size_of::<*mut Sketch>() as raw::c_int) as *mut *mut Sketch;
    if p.is_null() {
        sqlite3_result_error_nomem(ctx);
        return;
    }
    if (*p).is_null() {
        *p = Box::into_raw(Box::new(Sketch::default()));
    }
    let sketch = &mut **p;
    let args: Vec<_> = slice::from_raw_parts(values, num_values as usize)
        .iter()
        .filter_map(|v| match RawValue::new(*v) {
            RawValue::Null => None,
            other => Some(other),
        })
        .collect();
    sketch.add(args);
}

/// Finalize the aggregate by computing the cardinality
#[no_mangle]
pub unsafe extern "C" fn hyperminhash_final(ctx: *mut sqlite3_context) {
    let p = sqlite3_aggregate_context(ctx, 0) as *mut *mut Sketch;
    if p.is_null() {
        sqlite3_result_double(ctx, 0.0);
        return;
    }
    sqlite3_result_double(ctx, Box::from_raw(*p).cardinality());
}
