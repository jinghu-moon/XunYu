#ifndef XUNBAK_7Z_PLUGIN_UTILS_H
#define XUNBAK_7Z_PLUGIN_UTILS_H

#include "Export.h"
#include <string>
#include <vector>

namespace xunbak_utils {

HRESULT SetVariantBool(bool value, PROPVARIANT *variant) noexcept;
HRESULT SetVariantUInt32(uint32_t value, PROPVARIANT *variant) noexcept;
HRESULT SetVariantUInt64(uint64_t value, PROPVARIANT *variant) noexcept;
HRESULT SetVariantGuid(const GUID &guid, PROPVARIANT *variant) noexcept;
HRESULT SetVariantBinary(const char *bytes, size_t size, PROPVARIANT *variant) noexcept;
HRESULT SetVariantWideString(const wchar_t *value, PROPVARIANT *variant) noexcept;
HRESULT SetVariantFileTimeFromUnixNs(uint64_t unix_ns, PROPVARIANT *variant) noexcept;
HRESULT SetVariantPathFromUtf16Bytes(const std::vector<uint8_t> &bytes, PROPVARIANT *variant) noexcept;

std::wstring Utf8ToWide(const std::string &value);
std::string WideToUtf8(const std::wstring &value);

}  // namespace xunbak_utils

#endif
