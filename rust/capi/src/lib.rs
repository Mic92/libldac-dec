//! C-ABI shim exposing the Rust LDAC decoder under the same symbol
//! names as the reference `libldacBT_dec.so`, so existing consumers
//! (PipeWire, BlueZ, etc.) can link against it unchanged.
//!
//! Only the decode-side `ldacBT_*` entry points are provided; the
//! lower-level `ldaclib_*` symbols are implementation details of the C
//! library and have no Rust equivalent.

#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_int, c_uchar, c_void};

use ldac_dec::{ChannelMode, LdacDecoder, LdacError, SampleFormat};

// Error codes from inc/ldacBT.h ------------------------------------------------
const LDACBT_ERR_NONE: c_int = 0;
const LDACBT_ERR_DEC_CONFIG_UPDATED: c_int = 40;
const LDACBT_ERR_FATAL: c_int = 256;
const LDACBT_ERR_SYNTAX_BAND: c_int = 260;
const LDACBT_ERR_SYNTAX_GRAD_A: c_int = 261;
const LDACBT_ERR_SYNTAX_GRAD_B: c_int = 262;
const LDACBT_ERR_SYNTAX_GRAD_C: c_int = 263;
const LDACBT_ERR_SYNTAX_GRAD_D: c_int = 264;
const LDACBT_ERR_SYNTAX_GRAD_E: c_int = 265;
const LDACBT_ERR_SYNTAX_IDSF: c_int = 266;
const LDACBT_ERR_SYNTAX_SPEC: c_int = 267;
const LDACBT_ERR_ILL_SYNCWORD: c_int = 516;
const LDACBT_ERR_ILL_SMPL_FORMAT: c_int = 517;
const LDACBT_ERR_ASSERT_SUP_SAMPLING_FREQ: c_int = 531;
const LDACBT_ERR_ASSERT_CHANNEL_CONFIG: c_int = 533;
const LDACBT_ERR_ASSERT_CHANNEL_MODE: c_int = 539;
const LDACBT_ERR_INPUT_BUFFER_SIZE: c_int = 571;
const LDACBT_ERR_UNPACK_BLOCK_FAILED: c_int = 572;
const LDACBT_ERR_UNPACK_BLOCK_ALIGN: c_int = 573;
const LDACBT_ERR_UNPACK_FRAME_ALIGN: c_int = 574;
const LDACBT_ERR_FRAME_LENGTH_OVER: c_int = 575;
const LDACBT_ERR_HANDLE_NOT_INIT: c_int = 1000;
const LDACBT_ERR_ILL_SAMPLING_FREQ: c_int = 1025;
const LDACBT_ERR_FATAL_HANDLE: c_int = LDACBT_ERR_FATAL;

// Channel mode IDs from inc/ldacBT.h
const LDACBT_CHANNEL_MODE_STEREO: c_int = 0x01;
const LDACBT_CHANNEL_MODE_DUAL_CHANNEL: c_int = 0x02;
const LDACBT_CHANNEL_MODE_MONO: c_int = 0x04;

// Sample format IDs from inc/ldacBT.h
const LDACBT_SMPL_FMT_S16: c_int = 0x2;
const LDACBT_SMPL_FMT_S24: c_int = 0x3;
const LDACBT_SMPL_FMT_S32: c_int = 0x4;
const LDACBT_SMPL_FMT_F32: c_int = 0x5;

// Version: match the C library so callers that gate on it behave the same.
const LDACBT_LIB_VER_MAJOR: c_int = 2;
const LDACBT_LIB_VER_MINOR: c_int = 0;
const LDACBT_LIB_VER_BRANCH: c_int = 33;

fn map_err(e: LdacError) -> c_int {
    use LdacError::*;
    match e {
        SyntaxBand => LDACBT_ERR_SYNTAX_BAND,
        SyntaxGradA => LDACBT_ERR_SYNTAX_GRAD_A,
        SyntaxGradB => LDACBT_ERR_SYNTAX_GRAD_B,
        SyntaxGradC => LDACBT_ERR_SYNTAX_GRAD_C,
        SyntaxGradD => LDACBT_ERR_SYNTAX_GRAD_D,
        SyntaxGradE => LDACBT_ERR_SYNTAX_GRAD_E,
        SyntaxIdsf => LDACBT_ERR_SYNTAX_IDSF,
        SyntaxSpec => LDACBT_ERR_SYNTAX_SPEC,
        UnpackBlockFailed => LDACBT_ERR_UNPACK_BLOCK_FAILED,
        UnpackBlockAlign => LDACBT_ERR_UNPACK_BLOCK_ALIGN,
        UnpackFrameAlign => LDACBT_ERR_UNPACK_FRAME_ALIGN,
        FrameLengthOver => LDACBT_ERR_FRAME_LENGTH_OVER,
        IllSyncword => LDACBT_ERR_ILL_SYNCWORD,
        IllSamplingFreq => LDACBT_ERR_ILL_SAMPLING_FREQ,
        AssertSupSamplingFreq => LDACBT_ERR_ASSERT_SUP_SAMPLING_FREQ,
        AssertChannelConfig => LDACBT_ERR_ASSERT_CHANNEL_CONFIG,
        InputBufferSize => LDACBT_ERR_INPUT_BUFFER_SIZE,
    }
}

/// Opaque handle returned to C callers.  Layout is private; callers only
/// ever see it through a `*mut c_void`.
struct Handle {
    dec: Option<LdacDecoder>,
    /// Composite error code as the C API packs it: `api_err<<20 | lib_err`.
    error_code: c_int,
    /// Last channel-mode hint from init, kept for the DEC_CONFIG_UPDATED
    /// signal when the stream header disagrees.
    init_cm: Option<ChannelMode>,
}

impl Handle {
    fn new() -> Self {
        Self {
            dec: None,
            error_code: LDACBT_ERR_NONE,
            init_cm: None,
        }
    }
    fn set_api_err(&mut self, e: c_int) {
        self.error_code = e << 20;
    }
    fn set_lib_err(&mut self, e: c_int) {
        self.error_code = (LDACBT_ERR_FATAL << 20) | e;
    }
}

// -----------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn ldacBT_get_version() -> c_int {
    (LDACBT_LIB_VER_MAJOR << 16) | (LDACBT_LIB_VER_MINOR << 8) | LDACBT_LIB_VER_BRANCH
}

#[no_mangle]
pub extern "C" fn ldacBT_get_handle() -> *mut c_void {
    Box::into_raw(Box::new(Handle::new())) as *mut c_void
}

/// # Safety: `h` must be null or a pointer previously returned from
/// `ldacBT_get_handle` that has not yet been freed.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_free_handle(h: *mut c_void) {
    if !h.is_null() {
        drop(Box::from_raw(h as *mut Handle));
    }
}

/// # Safety: `h` must be null or a valid handle.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_close_handle(h: *mut c_void) {
    if let Some(handle) = (h as *mut Handle).as_mut() {
        *handle = Handle::new();
    }
}

/// # Safety: `h` must be null or a valid handle.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_get_error_code(h: *mut c_void) -> c_int {
    match (h as *mut Handle).as_ref() {
        Some(handle) => handle.error_code,
        None => LDACBT_ERR_FATAL_HANDLE << 10,
    }
}

/// # Safety: `h` must be null or a valid handle.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_get_sampling_freq(h: *mut c_void) -> c_int {
    match (h as *mut Handle).as_mut() {
        Some(handle) => match &handle.dec {
            Some(d) => d.sample_rate() as c_int,
            None => {
                handle.set_api_err(LDACBT_ERR_HANDLE_NOT_INIT);
                -1
            }
        },
        None => -1,
    }
}

/// # Safety: `h` must be null or a valid handle.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_get_bitrate(h: *mut c_void) -> c_int {
    match (h as *mut Handle).as_mut() {
        Some(handle) => match &handle.dec {
            Some(d) => d.bitrate() as c_int,
            None => {
                handle.set_api_err(LDACBT_ERR_HANDLE_NOT_INIT);
                -1
            }
        },
        None => -1,
    }
}

/// # Safety: `h` must be null or a valid handle.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_init_handle_decode(
    h: *mut c_void,
    cm: c_int,
    sf: c_int,
    _nshift: c_int,
    _var0: c_int,
    _var1: c_int,
) -> c_int {
    let Some(handle) = (h as *mut Handle).as_mut() else {
        return -1;
    };
    let cm = match cm {
        LDACBT_CHANNEL_MODE_STEREO => ChannelMode::Stereo,
        LDACBT_CHANNEL_MODE_DUAL_CHANNEL => ChannelMode::DualChannel,
        LDACBT_CHANNEL_MODE_MONO => ChannelMode::Mono,
        _ => {
            handle.set_api_err(LDACBT_ERR_ASSERT_CHANNEL_MODE);
            return -1;
        }
    };
    let sf = if sf < 0 { 0 } else { sf as u32 };
    match LdacDecoder::new(cm, sf) {
        Ok(d) => {
            handle.dec = Some(d);
            handle.init_cm = Some(cm);
            handle.error_code = LDACBT_ERR_NONE;
            0
        }
        Err(e) => {
            handle.set_api_err(map_err(e));
            -1
        }
    }
}

/// # Safety: `h` must be a valid handle; `p_bs` must point to at least
/// `bs_bytes` readable bytes (plus a two-byte look-ahead margin, as the
/// C API documents); `p_pcm` must point to a writable buffer large
/// enough for one decoded frame; `used_bytes`/`wrote_bytes` must be
/// valid non-null pointers.
#[no_mangle]
pub unsafe extern "C" fn ldacBT_decode(
    h: *mut c_void,
    p_bs: *const c_uchar,
    p_pcm: *mut c_uchar,
    fmt: c_int,
    bs_bytes: c_int,
    used_bytes: *mut c_int,
    wrote_bytes: *mut c_int,
) -> c_int {
    *used_bytes = 0;
    *wrote_bytes = 0;

    let Some(handle) = (h as *mut Handle).as_mut() else {
        return -1;
    };
    let Some(dec) = handle.dec.as_mut() else {
        handle.set_api_err(LDACBT_ERR_HANDLE_NOT_INIT);
        return -1;
    };
    if p_bs.is_null() || p_pcm.is_null() || bs_bytes < 0 {
        handle.set_api_err(LDACBT_ERR_FATAL);
        return -1;
    }

    let fmt = match fmt {
        LDACBT_SMPL_FMT_S16 => SampleFormat::S16,
        LDACBT_SMPL_FMT_S24 => SampleFormat::S24,
        LDACBT_SMPL_FMT_S32 => SampleFormat::S32,
        LDACBT_SMPL_FMT_F32 => SampleFormat::F32,
        _ => {
            handle.set_api_err(LDACBT_ERR_ILL_SMPL_FORMAT);
            return -1;
        }
    };

    let bs = std::slice::from_raw_parts(p_bs, bs_bytes as usize);
    // Maximum possible output: 256 samples × 2 ch × 4 bytes.
    const MAX_OUT: usize = 256 * 2 * 4;
    let pcm = std::slice::from_raw_parts_mut(p_pcm, MAX_OUT);

    // Detect stream re-configuration so callers see DEC_CONFIG_UPDATED
    // before we rebuild the decoder (matches C behaviour).
    let prev_sr = dec.sample_rate();
    let prev_ch = dec.channels();

    match dec.decode(bs, fmt, pcm) {
        Ok((used, wrote)) => {
            *used_bytes = used as c_int;
            *wrote_bytes = wrote as c_int;
            if dec.sample_rate() != prev_sr || dec.channels() != prev_ch {
                handle.set_api_err(LDACBT_ERR_DEC_CONFIG_UPDATED);
            } else {
                handle.error_code = LDACBT_ERR_NONE;
            }
            0
        }
        Err(e) => {
            handle.set_lib_err(map_err(e));
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn null_handle_is_harmless() {
        unsafe {
            ldacBT_free_handle(ptr::null_mut());
            ldacBT_close_handle(ptr::null_mut());
            assert_eq!(ldacBT_get_sampling_freq(ptr::null_mut()), -1);
            assert_eq!(ldacBT_get_bitrate(ptr::null_mut()), -1);
            assert_eq!(
                ldacBT_init_handle_decode(ptr::null_mut(), 1, 48000, 0, 0, 0),
                -1
            );
            // C API returns FATAL_HANDLE<<10 on null.
            assert_eq!(
                ldacBT_get_error_code(ptr::null_mut()),
                LDACBT_ERR_FATAL_HANDLE << 10
            );
        }
    }

    #[test]
    fn handle_lifecycle_no_leak() {
        // Miri tracks this Box allocation and would flag a leak if
        // free_handle forgot to drop it.
        let h = ldacBT_get_handle();
        assert!(!h.is_null());
        unsafe {
            assert_eq!(
                ldacBT_init_handle_decode(h, LDACBT_CHANNEL_MODE_STEREO, 48_000, 0, 0, 0),
                0
            );
            assert_eq!(ldacBT_get_sampling_freq(h), 48_000);
            assert_eq!(ldacBT_get_error_code(h), LDACBT_ERR_NONE);
            ldacBT_free_handle(h);
        }
    }

    #[test]
    fn close_then_reinit() {
        let h = ldacBT_get_handle();
        unsafe {
            ldacBT_init_handle_decode(h, LDACBT_CHANNEL_MODE_MONO, 44_100, 0, 0, 0);
            ldacBT_close_handle(h);
            // After close the decoder is gone; queries report not-init.
            assert_eq!(ldacBT_get_sampling_freq(h), -1);
            // Re-init on the same handle must work.
            assert_eq!(
                ldacBT_init_handle_decode(h, LDACBT_CHANNEL_MODE_STEREO, 96_000, 0, 0, 0),
                0
            );
            assert_eq!(ldacBT_get_sampling_freq(h), 96_000);
            ldacBT_free_handle(h);
        }
    }

    #[test]
    fn decode_garbage_is_safe() {
        let h = ldacBT_get_handle();
        unsafe {
            ldacBT_init_handle_decode(h, LDACBT_CHANNEL_MODE_STEREO, 48_000, 0, 0, 0);
            let bs = [0u8; 32]; // no syncword
            let mut pcm = [0u8; 256 * 2 * 4];
            let (mut used, mut wrote) = (0, 0);
            let r = ldacBT_decode(
                h,
                bs.as_ptr(),
                pcm.as_mut_ptr(),
                LDACBT_SMPL_FMT_S16,
                bs.len() as c_int,
                &mut used,
                &mut wrote,
            );
            assert_eq!(r, -1);
            assert_eq!(used, 0);
            assert_eq!(wrote, 0);
            assert_ne!(ldacBT_get_error_code(h), LDACBT_ERR_NONE);
            ldacBT_free_handle(h);
        }
    }

    #[test]
    fn bad_init_params_rejected() {
        let h = ldacBT_get_handle();
        unsafe {
            // Invalid channel mode.
            assert_eq!(ldacBT_init_handle_decode(h, 0x99, 48_000, 0, 0, 0), -1);
            // Invalid sample rate.
            assert_eq!(
                ldacBT_init_handle_decode(h, LDACBT_CHANNEL_MODE_STEREO, 12_345, 0, 0, 0),
                -1
            );
            ldacBT_free_handle(h);
        }
    }

    #[test]
    fn repeated_init_no_leak() {
        // Each init replaces the Option<LdacDecoder>; Miri would flag
        // if the old Box<AcSub> inside weren't dropped.
        let h = ldacBT_get_handle();
        unsafe {
            for _ in 0..10 {
                assert_eq!(
                    ldacBT_init_handle_decode(h, LDACBT_CHANNEL_MODE_STEREO, 48_000, 0, 0, 0),
                    0
                );
            }
            ldacBT_free_handle(h);
        }
    }
}
