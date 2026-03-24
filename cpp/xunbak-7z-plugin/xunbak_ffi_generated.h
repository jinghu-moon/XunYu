#ifndef XUNBAK_FFI_GENERATED_H
#define XUNBAK_FFI_GENERATED_H

#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define XUNBAK_OK 0

#define XUNBAK_ERR_INVALID_ARG 1

#define XUNBAK_ERR_OPEN 2

#define XUNBAK_ERR_RANGE 3

#define XUNBAK_ERR_BUFFER_TOO_SMALL 4

#define XUNBAK_ERR_IO 5

#define XUNBAK_PROP_PATH 0

#define XUNBAK_PROP_SIZE 1

#define XUNBAK_PROP_PACKED_SIZE 2

#define XUNBAK_PROP_MTIME_NS 3

#define XUNBAK_PROP_CTIME_NS 4

#define XUNBAK_PROP_WIN_ATTRIBUTES 5

#define XUNBAK_PROP_CODEC_ID 6

typedef struct XunbakArchiveHandle XunbakArchiveHandle;

typedef struct XunbakVolumeCallbacks {
  void *ctx;
  int32_t (*open_volume)(void *ctx,
                         const uint8_t *volume_name_ptr,
                         size_t volume_name_len,
                         void **out_handle);
  int32_t (*read)(void *ctx, void *stream_handle, uint8_t *out_buf, size_t buf_len, size_t *out_read);
  int32_t (*seek)(void *ctx, void *stream_handle, int64_t offset, uint32_t origin, uint64_t *out_pos);
  void (*close_volume)(void *ctx, void *stream_handle);
} XunbakVolumeCallbacks;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

int32_t xunbak_open(const uint8_t *data, size_t len, struct XunbakArchiveHandle **out);

int32_t xunbak_open_with_callbacks(const uint8_t *primary_name_ptr,
                                   size_t primary_name_len,
                                   const struct XunbakVolumeCallbacks *callbacks,
                                   struct XunbakArchiveHandle **out);

void xunbak_close(struct XunbakArchiveHandle *handle);

uint32_t xunbak_item_count(const struct XunbakArchiveHandle *handle);

uint32_t xunbak_volume_count(const struct XunbakArchiveHandle *handle);

int32_t xunbak_get_property(const struct XunbakArchiveHandle *archive,
                            uint32_t index,
                            uint32_t prop_id,
                            void *out_buf,
                            size_t buf_len,
                            size_t *out_written);

int32_t xunbak_extract(const struct XunbakArchiveHandle *archive,
                       uint32_t index,
                       uint8_t *out_buf,
                       size_t buf_len,
                       size_t *out_written);

int32_t xunbak_item_size(const struct XunbakArchiveHandle *archive,
                         uint32_t index,
                         uint64_t *out_size);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* XUNBAK_FFI_GENERATED_H */
