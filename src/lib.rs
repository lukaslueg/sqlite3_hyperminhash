mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
use bindings::*;
use std::{error, ffi, fmt, io, mem, os::raw, slice};

use hyperminhash::Sketch;

#[cfg(feature = "serialize")]
pub mod serialize;

#[derive(Debug)]
enum HMHError {
    #[cfg(not(feature = "serialize"))]
    FeatureMissing,
    #[cfg(feature = "serialize")]
    ValueIsNotBlob,
    UnknownValueType,
    Io(io::Error),
}
impl HMHError {
    unsafe fn set_ctx<F: FnOnce() -> Result<(), Self>>(ctx: *mut sqlite3_context, f: F) {
        if let Err(e) = f() {
            let err_msg = e.to_string();
            sqlite3_result_error(
                ctx,
                err_msg.as_bytes().as_ptr() as *const raw::c_char,
                err_msg.as_bytes().len() as raw::c_int,
            );
        }
    }
}

impl fmt::Display for HMHError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            #[cfg(not(feature = "serialize"))]
            HMHError::FeatureMissing => write!(f, "This function is unavailable because sqlite3_hyperminhash was compiled without the `serialize`-feature."),
            #[cfg(feature = "serialize")]
            HMHError::ValueIsNotBlob => write!(f, "value is not of type BLOB"),
            HMHError::Io(e) => write!(f, "IO-error in hyperminhash: {}", e),
            HMHError::UnknownValueType => write!(f, "Unkown value-type from sqlite")
        }
    }
}

// TODO This bullshit can go away
impl error::Error for HMHError {}

impl From<io::Error> for HMHError {
    fn from(e: io::Error) -> Self {
        HMHError::Io(e)
    }
}

#[derive(Hash)]
enum RawValue<'a> {
    Null,
    Int(i64),
    Float(u64), // Bit-representation of a double to make it hashable
    Text(&'a str),
    Blob(&'a [u8]),
}

impl<'a> RawValue<'a> {
    unsafe fn new(value: *mut sqlite3_value) -> Result<Self, HMHError> {
        match sqlite3_value_type(value) as u32 {
            SQLITE_NULL => Ok(RawValue::Null),
            SQLITE_INTEGER => Ok(RawValue::Int(sqlite3_value_int64(value))),
            SQLITE_FLOAT => Ok(RawValue::Float({
                // Hold my beer and watch this!
                std::mem::transmute(sqlite3_value_double(value))
            })),
            SQLITE_TEXT => {
                let s = sqlite3_value_text(value);
                assert!(!s.is_null());
                let s = std::ffi::CStr::from_ptr(s as *const raw::c_char);
                // We explicitely told sqlite3 that we want UTF8-data in shim.c!
                Ok(RawValue::Text(std::str::from_utf8_unchecked(s.to_bytes())))
            }
            SQLITE_BLOB => {
                let blob = sqlite3_value_blob(value);
                let len = sqlite3_value_bytes(value);
                if len > 0 {
                    Ok(RawValue::Blob(std::slice::from_raw_parts(
                        blob as *const u8,
                        len as usize,
                    )))
                } else {
                    Ok(RawValue::Blob(&[]))
                }
            }
            _ => Err(HMHError::UnknownValueType),
        }
    }

    #[cfg(feature = "serialize")]
    fn as_blob(&self) -> Result<&[u8], HMHError> {
        match self {
            RawValue::Blob(b) => Ok(b),
            _ => Err(HMHError::ValueIsNotBlob),
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
    HMHError::set_ctx(ctx, || {
        let p = sqlite3_aggregate_context(ctx, mem::size_of::<*mut Sketch>() as raw::c_int)
            as *mut *mut Sketch;
        if p.is_null() {
            sqlite3_result_error_nomem(ctx);
            return Ok(());
        }
        if (*p).is_null() {
            *p = Box::into_raw(Box::new(Sketch::default()));
        }
        let sketch = &mut **p;
        let args: Result<Vec<_>, _> = slice::from_raw_parts(values, num_values as usize)
            .iter()
            .filter_map(|v| match RawValue::new(*v) {
                Ok(RawValue::Null) => None,
                other => Some(other),
            })
            .collect();
        sketch.add(args?);
        Ok(())
    })
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

#[cfg(not(feature = "serialize"))]
pub mod serialize_stub {
    use super::*;

    #[no_mangle]
    pub unsafe extern "C" fn hyperminhash_serialize_final(ctx: *mut sqlite3_context) {
        HMHError::set_ctx(ctx, || Err(HMHError::FeatureMissing))
    }

    macro_rules! no_such_func {
        ($name:ident) => {
            #[no_mangle]
            pub unsafe extern "C" fn $name(
                ctx: *mut sqlite3_context,
                _num_values: raw::c_int,
                _values: *mut *mut sqlite3_value,
            ) {
                HMHError::set_ctx(ctx, || Err(HMHError::FeatureMissing))
            }
        };
    }

    no_such_func!(hyperminhash_zero);
    no_such_func!(hyperminhash_deserialize);
    no_such_func!(hyperminhash_union);
    no_such_func!(hyperminhash_intersection);
}
