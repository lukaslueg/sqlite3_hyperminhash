use std::{ffi, mem, os::raw, slice};

use super::bindings::*;
use super::{HMHError, RawValue, Sketch};

unsafe extern "C" fn drop_blob_buffer<T>(buf: *mut ffi::c_void) {
    drop(Box::<T>::from_raw(buf as *mut _))
}

unsafe fn set_blob_result<T>(ctx: *mut sqlite3_context, value: Box<T>) {
    let buf_len = std::mem::size_of_val(&*value);
    let p = Box::into_raw(value) as *const ffi::c_void;
    sqlite3_result_blob(ctx, p, buf_len as i32, Some(drop_blob_buffer::<T>));
}

unsafe fn sketch_to_result<'a>(
    sk: &Sketch,
    ctx: &'a *mut sqlite3_context,
) -> Result<(), HMHError<'a>> {
    let mut buf = Box::new([0; 32768]);
    sk.save(&mut buf[..])?;
    set_blob_result(*ctx, buf);
    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_zero(
    ctx: *mut sqlite3_context,
    _num_values: raw::c_int,
    _values: *mut *mut sqlite3_value,
) {
    HMHError::set_ctx(ctx, || sketch_to_result(&Sketch::default(), &ctx));
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_serialize_final(ctx: *mut sqlite3_context) {
    HMHError::set_ctx(ctx, || {
        let p = sqlite3_aggregate_context(ctx, 0) as *mut *mut Sketch;
        let sketch = if p.is_null() {
            Box::new(Sketch::default())
        } else {
            Box::from_raw(*p)
        };
        sketch_to_result(&sketch, &ctx)
    })
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
pub unsafe extern "C" fn hyperminhash_add(
    ctx: *mut sqlite3_context,
    num_values: raw::c_int,
    values: *mut *mut sqlite3_value,
) {
    HMHError::set_ctx(ctx, || {
        let args = slice::from_raw_parts(values, num_values as usize);
        let sum_sketch = args
            .iter()
            .map(|p| {
                RawValue::new(*p)
                    .and_then(|v| v.as_blob())
                    .and_then(|b| Sketch::load(b).map_err(Into::into))
            })
            .fold(None, |sk1: Option<Result<_, HMHError>>, sk2| {
                match (sk1, sk2) {
                    (None, Ok(sk2)) => Some(Ok(sk2.clone())),
                    (None, Err(e)) | (Some(Err(e)), _) | (Some(Ok(_)), Err(e)) => Some(Err(e)),
                    (Some(Ok(mut sk1)), Ok(sk2)) => {
                        sk1.union(&sk2);
                        Some(Ok(sk1))
                    }
                }
            })
            .transpose()?
            .unwrap_or_default();
        sketch_to_result(&sum_sketch, &ctx)?;
        Ok(())
    });
}

#[no_mangle]
pub unsafe extern "C" fn hyperminhash_union_step(
    ctx: *mut sqlite3_context,
    num_values: raw::c_int,
    values: *mut *mut sqlite3_value,
) {
    assert!(num_values == 1); // Declared as such in shim.c
    HMHError::set_ctx(ctx, || {
        let sketch = Sketch::load(RawValue::new(*values)?.as_blob()?)?;

        let p = sqlite3_aggregate_context(ctx, mem::size_of::<*mut Sketch>() as raw::c_int)
            as *mut *mut Sketch;
        if p.is_null() {
            sqlite3_result_error_nomem(ctx);
            return Ok(());
        }
        if (*p).is_null() {
            *p = Box::into_raw(Box::new(sketch.clone()));
        } else {
            let running_sketch = &mut **p;
            running_sketch.union(&sketch);
        }
        Ok(())
    })
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
