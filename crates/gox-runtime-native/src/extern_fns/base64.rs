//! encoding/base64 package C ABI for JIT.

use gox_runtime_core::builtins::base64 as core;
use gox_runtime_core::gc::{Gc, GcRef, TypeId};
use gox_runtime_core::objects::{array, slice, string};
use gox_common_core::ValueKind;

/// Helper to get bytes from a byte slice GcRef (packed storage)
unsafe fn slice_to_bytes(s: GcRef) -> &'static [u8] {
    let len = slice::len(s);
    let arr = slice::array_ref(s);
    let start = slice::start(s);
    let ptr = array::as_bytes(arr).add(start);
    std::slice::from_raw_parts(ptr, len)
}

/// Helper to create a byte slice from Rust bytes (packed storage)
unsafe fn bytes_to_slice(gc: &mut Gc, bytes: &[u8]) -> GcRef {
    let len = bytes.len();
    if len == 0 {
        return std::ptr::null_mut();
    }
    
    let arr = array::create(gc, ValueKind::Array as TypeId, ValueKind::Uint8 as TypeId, 1, len);
    let dest = array::as_bytes_mut(arr);
    std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, len);
    slice::create(gc, ValueKind::Slice as TypeId, arr, 0, len, len)
}

#[no_mangle]
pub unsafe extern "C" fn gox_base64_std_encode(gc: *mut Gc, src: GcRef, type_id: TypeId) -> GcRef {
    let src_bytes = slice_to_bytes(src);
    let result = core::encode_std(src_bytes);
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[no_mangle]
pub unsafe extern "C" fn gox_base64_std_decode(
    gc: *mut Gc, s: GcRef, str_type_id: TypeId
) -> (GcRef, GcRef) {
    match core::decode_std(string::as_str(s)) {
        Ok(bytes) => {
            let slice = bytes_to_slice(&mut *gc, &bytes);
            (slice, std::ptr::null_mut())
        }
        Err(e) => {
            (std::ptr::null_mut(), string::from_rust_str(&mut *gc, str_type_id, &e))
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn gox_base64_raw_encode(gc: *mut Gc, src: GcRef, type_id: TypeId) -> GcRef {
    let src_bytes = slice_to_bytes(src);
    let result = core::encode_std_no_pad(src_bytes);
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[no_mangle]
pub unsafe extern "C" fn gox_base64_url_encode(gc: *mut Gc, src: GcRef, type_id: TypeId) -> GcRef {
    let src_bytes = slice_to_bytes(src);
    let result = core::encode_url(src_bytes);
    string::from_rust_str(&mut *gc, type_id, &result)
}

#[no_mangle]
pub unsafe extern "C" fn gox_base64_url_decode(
    gc: *mut Gc, s: GcRef, str_type_id: TypeId
) -> (GcRef, GcRef) {
    match core::decode_url(string::as_str(s)) {
        Ok(bytes) => {
            let slice = bytes_to_slice(&mut *gc, &bytes);
            (slice, std::ptr::null_mut())
        }
        Err(e) => {
            (std::ptr::null_mut(), string::from_rust_str(&mut *gc, str_type_id, &e))
        }
    }
}
