#include "xunbak_handler.h"

#include "xunbak_utils.h"
#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <fstream>
#include <limits>
#include <string_view>
#include <vector>

namespace {

constexpr uint32_t kSeekOriginStart = 0;
constexpr uint32_t kSeekOriginCurrent = 1;
constexpr uint32_t kSeekOriginEnd = 2;
constexpr uint64_t kDefaultMemoryFallbackMaxBytes = 64ULL * 1024ULL * 1024ULL;

struct PropertyDescriptor {
  const wchar_t *name;
  PROPID prop_id;
  VARTYPE var_type;
};

constexpr PropertyDescriptor kItemPropertyTable[] = {
    {L"Path", kpidPath, VT_BSTR},
    {L"Size", kpidSize, VT_UI8},
    {L"Packed Size", kpidPackSize, VT_UI8},
    {L"Method", kpidMethod, VT_BSTR},
    {L"Modified", kpidMTime, VT_FILETIME},
    {L"Created", kpidCTime, VT_FILETIME},
    {L"Attributes", kpidAttrib, VT_UI4},
};

constexpr PropertyDescriptor kArchivePropertyTable[] = {
    {L"Read Only", kpidReadOnly, VT_BOOL},
    {L"Files", kpidNumSubFiles, VT_UI4},
    {L"Volumes", kpidNumVolumes, VT_UI4},
};

HRESULT MapCoreStatus(int32_t status) noexcept {
  switch (status) {
    case XUNBAK_OK:
      return S_OK;
    case XUNBAK_ERR_OPEN:
      return S_FALSE;
    case XUNBAK_ERR_INVALID_ARG:
      return E_INVALIDARG;
    case XUNBAK_ERR_RANGE:
      return E_BOUNDS;
    case XUNBAK_ERR_BUFFER_TOO_SMALL:
      return HRESULT_FROM_WIN32(ERROR_INSUFFICIENT_BUFFER);
    default:
      return E_FAIL;
  }
}

bool EndsWithSplitSuffix(const std::string &value) {
  return value.size() >= 4 &&
         value[value.size() - 4] == '.' &&
         std::isdigit(static_cast<unsigned char>(value[value.size() - 3])) &&
         std::isdigit(static_cast<unsigned char>(value[value.size() - 2])) &&
         std::isdigit(static_cast<unsigned char>(value[value.size() - 1]));
}

const wchar_t *CodecName(uint32_t codec_id) noexcept {
  switch (codec_id) {
    case 0:
      return L"Copy";
    case 1:
      return L"ZSTD";
    case 2:
      return L"LZ4";
    case 3:
      return L"LZMA";
    default:
      return L"Unknown";
  }
}

bool EnvFlagEnabled(const char *name) {
  const DWORD size = ::GetEnvironmentVariableA(name, nullptr, 0);
  if (size == 0) {
    return false;
  }
  std::string value(static_cast<size_t>(size), '\0');
  const DWORD written = ::GetEnvironmentVariableA(name, value.data(), size);
  if (written == 0) {
    return false;
  }
  value.resize(static_cast<size_t>(written));
  std::transform(value.begin(), value.end(), value.begin(), [](unsigned char ch) {
    return static_cast<char>(std::tolower(ch));
  });
  return value == "1" || value == "true" || value == "yes" || value == "on";
}

uint64_t ResolveMemoryFallbackMaxBytes() {
  const DWORD size = ::GetEnvironmentVariableA("XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES", nullptr, 0);
  if (size == 0) {
    return kDefaultMemoryFallbackMaxBytes;
  }
  std::string value(static_cast<size_t>(size), '\0');
  const DWORD written =
      ::GetEnvironmentVariableA("XUN_XUNBAK_PLUGIN_FALLBACK_MAX_BYTES", value.data(), size);
  if (written == 0) {
    return kDefaultMemoryFallbackMaxBytes;
  }
  value.resize(static_cast<size_t>(written));
  char *end = nullptr;
  const unsigned long long parsed = std::strtoull(value.c_str(), &end, 10);
  if (end == value.c_str() || (end && *end != '\0')) {
    return kDefaultMemoryFallbackMaxBytes;
  }
  return static_cast<uint64_t>(parsed);
}

void TraceOpenPath(std::string_view line) {
  const DWORD size = ::GetEnvironmentVariableA("XUN_XUNBAK_PLUGIN_TRACE_FILE", nullptr, 0);
  if (size == 0) {
    return;
  }
  std::string path(static_cast<size_t>(size), '\0');
  const DWORD written =
      ::GetEnvironmentVariableA("XUN_XUNBAK_PLUGIN_TRACE_FILE", path.data(), size);
  if (written == 0) {
    return;
  }
  path.resize(static_cast<size_t>(written));
  std::ofstream out(path, std::ios::app);
  if (!out) {
    return;
  }
  out << line << '\n';
}

bool IsMemoryFallbackAllowed(const std::string &primary_name,
                             uint64_t primary_stream_size,
                             uint64_t threshold_bytes,
                             std::string *reason) {
  if (EndsWithSplitSuffix(primary_name)) {
    if (reason) {
      *reason = "split_requires_callbacks";
    }
    return false;
  }
  if (threshold_bytes == 0 || primary_stream_size > threshold_bytes) {
    if (reason) {
      *reason = "threshold";
    }
    return false;
  }
  if (reason) {
    reason->clear();
  }
  return true;
}

HRESULT ReadAll(IInStream *stream, std::vector<uint8_t> *out) noexcept {
  out->clear();
  if (!stream) {
    return E_INVALIDARG;
  }

  LARGE_INTEGER zero{};
  stream->Seek(0, STREAM_SEEK_SET, nullptr);

  std::vector<uint8_t> chunk(64 * 1024);
  while (true) {
    UInt32 processed = 0;
    const HRESULT hr = stream->Read(chunk.data(), static_cast<UInt32>(chunk.size()), &processed);
    if (FAILED(hr)) {
      return hr;
    }
    if (processed == 0) {
      break;
    }
    out->insert(out->end(), chunk.begin(), chunk.begin() + processed);
  }
  return S_OK;
}

HRESULT CopyPropertyDescriptor(const PropertyDescriptor &descriptor,
                               BSTR *name,
                               PROPID *propID,
                               VARTYPE *varType) noexcept {
  if (!name || !propID || !varType) {
    return E_INVALIDARG;
  }
  *name = ::SysAllocString(descriptor.name);
  if (!*name) {
    return E_OUTOFMEMORY;
  }
  *propID = descriptor.prop_id;
  *varType = descriptor.var_type;
  return S_OK;
}

}  // namespace

struct XunbakInArchive::StreamHandle {
  enum class Kind {
    Memory,
    Com
  };

  Kind kind = Kind::Memory;
  std::vector<uint8_t> memory;
  uint64_t position = 0;
  CMyComPtr<IInStream> stream;
};

struct XunbakInArchive::BridgeContext {
  std::wstring primary_name_w;
  std::string primary_name_utf8;
  std::vector<uint8_t> primary_bytes;
  CMyComPtr<IInStream> primary_stream;
  uint64_t primary_stream_size = 0;
  CMyComPtr<IArchiveOpenVolumeCallback> open_volume_callback;
};

struct OutputWriteContext {
  ISequentialOutStream *stream = nullptr;
};

static int32_t XunbakOpenVolume(void *ctx,
                                const uint8_t *volume_name_ptr,
                                size_t volume_name_len,
                                void **out_handle) {
  if (!ctx || !volume_name_ptr || !out_handle) {
    return XUNBAK_ERR_INVALID_ARG;
  }
  auto *bridge = reinterpret_cast<XunbakInArchive::BridgeContext *>(ctx);
  const std::string name(reinterpret_cast<const char *>(volume_name_ptr), volume_name_len);
  if (EnvFlagEnabled("XUN_XUNBAK_PLUGIN_TEST_FAIL_CALLBACK_OPEN")) {
    TraceOpenPath("volume.callback.forced_failure name=" + name);
    return XUNBAK_ERR_OPEN;
  }
  auto handle = std::make_unique<XunbakInArchive::StreamHandle>();

  if (name == bridge->primary_name_utf8) {
    if (!bridge->primary_bytes.empty()) {
      handle->kind = XunbakInArchive::StreamHandle::Kind::Memory;
      handle->memory = bridge->primary_bytes;
      *out_handle = handle.release();
      return XUNBAK_OK;
    }
    if (bridge->primary_stream) {
      const HRESULT reset_hr = bridge->primary_stream->Seek(0, STREAM_SEEK_SET, nullptr);
      if (FAILED(reset_hr)) {
        return XUNBAK_ERR_IO;
      }
      handle->kind = XunbakInArchive::StreamHandle::Kind::Com;
      handle->stream = bridge->primary_stream;
      *out_handle = handle.release();
      return XUNBAK_OK;
    }
  }

  if (!bridge->open_volume_callback) {
    return XUNBAK_ERR_OPEN;
  }

  const std::wstring wide_name = xunbak_utils::Utf8ToWide(name);
  CMyComPtr<IInStream> stream;
  const HRESULT hr = bridge->open_volume_callback->GetStream(wide_name.c_str(), &stream);
  if (FAILED(hr) || !stream) {
    return XUNBAK_ERR_OPEN;
  }
  handle->kind = XunbakInArchive::StreamHandle::Kind::Com;
  handle->stream = stream;
  *out_handle = handle.release();
  return XUNBAK_OK;
}

static int32_t XunbakReadVolume(void *ctx,
                                void *stream_handle,
                                uint8_t *out_buf,
                                size_t buf_len,
                                size_t *out_read) {
  (void)ctx;
  if (!stream_handle || !out_buf || !out_read) {
    return XUNBAK_ERR_INVALID_ARG;
  }
  auto *handle = reinterpret_cast<XunbakInArchive::StreamHandle *>(stream_handle);
  if (handle->kind == XunbakInArchive::StreamHandle::Kind::Memory) {
    const uint64_t remain = handle->position >= handle->memory.size()
                                ? 0
                                : static_cast<uint64_t>(handle->memory.size()) - handle->position;
    const size_t to_copy = static_cast<size_t>(std::min<uint64_t>(remain, buf_len));
    if (to_copy > 0) {
      std::copy_n(handle->memory.data() + handle->position, to_copy, out_buf);
      handle->position += to_copy;
    }
    *out_read = to_copy;
    return XUNBAK_OK;
  }

  UInt32 processed = 0;
  const size_t requested_size = (std::min)(buf_len, static_cast<size_t>((std::numeric_limits<UInt32>::max)()));
  const UInt32 requested = static_cast<UInt32>(requested_size);
  const HRESULT hr = handle->stream->Read(out_buf, requested, &processed);
  if (FAILED(hr)) {
    return XUNBAK_ERR_IO;
  }
  *out_read = processed;
  return XUNBAK_OK;
}

static int32_t XunbakSeekVolume(void *ctx,
                                void *stream_handle,
                                int64_t offset,
                                uint32_t origin,
                                uint64_t *out_pos) {
  (void)ctx;
  if (!stream_handle || !out_pos) {
    return XUNBAK_ERR_INVALID_ARG;
  }
  auto *handle = reinterpret_cast<XunbakInArchive::StreamHandle *>(stream_handle);
  if (handle->kind == XunbakInArchive::StreamHandle::Kind::Memory) {
    int64_t base = 0;
    switch (origin) {
      case kSeekOriginStart:
        base = 0;
        break;
      case kSeekOriginCurrent:
        base = static_cast<int64_t>(handle->position);
        break;
      case kSeekOriginEnd:
        base = static_cast<int64_t>(handle->memory.size());
        break;
      default:
        return XUNBAK_ERR_INVALID_ARG;
    }
    const int64_t next = base + offset;
    if (next < 0) {
      return XUNBAK_ERR_INVALID_ARG;
    }
    handle->position = static_cast<uint64_t>(next);
    *out_pos = handle->position;
    return XUNBAK_OK;
  }

  DWORD move_method = STREAM_SEEK_SET;
  switch (origin) {
    case kSeekOriginStart:
      move_method = STREAM_SEEK_SET;
      break;
    case kSeekOriginCurrent:
      move_method = STREAM_SEEK_CUR;
      break;
    case kSeekOriginEnd:
      move_method = STREAM_SEEK_END;
      break;
    default:
      return XUNBAK_ERR_INVALID_ARG;
  }

  ULARGE_INTEGER new_pos{};
  const HRESULT hr = handle->stream->Seek(offset, move_method, &new_pos.QuadPart);
  if (FAILED(hr)) {
    return XUNBAK_ERR_IO;
  }
  *out_pos = new_pos.QuadPart;
  return XUNBAK_OK;
}

static void XunbakCloseVolume(void *ctx, void *stream_handle) {
  (void)ctx;
  auto *handle = reinterpret_cast<XunbakInArchive::StreamHandle *>(stream_handle);
  delete handle;
}

static int32_t XunbakWriteOut(void *ctx,
                              const uint8_t *data_ptr,
                              size_t data_len,
                              size_t *out_written) {
  if (!ctx || !data_ptr || !out_written) {
    return XUNBAK_ERR_INVALID_ARG;
  }
  auto *writer = reinterpret_cast<OutputWriteContext *>(ctx);
  if (!writer->stream) {
    return XUNBAK_ERR_INVALID_ARG;
  }

  const UInt32 requested = static_cast<UInt32>((std::min)(
      data_len, static_cast<size_t>((std::numeric_limits<UInt32>::max)())));
  UInt32 processed = 0;
  const HRESULT hr = writer->stream->Write(data_ptr, requested, &processed);
  if (FAILED(hr)) {
    return XUNBAK_ERR_IO;
  }
  *out_written = processed;
  return XUNBAK_OK;
}

XunbakInArchive::XunbakInArchive() = default;

XunbakInArchive::~XunbakInArchive() {
  Close();
}

HRESULT XunbakInArchive::Open(IInStream *stream,
                              const UInt64 *maxCheckStartPosition,
                              IArchiveOpenCallback *openCallback) noexcept {
  (void)maxCheckStartPosition;
  return OpenCore(stream, openCallback);
}

HRESULT XunbakInArchive::OpenCore(IInStream *stream, IArchiveOpenCallback *openCallback) noexcept {
  Close();
  if (!stream) {
    return E_INVALIDARG;
  }

  bridge_ = std::make_unique<BridgeContext>();
  bridge_->primary_stream = stream;
  {
    ULARGE_INTEGER current_pos{};
    ULARGE_INTEGER end_pos{};
    if (FAILED(stream->Seek(0, STREAM_SEEK_CUR, &current_pos.QuadPart)) ||
        FAILED(stream->Seek(0, STREAM_SEEK_END, &end_pos.QuadPart)) ||
        FAILED(stream->Seek(static_cast<Int64>(current_pos.QuadPart), STREAM_SEEK_SET, nullptr))) {
      bridge_.reset();
      return E_FAIL;
    }
    bridge_->primary_stream_size = end_pos.QuadPart;
  }

  if (openCallback) {
    openCallback->QueryInterface(IID_IArchiveOpenVolumeCallback,
                                 reinterpret_cast<void **>(&bridge_->open_volume_callback));
  }

  if (bridge_->open_volume_callback) {
    PROPVARIANT prop{};
    ::PropVariantInit(&prop);
    if (bridge_->open_volume_callback->GetProperty(kpidName, &prop) == S_OK && prop.vt == VT_BSTR) {
      bridge_->primary_name_w.assign(prop.bstrVal, ::SysStringLen(prop.bstrVal));
      bridge_->primary_name_utf8 = xunbak_utils::WideToUtf8(bridge_->primary_name_w);
    }
    ::PropVariantClear(&prop);
  }

  if (bridge_->primary_name_utf8.empty()) {
    bridge_->primary_name_w = L"memory.xunbak";
    bridge_->primary_name_utf8 = "memory.xunbak";
  }

  TraceOpenPath("open.start name=" + bridge_->primary_name_utf8 +
                " size=" + std::to_string(bridge_->primary_stream_size));

  XunbakVolumeCallbacks callbacks{};
  callbacks.ctx = bridge_.get();
  callbacks.open_volume = &XunbakOpenVolume;
  callbacks.read = &XunbakReadVolume;
  callbacks.seek = &XunbakSeekVolume;
  callbacks.close_volume = &XunbakCloseVolume;

  std::vector<std::string> candidates{bridge_->primary_name_utf8};
  if (!EndsWithSplitSuffix(bridge_->primary_name_utf8)) {
    candidates.push_back(bridge_->primary_name_utf8 + ".001");
  }

  int32_t last_status = XUNBAK_ERR_OPEN;
  for (const auto &candidate : candidates) {
    TraceOpenPath("open.callback.try candidate=" + candidate);
    last_status = xunbak_open_with_callbacks(
        reinterpret_cast<const uint8_t *>(candidate.data()),
        candidate.size(),
        &callbacks,
        &archive_);
    TraceOpenPath("open.callback.result candidate=" + candidate +
                  " status=" + std::to_string(last_status));
    if (last_status == XUNBAK_OK) {
      TraceOpenPath("open.callback.success candidate=" + candidate);
      return S_OK;
    }
  }

  const uint64_t fallback_max_bytes = ResolveMemoryFallbackMaxBytes();
  std::string fallback_reason;
  if (!IsMemoryFallbackAllowed(
          bridge_->primary_name_utf8,
          bridge_->primary_stream_size,
          fallback_max_bytes,
          &fallback_reason)) {
    TraceOpenPath("open.fallback.rejected reason=" + fallback_reason +
                  " size=" + std::to_string(bridge_->primary_stream_size) +
                  " threshold=" + std::to_string(fallback_max_bytes) +
                  " callback_status=" + std::to_string(last_status));
    if (fallback_reason == "threshold") {
      return HRESULT_FROM_WIN32(ERROR_FILE_TOO_LARGE);
    }
    return HRESULT_FROM_WIN32(ERROR_NOT_SUPPORTED);
  }

  TraceOpenPath("open.fallback.allowed size=" + std::to_string(bridge_->primary_stream_size) +
                " threshold=" + std::to_string(fallback_max_bytes) +
                " callback_status=" + std::to_string(last_status));
  bridge_->primary_stream.Release();
  TraceOpenPath("open.readall.begin");
  if (FAILED(ReadAll(stream, &bridge_->primary_bytes))) {
    TraceOpenPath("open.readall.failed");
    bridge_.reset();
    return E_FAIL;
  }
  TraceOpenPath("open.readall.end bytes=" + std::to_string(bridge_->primary_bytes.size()));

  const int32_t direct_status = xunbak_open(
      bridge_->primary_bytes.data(),
      bridge_->primary_bytes.size(),
      &archive_);
  TraceOpenPath("open.direct.result status=" + std::to_string(direct_status));
  if (direct_status == XUNBAK_OK) {
    TraceOpenPath("open.direct.success");
    return S_OK;
  }

  return MapCoreStatus(direct_status);
}

HRESULT XunbakInArchive::Close() noexcept {
  if (archive_) {
    xunbak_close(archive_);
    archive_ = nullptr;
  }
  bridge_.reset();
  return S_OK;
}

HRESULT XunbakInArchive::GetNumberOfItems(UInt32 *numItems) noexcept {
  if (!numItems || !archive_) {
    return E_INVALIDARG;
  }
  *numItems = xunbak_item_count(archive_);
  return S_OK;
}

HRESULT XunbakInArchive::FillProperty(UInt32 index, PROPID propID, PROPVARIANT *value) noexcept {
  if (!archive_ || !value) {
    return E_INVALIDARG;
  }
  ::PropVariantInit(value);
  switch (propID) {
    case kpidPath: {
      size_t required = 0;
      int32_t status = xunbak_get_property(archive_, index, XUNBAK_PROP_PATH, nullptr, 0, &required);
      if (status != XUNBAK_ERR_BUFFER_TOO_SMALL && status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      std::vector<uint8_t> bytes(required);
      status = xunbak_get_property(archive_, index, XUNBAK_PROP_PATH, bytes.data(), bytes.size(), &required);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantPathFromUtf16Bytes(bytes, value);
    }
    case kpidIsDir:
      return xunbak_utils::SetVariantBool(false, value);
    case kpidSize: {
      uint64_t size = 0;
      const int32_t status = xunbak_item_size(archive_, index, &size);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantUInt64(size, value);
    }
    case kpidPackSize: {
      uint64_t packed = 0;
      size_t written = 0;
      const int32_t status = xunbak_get_property(
          archive_, index, XUNBAK_PROP_PACKED_SIZE, &packed, sizeof(packed), &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantUInt64(packed, value);
    }
    case kpidMethod: {
      uint32_t codec_id = 0;
      size_t written = 0;
      const int32_t status = xunbak_get_property(
          archive_, index, XUNBAK_PROP_CODEC_ID, &codec_id, sizeof(codec_id), &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantWideString(CodecName(codec_id), value);
    }
    case kpidMTime: {
      uint64_t mtime = 0;
      size_t written = 0;
      const int32_t status =
          xunbak_get_property(archive_, index, XUNBAK_PROP_MTIME_NS, &mtime, sizeof(mtime), &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantFileTimeFromUnixNs(mtime, value);
    }
    case kpidCTime: {
      uint64_t ctime = 0;
      size_t written = 0;
      const int32_t status =
          xunbak_get_property(archive_, index, XUNBAK_PROP_CTIME_NS, &ctime, sizeof(ctime), &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantFileTimeFromUnixNs(ctime, value);
    }
    case kpidAttrib: {
      uint32_t attrs = 0;
      size_t written = 0;
      const int32_t status = xunbak_get_property(
          archive_, index, XUNBAK_PROP_WIN_ATTRIBUTES, &attrs, sizeof(attrs), &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      return xunbak_utils::SetVariantUInt32(attrs, value);
    }
    default:
      return S_OK;
  }
}

HRESULT XunbakInArchive::GetProperty(UInt32 index, PROPID propID, PROPVARIANT *value) noexcept {
  return FillProperty(index, propID, value);
}

HRESULT XunbakInArchive::Extract(const UInt32 *indices,
                                 UInt32 numItems,
                                 Int32 testMode,
                                 IArchiveExtractCallback *extractCallback) noexcept {
  if (!archive_ || !extractCallback) {
    return E_INVALIDARG;
  }

  const UInt32 total = indices ? numItems : xunbak_item_count(archive_);
  for (UInt32 i = 0; i < total; ++i) {
    const UInt32 index = indices ? indices[i] : i;
    const Int32 askMode = testMode ? NArchive::NExtract::NAskMode::kTest
                                   : NArchive::NExtract::NAskMode::kExtract;
    extractCallback->PrepareOperation(askMode);
    CMyComPtr<ISequentialOutStream> outStream;
    const HRESULT get_stream_hr = extractCallback->GetStream(index, &outStream, askMode);
    if (FAILED(get_stream_hr)) {
      return get_stream_hr;
    }

    if (!testMode && outStream) {
      OutputWriteContext writer_ctx{};
      writer_ctx.stream = outStream;
      XunbakWriteCallbacks callbacks{};
      callbacks.ctx = &writer_ctx;
      callbacks.write = &XunbakWriteOut;
      size_t written = 0;
      const int32_t status =
          xunbak_extract_with_writer(archive_, index, &callbacks, &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
    }

    extractCallback->SetOperationResult(NArchive::NExtract::NOperationResult::kOK);
  }

  return S_OK;
}

HRESULT XunbakInArchive::GetArchiveProperty(PROPID propID, PROPVARIANT *value) noexcept {
  if (!value) {
    return E_INVALIDARG;
  }
  ::PropVariantInit(value);
  switch (propID) {
    case kpidPhySize:
      return xunbak_utils::SetVariantUInt64(
          bridge_ ? (bridge_->primary_bytes.empty() ? bridge_->primary_stream_size
                                                    : static_cast<uint64_t>(bridge_->primary_bytes.size()))
                  : 0,
          value);
    case kpidReadOnly:
      return xunbak_utils::SetVariantBool(false, value);
    case kpidNumSubFiles:
      return xunbak_utils::SetVariantUInt32(archive_ ? xunbak_item_count(archive_) : 0, value);
    case kpidNumVolumes:
      return xunbak_utils::SetVariantUInt32(archive_ ? xunbak_volume_count(archive_) : 0, value);
    default:
      return S_OK;
  }
}

HRESULT XunbakInArchive::GetNumberOfProperties(UInt32 *numProps) noexcept {
  if (!numProps) {
    return E_INVALIDARG;
  }
  *numProps = static_cast<UInt32>(std::size(kItemPropertyTable));
  return S_OK;
}

HRESULT XunbakInArchive::GetPropertyInfo(UInt32 index,
                                         BSTR *name,
                                         PROPID *propID,
                                         VARTYPE *varType) noexcept {
  if (index >= std::size(kItemPropertyTable)) {
    return E_INVALIDARG;
  }
  return CopyPropertyDescriptor(kItemPropertyTable[index], name, propID, varType);
}

HRESULT XunbakInArchive::GetNumberOfArchiveProperties(UInt32 *numProps) noexcept {
  if (!numProps) {
    return E_INVALIDARG;
  }
  *numProps = static_cast<UInt32>(std::size(kArchivePropertyTable));
  return S_OK;
}

HRESULT XunbakInArchive::GetArchivePropertyInfo(UInt32 index,
                                                BSTR *name,
                                                PROPID *propID,
                                                VARTYPE *varType) noexcept {
  if (index >= std::size(kArchivePropertyTable)) {
    return E_INVALIDARG;
  }
  return CopyPropertyDescriptor(kArchivePropertyTable[index], name, propID, varType);
}
