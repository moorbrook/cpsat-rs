use crate::error::SolveError;
use std::ptr;

// OR-Tools C API (`ortools/sat/c_api/cp_solver_c.cc`) allocates `cres` with C `malloc`.
// Pair every call with `free` via `MallocGuard` on all exit paths.
unsafe extern "C" {
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

struct MallocGuard(*mut u8);

impl Drop for MallocGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is non-null (checked above), was returned by
            // `SolveCpModelWithParameters` which allocates the output buffer via
            // C `malloc` (verified in OR-Tools `ortools/sat/c_api/cp_solver_c.cc`),
            // and `MallocGuard` uniquely owns it with no surviving aliases.
            unsafe {
                free(self.0.cast::<std::ffi::c_void>());
            }
        }
    }
}

fn usize_to_i32(val: usize) -> Result<i32, SolveError> {
    i32::try_from(val).map_err(|_| SolveError::ModelTooLarge(val))
}

fn response_len_to_usize(response_len: i32) -> Result<usize, SolveError> {
    usize::try_from(response_len).map_err(|_| SolveError::FfiError(response_len))
}

pub(crate) fn solve_raw(
    model_bytes: &[u8],
    params_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, SolveError> {
    let model_len = usize_to_i32(model_bytes.len())?;

    let (params_ptr, params_len) = match params_bytes {
        Some(p) => (p.as_ptr(), usize_to_i32(p.len())?),
        None => (ptr::null(), 0),
    };

    let mut response_buf: *mut u8 = ptr::null_mut();
    let mut response_len: i32 = 0;

    // SAFETY: `model_bytes.as_ptr()` is valid for reading `model_len` bytes
    // (bounds-checked by `usize_to_i32`). `params_ptr` is valid for reading
    // `params_len` bytes when `Some`, or null with length `0` when `None`.
    // `response_buf` and `response_len` are valid stack-local out-pointers
    // created via `ptr::addr_of_mut!`, and OR-Tools does not retain any input
    // pointer after returning.
    unsafe {
        SolveCpModelWithParameters(
            model_bytes.as_ptr(),
            model_len,
            params_ptr,
            params_len,
            ptr::addr_of_mut!(response_buf),
            ptr::addr_of_mut!(response_len),
        );
    }

    let _response_guard = MallocGuard(response_buf);
    if response_buf.is_null() {
        return Err(SolveError::FfiError(-1));
    }

    let len = response_len_to_usize(response_len)?;
    if len == 0 {
        return Ok(Vec::new());
    }

    // SAFETY: `response_buf` is non-null (checked above), `len > 0` and
    // `len <= i32::MAX <= isize::MAX` (validated by `response_len_to_usize`),
    // OR-Tools populated `len` initialized `u8` bytes in the `malloc` buffer,
    // and `_response_guard` keeps the buffer live until `to_vec()` completes.
    let slice = unsafe { std::slice::from_raw_parts(response_buf, len) };
    Ok(slice.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_response_len_returns_ffi_error() {
        assert!(matches!(
            response_len_to_usize(-1),
            Err(SolveError::FfiError(-1))
        ));
        assert!(matches!(
            response_len_to_usize(i32::MIN),
            Err(SolveError::FfiError(i32::MIN))
        ));
        assert_eq!(response_len_to_usize(0).unwrap(), 0);
        assert_eq!(response_len_to_usize(42).unwrap(), 42);
    }

    #[test]
    fn usize_to_i32_overflow_returns_model_too_large() {
        let too_large = usize::try_from(i32::MAX).unwrap() + 1;
        assert!(matches!(
            usize_to_i32(too_large),
            Err(SolveError::ModelTooLarge(v)) if v == too_large
        ));
        assert_eq!(usize_to_i32(128).unwrap(), 128);
    }
}
