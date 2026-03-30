/* Valgrind/ASan harness: dlopen the Rust-built libldacBT_dec.so and
 * run a full init→decode→free cycle over every .ldac fixture, twice,
 * plus stress patterns (re-init, close/reopen, error paths).
 *
 * Intended to be run under valgrind --leak-check=full to prove the
 * C-ABI surface neither leaks nor reads/writes out of bounds across
 * the language boundary.
 */
#define _GNU_SOURCE
#include <assert.h>
#include <dirent.h>
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef void* H;
typedef H (*FnGet)(void);
typedef void (*FnFree)(H);
typedef int (*FnInit)(H, int, int, int, int, int);
typedef int (*FnDec)(H, unsigned char*, unsigned char*, int, int, int*, int*);
typedef int (*FnInt)(H);

static FnGet  f_get;
static FnFree f_free, f_close;
static FnInit f_init;
static FnDec  f_decode;
static FnInt  f_err, f_sf, f_br;

static void decode_file(const char* path) {
  FILE* fp = fopen(path, "rb");
  assert(fp);
  fseek(fp, 0, SEEK_END);
  long sz = ftell(fp);
  fseek(fp, 0, SEEK_SET);
  unsigned char* buf = malloc(sz + 4); /* +look-ahead pad */
  fread(buf, 1, sz, fp);
  memset(buf + sz, 0, 4);
  fclose(fp);

  int sr_id = (buf[1] >> 5) & 7;
  int cc_id = (buf[1] >> 3) & 3;
  int sr = (int[]){44100, 48000, 88200, 96000}[sr_id];
  int cm = (int[]){0x04, 0x02, 0x01, 0x01}[cc_id];

  H h = f_get();
  assert(h);
  int r = f_init(h, cm, sr, 0, 0, 0);
  assert(r == 0);
  assert(f_sf(h) == sr);

  unsigned char pcm[256 * 2 * 4];
  long off = 0;
  int frames = 0;
  while (off + 5 < sz) {
    int used = 0, wrote = 0;
    r = f_decode(h, buf + off, pcm, 2, (int)(sz - off), &used, &wrote);
    if (r != 0 || used == 0) break;
    off += used;
    frames++;
  }
  (void)f_br(h);
  (void)f_err(h);
  f_free(h);
  free(buf);
  fprintf(stderr, "  %s: %d frames\n", path, frames);
}

int main(int argc, char** argv) {
  assert(argc >= 2 && "usage: harness <libldacBT_dec.so> [fixture_dir]");
  void* lib = dlopen(argv[1], RTLD_NOW);
  if (!lib) { fprintf(stderr, "dlopen: %s\n", dlerror()); return 1; }
  #define S(n) ({ void* p = dlsym(lib, n); assert(p); p; })
  f_get    = (FnGet)  S("ldacBT_get_handle");
  f_free   = (FnFree) S("ldacBT_free_handle");
  f_close  = (FnFree) S("ldacBT_close_handle");
  f_init   = (FnInit) S("ldacBT_init_handle_decode");
  f_decode = (FnDec)  S("ldacBT_decode");
  f_err    = (FnInt)  S("ldacBT_get_error_code");
  f_sf     = (FnInt)  S("ldacBT_get_sampling_freq");
  f_br     = (FnInt)  S("ldacBT_get_bitrate");
  #undef S

  /* Stress: null safety */
  f_free(NULL);
  f_close(NULL);
  (void)f_err(NULL);

  /* Stress: repeated init on one handle (old decoder must be dropped) */
  H h = f_get();
  for (int i = 0; i < 20; i++) assert(f_init(h, 0x01, 48000, 0, 0, 0) == 0);
  f_close(h);
  assert(f_init(h, 0x04, 96000, 0, 0, 0) == 0);
  f_free(h);

  /* Stress: error path does not leak */
  h = f_get();
  assert(f_init(h, 0x99, 48000, 0, 0, 0) == -1);  /* bad cm */
  assert(f_init(h, 0x01, 12345, 0, 0, 0) == -1);  /* bad sr */
  f_free(h);

  /* Decode every fixture, twice */
  const char* dir = argc > 2 ? argv[2] : "../tests/fixtures";
  DIR* d = opendir(dir);
  assert(d);
  struct dirent* e;
  for (int pass = 0; pass < 2; pass++) {
    rewinddir(d);
    while ((e = readdir(d))) {
      if (!strstr(e->d_name, ".ldac")) continue;
      char p[512];
      snprintf(p, sizeof p, "%s/%s", dir, e->d_name);
      decode_file(p);
    }
  }
  closedir(d);

  dlclose(lib);
  fprintf(stderr, "harness OK\n");
  return 0;
}
