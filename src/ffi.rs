//! The single unsafe FFI bridge to OR-Tools CP-SAT.
//!
//! Uses the official C API from `ortools/sat/c_api/cp_solver_c.h`.
//! This module contains the ONLY unsafe code in the crate.

use crate::error::SolveError;
use std::ptr;

extern "C" {
    fn SolveCpModelWithParameters(
        creq: *const u8,
        creq_len: i32,
        cparams: *const u8,
        cparams_len: i32,
        cres: *mut *mut u8,
        cres_len: *mut i32,
    );

    fn free(ptr: *mut std::ffi::c_void);
}

/// Serialize a model and parameters, call OR-Tools, return response bytes.
///
/// # Errors
///
/// Returns `SolveError::FfiError` if the C API returns a null response buffer.
pub(crate) fn solve_raw(
    model_bytes: &[u8],
    params_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, SolveError> {
    let mut response_buf: *mut u8 = ptr::null_mut();
    let mut response_len: i32 = 0;

    let (params_ptr, params_len) = match params_bytes {
        Some(p) => (p.as_ptr(), p.len() as i32),
        None => (ptr::null(), 0),
    };

    // SAFETY: All pointers are valid for the duration of the call.
    // The C API allocates the response buffer which we must free.
    unsafe {
        SolveCpModelWithParameters(
            model_bytes.as_ptr(),
            model_bytes.len() as i32,
            params_ptr,
            params_len,
            ptr::addr_of_mut!(response_buf),
            ptr::addr_of_mut!(response_len),
        );
    }

    if response_buf.is_null() || response_len <= 0 {
        return Err(SolveError::FfiError(-1));
    }

    // SAFETY: C++ allocated response_buf with malloc, length is response_len.
    // We copy into a Vec immediately and free the C++ buffer.
    let response = unsafe {
        let slice = std::slice::from_raw_parts(response_buf, response_len as usize);
        let owned = slice.to_vec();
        free(response_buf.cast::<std::ffi::c_void>());
        owned
    };

    Ok(response)
}
