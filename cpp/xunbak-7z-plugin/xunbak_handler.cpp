#include "xunbak_handler.h"

#include "xunbak_utils.h"
#include <algorithm>
#include <cctype>
#include <limits>
#include <vector>

namespace {

constexpr uint32_t kSeekOriginStart = 0;
constexpr uint32_t kSeekOriginCurrent = 1;
constexpr uint32_t kSeekOriginEnd = 2;

struct PropertyDescriptor {
  const wchar_t *name;
  PROPID prop_id;
  VARTYPE var_type;
};

constexpr PropertyDescriptor kItemPropertyTable[] = {
    {L"Path", kpidPath, VT_BSTR},
    {L"Size", kpidSize, VT_UI8},
    {L"Packed Size", kpidPackSize, VT_UI8},
    {L"Modified", kpidMTime, VT_FILETIME},
    {L"Created", kpidCTime, VT_FILETIME},
    {L"Attributes", kpidAttrib, VT_UI4},
};

constexpr PropertyDescriptor kArchivePropertyTable[] = {
    {L"Read Only", kpidReadOnly, VT_BOOL},
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
  CMyComPtr<IArchiveOpenVolumeCallback> open_volume_callback;
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
  auto handle = std::make_unique<XunbakInArchive::StreamHandle>();

  if (name == bridge->primary_name_utf8) {
    handle->kind = XunbakInArchive::StreamHandle::Kind::Memory;
    handle->memory = bridge->primary_bytes;
    *out_handle = handle.release();
    return XUNBAK_OK;
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
  if (FAILED(ReadAll(stream, &bridge_->primary_bytes))) {
    bridge_.reset();
    return E_FAIL;
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

  const int32_t direct_status = xunbak_open(
      bridge_->primary_bytes.data(),
      bridge_->primary_bytes.size(),
      &archive_);
  if (direct_status == XUNBAK_OK) {
    return S_OK;
  }

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

  int32_t last_status = direct_status;
  for (const auto &candidate : candidates) {
    last_status = xunbak_open_with_callbacks(
        reinterpret_cast<const uint8_t *>(candidate.data()),
        candidate.size(),
        &callbacks,
        &archive_);
    if (last_status == XUNBAK_OK) {
      return S_OK;
    }
  }

  return MapCoreStatus(last_status);
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
      uint64_t size = 0;
      const int32_t size_status = xunbak_item_size(archive_, index, &size);
      if (size_status != XUNBAK_OK) {
        return MapCoreStatus(size_status);
      }
      std::vector<uint8_t> buffer(static_cast<size_t>(size));
      size_t written = 0;
      const int32_t status =
          xunbak_extract(archive_, index, buffer.data(), buffer.size(), &written);
      if (status != XUNBAK_OK) {
        return MapCoreStatus(status);
      }
      UInt32 processed = 0;
      const HRESULT write_hr =
          outStream->Write(buffer.data(), static_cast<UInt32>(written), &processed);
      if (FAILED(write_hr)) {
        return write_hr;
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
      return xunbak_utils::SetVariantUInt64(static_cast<uint64_t>(bridge_ ? bridge_->primary_bytes.size() : 0), value);
    case kpidReadOnly:
      return xunbak_utils::SetVariantBool(false, value);
    case kpidNumVolumes:
      return xunbak_utils::SetVariantUInt32(1, value);
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
