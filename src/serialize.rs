use std::ffi;
use std::os::raw;
use std::slice;

use super::bindings::*;
use super::{HMHError, RawValue, Sketch};

pub unsafe extern "C" fn drop_blob_buffer(buf: *mut ffi::c_void) {
    drop(Box::from_raw(buf))
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_zero(
    ctx: *mut sqlite3_context,
    _num_values: raw::c_int,
    _values: *mut *mut sqlite3_value,
) {
    HMHError::set_ctx(ctx, || {
        let mut buf = Vec::with_capacity(32768);
        Sketch::default().save(&mut buf).unwrap();
        let buf_len = buf.len();
        let buf = Box::into_raw(buf.into_boxed_slice()) as *const ffi::c_void;
        sqlite3_result_blob(ctx, buf, buf_len as i32, Some(drop_blob_buffer));
        Ok(())
    });
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_serialize_final(ctx: *mut sqlite3_context) {
    let p = sqlite3_aggregate_context(ctx, 0) as *mut *mut Sketch;
    let sketch = if p.is_null() {
        Box::new(Sketch::default())
    } else {
        Box::from_raw(*p)
    };
    let mut buf = Vec::with_capacity(32768);
    sketch.save(&mut buf).unwrap();
    let buf_len = buf.len();
    let buf = Box::into_raw(buf.into_boxed_slice()) as *const ffi::c_void;
    sqlite3_result_blob(ctx, buf, buf_len as i32, Some(drop_blob_buffer));
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_deserialize(
    ctx: *mut sqlite3_context,
    num_values: raw::c_int,
    values: *mut *mut sqlite3_value,
) {
    assert!(num_values == 1); // Declared as such in shim.c
    HMHError::set_ctx(ctx, || {
        let sk = Sketch::load(RawValue::new(*values)?.as_blob()?)?;
        sqlite3_result_double(ctx, sk.cardinality());
        Ok(())
    });
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_union(
    ctx: *mut sqlite3_context,
    num_values: raw::c_int,
    values: *mut *mut sqlite3_value,
) {
    assert!(num_values == 2); // Declared as such in shim.c
    HMHError::set_ctx(ctx, || {
        let args = slice::from_raw_parts(values, num_values as usize);
        let mut sketch1 = Sketch::load(RawValue::new(args[0])?.as_blob()?)?;
        let sketch2 = Sketch::load(RawValue::new(args[1])?.as_blob()?)?;
        sketch1.union(&sketch2);

        let mut buf = Vec::with_capacity(32768);
        sketch1.save(&mut buf)?;
        let buf_len = buf.len();
        let buf = Box::into_raw(buf.into_boxed_slice()) as *const ffi::c_void;
        sqlite3_result_blob(ctx, buf, buf_len as i32, Some(drop_blob_buffer));
        Ok(())
    });
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_intersection(
    ctx: *mut sqlite3_context,
    num_values: raw::c_int,
    values: *mut *mut sqlite3_value,
) {
    assert!(num_values == 2); // Declared as such in shim.c
    HMHError::set_ctx(ctx, || {
        let args = slice::from_raw_parts(values, num_values as usize);
        let sketch1 = Sketch::load(RawValue::new(args[0])?.as_blob()?)?;
        let sketch2 = Sketch::load(RawValue::new(args[1])?.as_blob()?)?;

        sqlite3_result_double(ctx, sketch1.intersection(&sketch2));
        Ok(())
    });
}
