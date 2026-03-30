//! Differential test: decode the same LDAC bitstream with the C
//! reference library and the Rust port and assert bit-exact output.
//!
//! Skipped (passes trivially) when no `tests/fixtures/*.ldac` files are
//! present or when the C shared library cannot be found.
//!
//! To enable: build the C decoder (`cmake -B build && make -C build
//! ldacBT_dec`) and drop raw LDAC transport-frame dumps into
//! `rust/tests/fixtures/`.

use std::ffi::c_int;
use std::path::Path;

use ldac_dec::{ChannelMode, LdacDecoder, SampleFormat};

#[test]
fn diff_against_c_s16() {
    diff_against_c_fmt(SampleFormat::S16, 2, 2);
}

#[test]
fn diff_against_c_s24() {
    diff_against_c_fmt(SampleFormat::S24, 3, 3);
}

#[test]
fn diff_against_c_s32() {
    diff_against_c_fmt(SampleFormat::S32, 4, 4);
}

#[test]
fn diff_against_c_f32() {
    diff_against_c_fmt(SampleFormat::F32, 5, 4);
}

fn diff_against_c_fmt(rfmt: SampleFormat, cfmt: c_int, wl: usize) {
    let fixtures: Vec<_> = std::fs::read_dir("tests/fixtures")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "ldac"))
        .map(|e| e.path())
        .collect();
    if fixtures.is_empty() {
        eprintln!("diff_against_c: no fixtures, skipping");
        return;
    }

    let lib_path = std::env::var("LDAC_C_REFERENCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| Path::new("../build/libldacBT_dec.so").into());
    if !lib_path.exists() {
        eprintln!("diff_against_c: C library not built, skipping");
        return;
    }
    let cdec = CDecoder::load(&lib_path);

    for fx in fixtures {
        eprintln!("fixture: {}", fx.display());
        let stream = std::fs::read(&fx).unwrap();

        // Derive sample rate + channel mode from the first frame header.
        let sr_id = (stream[1] >> 5) & 7;
        let cc_id = (stream[1] >> 3) & 3;
        let sr = [44_100, 48_000, 88_200, 96_000][sr_id as usize];
        let (ccm, rcm, nch) = match cc_id {
            0 => (0x04, ChannelMode::Mono, 1),
            1 => (0x02, ChannelMode::DualChannel, 2),
            _ => (0x01, ChannelMode::Stereo, 2),
        };

        // C side.  The reference ldacBT_init_handle_decode calls
        // set_config_info with unset fields (a benign bug); the per-frame
        // decode re-reads config from the header, so ignore the return code.
        let h = cdec.get_handle();
        let _ = cdec.init(h, ccm, sr as c_int);

        // Rust side
        let mut rdec = LdacDecoder::new(rcm, sr).unwrap();

        let mut off = 0usize;
        let mut cout = vec![0u8; 256 * nch * wl];
        let mut rout = vec![0u8; 256 * nch * wl];
        let mut frame = 0;

        while off + 5 < stream.len() {
            let (cused, cwrote) = cdec.decode(h, &stream[off..], &mut cout, cfmt);
            let (rused, rwrote) = rdec.decode(&stream[off..], rfmt, &mut rout).unwrap();

            assert_eq!(cused, rused, "frame {frame}: used bytes mismatch");
            assert_eq!(cwrote, rwrote, "frame {frame}: wrote bytes mismatch");
            assert_eq!(
                &cout[..cwrote],
                &rout[..rwrote],
                "frame {frame}: PCM data mismatch"
            );
            off += rused;
            frame += 1;
        }
        cdec.free(h);
        eprintln!("  OK: {frame} frames identical");
    }
}

// ---- minimal dlopen wrapper (avoids extra deps) ---------------------------

use std::ffi::{c_char, c_void, CString};

#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flag: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

type FnGetHandle = extern "C" fn() -> *mut c_void;
type FnFree = extern "C" fn(*mut c_void);
type FnInit = extern "C" fn(*mut c_void, c_int, c_int, c_int, c_int, c_int) -> c_int;
type FnDecode =
    extern "C" fn(*mut c_void, *const u8, *mut u8, c_int, c_int, *mut c_int, *mut c_int) -> c_int;

#[allow(dead_code)]
type FnErr = extern "C" fn(*mut c_void) -> c_int;

#[allow(dead_code)]
struct CDecoder {
    get_handle: FnGetHandle,
    free_handle: FnFree,
    init_decode: FnInit,
    decode: FnDecode,
    error_code: FnErr,
}

impl CDecoder {
    fn load(p: &Path) -> Self {
        let cp = CString::new(p.to_str().unwrap()).unwrap();
        let h = unsafe {
            dlopen(cp.as_ptr(), 2 /* RTLD_NOW */)
        };
        assert!(!h.is_null(), "dlopen failed");
        let sym = |s: &str| {
            let cs = CString::new(s).unwrap();
            let p = unsafe { dlsym(h, cs.as_ptr()) };
            assert!(!p.is_null(), "missing symbol {s}");
            p
        };
        unsafe {
            Self {
                get_handle: std::mem::transmute::<*mut c_void, FnGetHandle>(sym(
                    "ldacBT_get_handle",
                )),
                free_handle: std::mem::transmute::<*mut c_void, FnFree>(sym("ldacBT_free_handle")),
                init_decode: std::mem::transmute::<*mut c_void, FnInit>(sym(
                    "ldacBT_init_handle_decode",
                )),
                decode: std::mem::transmute::<*mut c_void, FnDecode>(sym("ldacBT_decode")),
                error_code: std::mem::transmute::<*mut c_void, FnErr>(sym("ldacBT_get_error_code")),
            }
        }
    }
    fn get_handle(&self) -> *mut c_void {
        (self.get_handle)()
    }
    fn free(&self, h: *mut c_void) {
        (self.free_handle)(h)
    }
    #[allow(dead_code)]
    fn error(&self, h: *mut c_void) -> c_int {
        (self.error_code)(h)
    }
    fn init(&self, h: *mut c_void, cm: c_int, sf: c_int) -> c_int {
        (self.init_decode)(h, cm, sf, 0, 0, 0)
    }
    fn decode(&self, h: *mut c_void, bs: &[u8], out: &mut [u8], fmt: c_int) -> (usize, usize) {
        let mut used = 0;
        let mut wrote = 0;
        (self.decode)(
            h,
            bs.as_ptr(),
            out.as_mut_ptr(),
            fmt,
            bs.len() as c_int,
            &mut used,
            &mut wrote,
        );
        (used as usize, wrote as usize)
    }
}
