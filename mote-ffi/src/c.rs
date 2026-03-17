//! Foreign function interface for C

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use mote_api::{MoteLink, messages::host_to_mote};

pub struct MoteLinkHandle {
    inner: MoteLink,
}

/// Create a new MoteLink handle. The returned pointer must be freed with `mote_link_free`.
#[unsafe(no_mangle)]
pub extern "C" fn mote_link_new() -> *mut MoteLinkHandle {
    Box::into_raw(Box::new(MoteLinkHandle {
        inner: MoteLink::new(),
    }))
}

/// Free a MoteLink handle.
///
/// # Safety
/// `handle` must be a valid pointer returned by `mote_link_new` and must not be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mote_link_free(handle: *mut MoteLinkHandle) {
    if !handle.is_null() {
        unsafe { drop(Box::from_raw(handle)) };
    }
}

/// Queue a JSON-encoded host-to-mote message for transmission.
///
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `handle` must be a valid non-null pointer. `json_message` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mote_link_send(
    handle: *mut MoteLinkHandle,
    json_message: *const c_char,
) -> c_int {
    let handle = unsafe { &mut *handle };
    let json = match unsafe { CStr::from_ptr(json_message) }.to_str() {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let msg: host_to_mote::Message = match serde_json::from_str(json) {
        Ok(m) => m,
        Err(_) => return -1,
    };
    match handle.inner.send(msg) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Copy the next transmit packet into `buf`.
///
/// Returns the number of bytes written, 0 if there is no packet to transmit,
/// or -1 if `buf` is too small.
///
/// # Safety
/// `handle` must be a valid non-null pointer. `buf` must point to a writable buffer of at least `buf_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mote_link_poll_transmit(
    handle: *mut MoteLinkHandle,
    buf: *mut u8,
    buf_len: c_int,
) -> c_int {
    let handle = unsafe { &mut *handle };
    match handle.inner.poll_transmit() {
        Some(bytes) => {
            if bytes.len() > buf_len as usize {
                return -1;
            }
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf, bytes.len()) };
            bytes.len() as c_int
        }
        None => 0,
    }
}

/// Feed a received packet into the link.
///
/// Returns 0 on success.
///
/// # Safety
/// `handle` must be a valid non-null pointer. `buf` must point to a readable buffer of at least `buf_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mote_link_handle_receive(
    handle: *mut MoteLinkHandle,
    buf: *const u8,
    buf_len: c_int,
) -> c_int {
    let handle = unsafe { &mut *handle };
    let bytes = unsafe { std::slice::from_raw_parts(buf, buf_len as usize) };
    handle.inner.handle_receive(bytes);
    0
}

/// Copy the next decoded mote-to-host message as a null-terminated JSON string into `buf`.
///
/// Returns the number of bytes written including the null terminator, 0 if no message is ready,
/// or -1 on error or if `buf` is too small.
///
/// # Safety
/// `handle` must be a valid non-null pointer. `buf` must point to a writable buffer of at least `buf_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn mote_link_poll_receive(
    handle: *mut MoteLinkHandle,
    buf: *mut c_char,
    buf_len: c_int,
) -> c_int {
    let handle = unsafe { &mut *handle };
    match handle.inner.poll_receive() {
        Ok(Some(msg)) => {
            let json = match serde_json::to_string(&msg) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let cstr = match CString::new(json) {
                Ok(s) => s,
                Err(_) => return -1,
            };
            let bytes = cstr.as_bytes_with_nul();
            if bytes.len() > buf_len as usize {
                return -1;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const c_char, buf, bytes.len())
            };
            bytes.len() as c_int
        }
        Ok(None) => 0,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mote_api::{HostLink, messages::mote_to_host};

    unsafe fn send_ping(handle: *mut MoteLinkHandle) {
        let json = c"\"Ping\"";
        let ret = unsafe { mote_link_send(handle, json.as_ptr()) };
        assert_eq!(ret, 0);
    }

    #[test]
    fn test_c_ffi_new_and_free() {
        unsafe {
            let handle = mote_link_new();
            assert!(!handle.is_null());
            mote_link_free(handle);
        }
    }

    #[test]
    fn test_c_ffi_send_and_poll_transmit() {
        unsafe {
            let handle = mote_link_new();
            send_ping(handle);

            let mut buf = [0u8; 256];
            let n = mote_link_poll_transmit(handle, buf.as_mut_ptr(), buf.len() as c_int);
            assert!(n > 0);

            let n2 = mote_link_poll_transmit(handle, buf.as_mut_ptr(), buf.len() as c_int);
            assert_eq!(n2, 0);

            mote_link_free(handle);
        }
    }

    #[test]
    fn test_c_ffi_poll_transmit_buffer_too_small() {
        unsafe {
            let handle = mote_link_new();
            send_ping(handle);

            let mut buf = [0u8; 1];
            let n = mote_link_poll_transmit(handle, buf.as_mut_ptr(), buf.len() as c_int);
            assert_eq!(n, -1);

            mote_link_free(handle);
        }
    }

    #[test]
    fn test_c_ffi_send_invalid_json() {
        unsafe {
            let handle = mote_link_new();
            let bad = c"not valid json";
            let ret = mote_link_send(handle, bad.as_ptr());
            assert_eq!(ret, -1);
            mote_link_free(handle);
        }
    }

    #[test]
    fn test_c_ffi_poll_receive_empty() {
        unsafe {
            let handle = mote_link_new();
            let mut buf = [0i8; 256];
            let n = mote_link_poll_receive(handle, buf.as_mut_ptr(), buf.len() as c_int);
            assert_eq!(n, 0);
            mote_link_free(handle);
        }
    }

    #[test]
    fn test_c_ffi_round_trip() {
        unsafe {
            let handle = mote_link_new();
            send_ping(handle);

            let mut packet_buf = [0u8; 4096];
            let n =
                mote_link_poll_transmit(handle, packet_buf.as_mut_ptr(), packet_buf.len() as c_int);
            assert!(n > 0);

            let mut mote = HostLink::new();
            mote.handle_receive(&packet_buf[..n as usize]);
            let received = mote.poll_receive().unwrap().unwrap();
            assert_eq!(received, host_to_mote::Message::Ping);

            mote_link_free(handle);
        }
    }

    #[test]
    fn test_c_ffi_handle_receive_and_poll_receive() {
        unsafe {
            let mut mote = HostLink::new();
            mote.send(mote_to_host::Message::Pong).unwrap();
            let payload = mote.poll_transmit().unwrap();

            let handle = mote_link_new();
            let ret = mote_link_handle_receive(handle, payload.as_ptr(), payload.len() as c_int);
            assert_eq!(ret, 0);

            let mut buf = [0i8; 256];
            let n = mote_link_poll_receive(handle, buf.as_mut_ptr(), buf.len() as c_int);
            assert!(n > 0);

            let json = CStr::from_ptr(buf.as_ptr()).to_str().unwrap();
            let msg: mote_to_host::Message = serde_json::from_str(json).unwrap();
            assert_eq!(msg, mote_to_host::Message::Pong);

            mote_link_free(handle);
        }
    }
}
