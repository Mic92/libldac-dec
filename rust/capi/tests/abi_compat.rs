//! Dogfood the C ABI: dlopen the Rust-built `libldacBT_dec.so` and the
//! C reference side-by-side, drive both through the identical `ldacBT_*`
//! entry points, and assert bit-exact output.
//!
//! This proves the Rust shim is a real drop-in for the C shared object.

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};

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
type FnInt = extern "C" fn(*mut c_void) -> c_int;
type FnVer = extern "C" fn() -> c_int;

struct Lib {
    get_handle: FnGetHandle,
    free_handle: FnFree,
    init_decode: FnInit,
    decode: FnDecode,
    get_version: FnVer,
    get_sf: FnInt,
    get_bitrate: FnInt,
}

impl Lib {
    fn load(p: &Path) -> Self {
        let cp = CString::new(p.to_str().unwrap()).unwrap();
        let h = unsafe { dlopen(cp.as_ptr(), 2) };
        assert!(!h.is_null(), "dlopen {} failed", p.display());
        let sym = |s: &str| {
            let cs = CString::new(s).unwrap();
            let p = unsafe { dlsym(h, cs.as_ptr()) };
            assert!(!p.is_null(), "missing symbol {s}");
            p
        };
        unsafe {
            Self {
                get_handle: std::mem::transmute::<_, FnGetHandle>(sym("ldacBT_get_handle")),
                free_handle: std::mem::transmute::<_, FnFree>(sym("ldacBT_free_handle")),
                init_decode: std::mem::transmute::<_, FnInit>(sym("ldacBT_init_handle_decode")),
                decode: std::mem::transmute::<_, FnDecode>(sym("ldacBT_decode")),
                get_version: std::mem::transmute::<_, FnVer>(sym("ldacBT_get_version")),
                get_sf: std::mem::transmute::<_, FnInt>(sym("ldacBT_get_sampling_freq")),
                get_bitrate: std::mem::transmute::<_, FnInt>(sym("ldacBT_get_bitrate")),
            }
        }
    }
}

fn find_rust_cdylib() -> Option<PathBuf> {
    // cargo builds cdylibs next to the test binary's deps dir.
    let exe = std::env::current_exe().ok()?;
    let deps = exe.parent()?;
    let profile = deps.parent()?;
    for dir in [profile, deps] {
        let cand = dir.join("libldacBT_dec.so");
        if cand.exists() {
            return Some(cand);
        }
    }
    None
}

#[test]
fn abi_compat() {
    let c_path = Path::new("../../build/libldacBT_dec.so");
    if !c_path.exists() {
        eprintln!("abi_compat: C library not built, skipping");
        return;
    }
    let Some(rust_path) = find_rust_cdylib() else {
        panic!("rust cdylib not found; run `cargo build -p ldac-dec-capi` first");
    };

    let clib = Lib::load(c_path);
    let rlib = Lib::load(&rust_path);

    assert_eq!(
        (clib.get_version)(),
        (rlib.get_version)(),
        "version mismatch"
    );

    let fixtures: Vec<_> = std::fs::read_dir("../tests/fixtures")
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "ldac"))
        .map(|e| e.path())
        .collect();
    assert!(!fixtures.is_empty(), "no .ldac fixtures");

    for fx in &fixtures {
        eprintln!("fixture: {}", fx.display());
        let stream = std::fs::read(fx).unwrap();
        let sr_id = (stream[1] >> 5) & 7;
        let cc_id = (stream[1] >> 3) & 3;
        let sr = [44_100, 48_000, 88_200, 96_000][sr_id as usize];
        let cm = [0x04, 0x02, 0x01, 0x01][cc_id as usize];

        let ch = (clib.get_handle)();
        let rh = (rlib.get_handle)();
        (clib.init_decode)(ch, cm, sr, 0, 0, 0);
        assert_eq!((rlib.init_decode)(rh, cm, sr, 0, 0, 0), 0);

        assert_eq!((clib.get_sf)(ch), sr);
        assert_eq!((rlib.get_sf)(rh), sr);

        let mut cout = vec![0u8; 256 * 2 * 4];
        let mut rout = vec![0u8; 256 * 2 * 4];
        let mut off = 0usize;
        let mut frames = 0;

        while off + 5 < stream.len() {
            let (mut cu, mut cw, mut ru, mut rw) = (0, 0, 0, 0);
            let rem = (stream.len() - off) as c_int;
            (clib.decode)(
                ch,
                stream[off..].as_ptr(),
                cout.as_mut_ptr(),
                2,
                rem,
                &mut cu,
                &mut cw,
            );
            (rlib.decode)(
                rh,
                stream[off..].as_ptr(),
                rout.as_mut_ptr(),
                2,
                rem,
                &mut ru,
                &mut rw,
            );
            assert_eq!(cu, ru, "frame {frames}: used bytes");
            assert_eq!(cw, rw, "frame {frames}: wrote bytes");
            assert_eq!(
                &cout[..cw as usize],
                &rout[..rw as usize],
                "frame {frames}: PCM mismatch"
            );
            off += ru as usize;
            frames += 1;
        }
        assert_eq!((clib.get_bitrate)(ch), (rlib.get_bitrate)(rh), "bitrate");
        (clib.free_handle)(ch);
        (rlib.free_handle)(rh);
        eprintln!("  OK: {frames} frames identical via C ABI");
    }
}
