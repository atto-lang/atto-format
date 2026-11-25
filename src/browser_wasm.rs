//! A bounded, dependency-free C ABI for the canonical Rust formatter.
//! wasm32-unknown-unknown only; no JS glue, WASI, file/network imports or source telemetry.
//!
//! All pointers are to exported linear memory. Caller allocates source/output
//! with oresfmt_alloc and frees both with oresfmt_free using original byte sizes.
//! oresfmt_format returns output byte length >=0 for success, or -(error_len+1)
//! for a diagnostic stored as UTF-8 in the output buffer. i32::MIN means an
//! invalid buffer, input or output size. The formatter is the actual crate API.
use crate::format_source;
use std::ptr;
const MAX_SOURCE: usize = 65_536;
const MAX_OUTPUT: usize = 262_144;
#[unsafe(no_mangle)]
pub extern "C" fn oresfmt_alloc(length: usize) -> *mut u8 {
    if length == 0 || length > MAX_OUTPUT {
        return ptr::null_mut();
    }
    Box::into_raw(vec![0u8; length].into_boxed_slice()) as *mut u8
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oresfmt_free(ptr: *mut u8, length: usize) {
    if ptr.is_null() || length == 0 || length > MAX_OUTPUT {
        return;
    }
    // SAFETY: pointer and length must be the matching values from oresfmt_alloc.
    unsafe {
        drop(Box::from_raw(ptr::slice_from_raw_parts_mut(ptr, length)));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn oresfmt_format(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_cap: usize,
) -> i32 {
    if input_ptr.is_null()
        || output_ptr.is_null()
        || input_len == 0
        || input_len > MAX_SOURCE
        || output_cap == 0
        || output_cap > MAX_OUTPUT
    {
        return i32::MIN;
    }
    // SAFETY: pointers are caller-provided allocations in the same linear
    // memory, sized to at least input_len / output_cap respectively.
    let source = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let result = match std::str::from_utf8(source) {
        Ok(text) => format_source(text).map_err(|error| error.to_string()),
        Err(error) => Err(format!("Invalid UTF-8: {error}")),
    };
    let (output, is_error) = match &result {
        Ok(formatted) => (formatted.as_bytes(), false),
        Err(message) => (message.as_bytes(), true),
    };
    if output.len() > output_cap || output.len() > MAX_OUTPUT {
        return i32::MIN;
    }
    // SAFETY: destination is output_cap bytes, separate from the input allocation.
    unsafe {
        ptr::copy_nonoverlapping(output.as_ptr(), output_ptr, output.len());
    }
    if is_error {
        -(output.len() as i32) - 1
    } else {
        output.len() as i32
    }
}
