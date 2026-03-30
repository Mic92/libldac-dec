# ldac-dec (Rust)

Memory-safe Rust rewrite of the LDAC Bluetooth audio decoder.

LDAC is a lossy audio codec developed by Sony for high-resolution audio
streaming over Bluetooth A2DP (44.1 – 96 kHz, up to ~990 kbit/s).

## Status

* `#![forbid(unsafe_code)]` – **zero `unsafe`** in the core decoder.
* Bit-exact against the C reference for all test vectors
  (stereo HQ/SQ/MQ @ 48 kHz, HQ @ 96 kHz, mono, dual-channel)
  across all four output formats.
* All sample rates (44.1/48/88.2/96 kHz) and channel modes
  (mono / dual / stereo) supported.
* Output formats: S16, S24, S32, F32 little-endian PCM.
* Optional C-ABI shim builds a drop-in `libldacBT_dec.so`.

## Usage

### As a Rust library

Add to `Cargo.toml`:

```toml
[dependencies]
ldac-dec = { path = "path/to/libldac-dec/rust" }
```

```rust
use ldac_dec::{ChannelMode, LdacDecoder, SampleFormat};

let mut dec = LdacDecoder::new(ChannelMode::Stereo, 48_000)?;
let mut pcm = vec![0u8; dec.frame_samples() as usize
                        * dec.channels() as usize
                        * SampleFormat::S16.word_len()];

// Feed one LDAC transport frame at a time (3-byte header + payload).
// The input slice should have ~2 bytes of slack past the frame end
// for look-ahead reads (matching the C API contract).
let (used, wrote) = dec.decode(&stream, SampleFormat::S16, &mut pcm)?;
// pcm[..wrote] now holds interleaved little-endian S16 samples.
// Advance `stream` by `used` bytes for the next frame.
```

Query decoder state between frames:

```rust
dec.sample_rate()    // Hz
dec.channels()       // 1 or 2
dec.frame_samples()  // samples per channel per frame (128 or 256)
dec.bitrate()        // kbit/s of the last decoded frame
```

### As a C library (drop-in `libldacBT_dec.so` replacement)

The `capi/` sub-crate exposes the exact `ldacBT_*` C API from
`inc/ldacBT.h`, so existing consumers (PipeWire, BlueZ, PulseAudio
LDAC modules) can link against it unchanged:

```bash
cargo build --release -p ldac-dec-capi
# → target/release/libldacBT_dec.so  (cdylib)
# → target/release/libldacBT_dec.a   (staticlib)
```

Install alongside the upstream header:

```bash
install -Dm755 target/release/libldacBT_dec.so "$PREFIX/lib/libldacBT_dec.so"
install -Dm644 ../inc/ldacBT.h                  "$PREFIX/include/ldac/ldacBT.h"
```

Or quick-test with an existing binary via `LD_PRELOAD`:

```bash
LD_PRELOAD=./target/release/libldacBT_dec.so pipewire
```

Exported symbols (decode-side only, matches the reference shared object):

```
ldacBT_get_handle        ldacBT_free_handle         ldacBT_close_handle
ldacBT_get_version       ldacBT_get_error_code      ldacBT_get_sampling_freq
ldacBT_get_bitrate       ldacBT_init_handle_decode  ldacBT_decode
```

The shim reports version `2.0.33` and the same `LDACBT_ERR_*` integer
codes as the C library, so callers that gate on either behave
identically.  The lower-level `ldaclib_*` symbols are intentionally
not exported — they are implementation details of the C library, not
part of the public `ldacBT.h` contract.

## Building & testing

### With Nix

```bash
nix build .#ldac-dec
# → result/lib/libldacBT_dec.{so,a}
# → result/include/ldac/ldacBT.h
# → result/bin/ldac-dec-test
```

The flake runs the full test suite in the sandbox, including the C-ABI
diff test (against a nix-built C reference) and valgrind.  The
`devShell` builds the C reference on entry so `cargo test` picks it up:

```bash
nix develop
cargo test --workspace
```

Override the C-reference location via `LDAC_C_REFERENCE`:

```bash
LDAC_C_REFERENCE=/path/to/libldacBT_dec.so cargo test
```

### Without Nix

```bash
cargo build --release --workspace
cargo test  --workspace
```

Test suites:

* `tests/basic.rs` — API smoke tests.
* `tests/security.rs` — regression PoCs for OOB reads fixed in the port.
* `tests/diff_against_c.rs` — dlopens `../build/libldacBT_dec.so` and
  asserts bit-exact output for every `.ldac` file in `tests/fixtures/`,
  one test per output format (S16/S24/S32/F32).
* `capi/tests/abi_compat.rs` — dlopens *both* the C reference and the
  Rust-built `libldacBT_dec.so`, drives them through identical C-ABI
  calls, and asserts bit-identical PCM plus matching bitrate/version.
* `capi/tests/valgrind.rs` — runs a C harness (built via the `cc`
  crate) under valgrind against the Rust cdylib, exercising full
  decode loops, repeated re-init, close/reopen and error paths.
  Asserts `ERROR SUMMARY: 0 errors` and zero leaks.

The diff and valgrind tests skip gracefully when their prerequisites
(C reference library / valgrind) are not available.
To enable them:

```bash
cmake -B build && make -C build ldacBT_dec
```

Generate fresh fixtures (requires `libldacBT_enc` from nixpkgs or AOSP):

```bash
(cd tests/fixtures && \
  gcc -I../../../inc make_fixture.c -lldacBT_enc -lm -o make_fixture && \
  ./make_fixture)
```

## Architecture

The pointer-heavy C AB/AC graph is flattened into owned, index-addressed
Rust vectors inside `SfInfo`, eliminating aliasing and lifetime
headaches.  Large numeric tables (IMDCT window / trig / permutation) are
generated from the vetted C source at build time via `build.rs` to avoid
transcription errors.

| Module           | Purpose                                  |
|------------------|------------------------------------------|
| `bitreader`      | Bounds-checked big-endian bit reader     |
| `unpack`         | Frame-header & raw-data-frame parser     |
| `bitalloc`       | Gradient / word-length reconstruction    |
| `dequant`        | Inverse quantisation                     |
| `imdct`          | Inverse MDCT + overlap-add window        |
| `setpcm`         | Float → S16/S24/S32/F32 conversion       |
| `tables*`        | Static lookup tables                     |
| `capi/`          | C-ABI shim (`cdylib` + `staticlib`)      |

The core crate stays `#![forbid(unsafe_code)]`; only the thin `capi/`
FFI layer uses `unsafe` for raw-pointer marshalling.
