/* Generate an LDAC bitstream fixture for the Rust diff test.
 *
 * Links against nixpkgs#ldacbt's libldacBT_enc.so and encodes a short
 * 48 kHz stereo sine sweep to tests/fixtures/sine48k.ldac.
 */
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "ldacBT.h"

static void gen(const char* path, int eqmid, int sr) {
    HANDLE_LDAC_BT h = ldacBT_get_handle();
    int r = ldacBT_init_handle_encode(
        h, 679, eqmid, LDACBT_CHANNEL_MODE_STEREO,
        LDACBT_SMPL_FMT_S16, sr);
    if (r) { fprintf(stderr, "init failed: %x\n",
                     ldacBT_get_error_code(h)); exit(1); }

    FILE* out = fopen(path, "wb");
    short pcm[128 * 2];
    unsigned char stream[1024];
    int used, wrote, nfrm;
    double phase = 0, freq = 440.0;

    /* ~1 second of sine sweep */
    for (int frm = 0; frm < 400; ++frm) {
        for (int i = 0; i < 128; ++i) {
            double s = sin(phase) * 16000.0;
            phase += 2.0 * M_PI * freq / (double)sr;
            pcm[2*i] = pcm[2*i+1] = (short)s;
        }
        freq *= 1.005;
        r = ldacBT_encode(h, pcm, &used, stream, &wrote, &nfrm);
        if (r) { fprintf(stderr, "encode failed\n"); exit(1); }
        if (wrote) fwrite(stream, 1, wrote, out);
    }
    /* flush */
    ldacBT_encode(h, NULL, &used, stream, &wrote, &nfrm);
    if (wrote) fwrite(stream, 1, wrote, out);

    fclose(out);
    ldacBT_free_handle(h);
}

int main(void) {
    gen("sine48k_hq.ldac", LDACBT_EQMID_HQ, 48000);
    gen("sine48k_sq.ldac", LDACBT_EQMID_SQ, 48000);
    gen("sine48k_mq.ldac", LDACBT_EQMID_MQ, 48000);
    gen("sine96k_hq.ldac", LDACBT_EQMID_HQ, 96000);
    return 0;
}
