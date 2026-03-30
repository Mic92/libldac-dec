# ldac-dec (Rust)

Memory-safe Rust rewrite of the LDAC Bluetooth audio decoder.

LDAC is a lossy audio codec developed by Sony for high-resolution audio
streaming over Bluetooth A2DP (44.1 – 96 kHz, up to ~990 kbit/s).

## Status

* `#![forbid(unsafe_code)]` – **zero `unsafe`** in the decoder.
* Bit-exact against the C reference for all test vectors
  (HQ/SQ/MQ @ 48 kHz, HQ @ 96 kHz).
* All sample rates (44.1/48/88.2/96 kHz) and channel modes
  (mono / dual / stereo) supported.
* Output formats: S16, S24, S32, F32 little-endian PCM.

## Example

```rust
use ldac_dec::{ChannelMode, LdacDecoder, SampleFormat};

let mut dec = LdacDecoder::new(ChannelMode::Stereo, 48_000)?;
let mut pcm = vec![0u8; dec.frame_samples() as usize * 2 * 2];
let (used, wrote) = dec.decode(&transport_frame, SampleFormat::S16, &mut pcm)?;
```

## Building & testing

```bash
cargo build --release
cargo test        # runs unit + diff tests
```

The diff test (`tests/diff_against_c.rs`) dlopens `../build/libldacBT_dec.so`
and asserts bit-exact output for every `.ldac` file found in
`tests/fixtures/`.  Generate fresh fixtures with:

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
