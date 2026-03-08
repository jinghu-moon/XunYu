#include <avif/avif.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

#ifdef _WIN32
#define XUN_API __declspec(dllexport)
#else
#define XUN_API
#endif

typedef struct {
    uint8_t *pixels;
    uint32_t width;
    uint32_t height;
    uint32_t stride;
} XunAvifImage;

static int env_debug_enabled(void) {
    const char *v = getenv("XUN_AVIF_C_DEBUG");
    return (v && v[0] != '\0');
}

static void maybe_log_image_meta(const avifImage *image, const char *stage) {
    if (!env_debug_enabled() || !image) {
        return;
    }
    fprintf(
        stderr,
        "[xun-avif] %s: w=%u h=%u depth=%u yuvFormat=%u yuvRange=%u primaries=%u trc=%u matrix=%u icc=%zu alphaPremul=%u\n",
        stage,
        image->width,
        image->height,
        image->depth,
        (unsigned)image->yuvFormat,
        (unsigned)image->yuvRange,
        (unsigned)image->colorPrimaries,
        (unsigned)image->transferCharacteristics,
        (unsigned)image->matrixCoefficients,
        image->icc.size,
        (unsigned)image->alphaPremultiplied
    );
}

static void apply_decode_overrides(avifImage *image) {
    const char *matrix = getenv("XUN_AVIF_FORCE_MATRIX");
    const char *range = getenv("XUN_AVIF_FORCE_RANGE");
    const char *primaries = getenv("XUN_AVIF_FORCE_PRIMARIES");
    const char *trc = getenv("XUN_AVIF_FORCE_TRC");

    if (!image) {
        return;
    }

    if (matrix && matrix[0] != '\0') {
        if (strcmp(matrix, "identity") == 0) {
            image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_IDENTITY;
        } else if (strcmp(matrix, "bt709") == 0) {
            image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_BT709;
        } else if (strcmp(matrix, "bt601") == 0) {
            image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_BT601;
        } else if (strcmp(matrix, "unspecified") == 0) {
            image->matrixCoefficients = AVIF_MATRIX_COEFFICIENTS_UNSPECIFIED;
        }
    }

    if (range && range[0] != '\0') {
        if (strcmp(range, "full") == 0) {
            image->yuvRange = AVIF_RANGE_FULL;
        } else if (strcmp(range, "limited") == 0) {
            image->yuvRange = AVIF_RANGE_LIMITED;
        }
    }

    if (primaries && primaries[0] != '\0') {
        if (strcmp(primaries, "bt709") == 0) {
            image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
        } else if (strcmp(primaries, "bt601") == 0) {
            image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT601;
        } else if (strcmp(primaries, "srgb") == 0) {
            image->colorPrimaries = AVIF_COLOR_PRIMARIES_BT709;
        } else if (strcmp(primaries, "p3") == 0) {
            image->colorPrimaries = AVIF_COLOR_PRIMARIES_SMPTE432;
        } else if (strcmp(primaries, "unspecified") == 0) {
            image->colorPrimaries = AVIF_COLOR_PRIMARIES_UNSPECIFIED;
        }
    }

    if (trc && trc[0] != '\0') {
        if (strcmp(trc, "srgb") == 0) {
            image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB;
        } else if (strcmp(trc, "bt709") == 0) {
            image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_BT709;
        } else if (strcmp(trc, "linear") == 0) {
            image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_LINEAR;
        } else if (strcmp(trc, "unspecified") == 0) {
            image->transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_UNSPECIFIED;
        }
    }

    maybe_log_image_meta(image, "after-overrides");
}

static int env_truthy(const char *name) {
    const char *v = getenv(name);
    if (!v || v[0] == '\0') {
        return 0;
    }
    if (strcmp(v, "0") == 0 || strcmp(v, "false") == 0 || strcmp(v, "FALSE") == 0) {
        return 0;
    }
    return 1;
}

static int choose_auto_decode_threads(const avifImage *image) {
    if (!image) {
        return 1;
    }
    uint64_t pixels = (uint64_t)image->width * (uint64_t)image->height;
    if (pixels >= 18000000ULL) {
        return 4;
    }
    if (pixels >= 8000000ULL) {
        return 3;
    }
    if (pixels >= 2000000ULL) {
        return 2;
    }
    return 1;
}

XUN_API int32_t xun_avif_decode_rgba8(const uint8_t *data, size_t len, XunAvifImage *out) {
    if (!data || !out || len == 0) {
        return -1;
    }
    memset(out, 0, sizeof(*out));

    avifDecoder *dec = avifDecoderCreate();
    if (!dec) {
        return -2;
    }

    int forcedDecThreads = 0;
    const char *decThreads = getenv("XUN_AVIF_DEC_THREADS");
    if (decThreads && decThreads[0] != '\0') {
        int t = atoi(decThreads);
        if (t > 0) {
            forcedDecThreads = t;
        }
    }

    avifResult r = avifDecoderSetIOMemory(dec, data, len);
    if (r == AVIF_RESULT_OK) {
        r = avifDecoderParse(dec);
    }
    if (r == AVIF_RESULT_OK) {
        dec->maxThreads = (forcedDecThreads > 0) ? forcedDecThreads : choose_auto_decode_threads(dec->image);
        r = avifDecoderNextImage(dec);
    }
    if (r != AVIF_RESULT_OK) {
        avifDecoderDestroy(dec);
        return -3;
    }

    maybe_log_image_meta(dec->image, "decoded");
    apply_decode_overrides(dec->image);

    avifRGBImage rgb;
    avifRGBImageSetDefaults(&rgb, dec->image);
    rgb.format = AVIF_RGB_FORMAT_RGBA;
    rgb.depth = 8;
    // Prefer libyuv fast path by default, keep a kill-switch for exact behavior fallback.
    rgb.chromaUpsampling = AVIF_CHROMA_UPSAMPLING_AUTOMATIC;
    rgb.avoidLibYUV = AVIF_FALSE;
    if (env_truthy("XUN_AVIF_DISABLE_FAST_RGB")) {
        rgb.chromaUpsampling = AVIF_CHROMA_UPSAMPLING_BEST_QUALITY;
        rgb.avoidLibYUV = AVIF_TRUE;
    }

    const uint32_t stride = dec->image->width * 4;
    const size_t size = (size_t)stride * (size_t)dec->image->height;
    uint8_t *buf = (uint8_t *)malloc(size);
    if (!buf) {
        avifDecoderDestroy(dec);
        return -4;
    }

    rgb.pixels = buf;
    rgb.rowBytes = stride;
    r = avifImageYUVToRGB(dec->image, &rgb);
    if (r != AVIF_RESULT_OK) {
        free(buf);
        avifDecoderDestroy(dec);
        return -5;
    }

    out->pixels = buf;
    out->width = dec->image->width;
    out->height = dec->image->height;
    out->stride = stride;

    avifDecoderDestroy(dec);
    return 0;
}

XUN_API void xun_avif_free(void *p) {
    if (p) {
        free(p);
    }
}
