#include "xunbak_utils.h"

namespace xunbak_utils {

HRESULT SetVariantBool(bool value, PROPVARIANT *variant) noexcept {
  variant->vt = VT_BOOL;
  variant->boolVal = value ? VARIANT_TRUE : VARIANT_FALSE;
  return S_OK;
}

HRESULT SetVariantUInt32(uint32_t value, PROPVARIANT *variant) noexcept {
  variant->vt = VT_UI4;
  variant->ulVal = value;
  return S_OK;
}

HRESULT SetVariantUInt64(uint64_t value, PROPVARIANT *variant) noexcept {
  variant->vt = VT_UI8;
  variant->uhVal.QuadPart = value;
  return S_OK;
}

HRESULT SetVariantGuid(const GUID &guid, PROPVARIANT *variant) noexcept {
  variant->vt = VT_BSTR;
  variant->bstrVal = ::SysAllocStringByteLen(reinterpret_cast<const char *>(&guid), sizeof(guid));
  return variant->bstrVal ? S_OK : E_OUTOFMEMORY;
}

HRESULT SetVariantBinary(const char *bytes, size_t size, PROPVARIANT *variant) noexcept {
  variant->vt = VT_BSTR;
  variant->bstrVal = ::SysAllocStringByteLen(bytes, static_cast<UINT>(size));
  return variant->bstrVal ? S_OK : E_OUTOFMEMORY;
}

HRESULT SetVariantWideString(const wchar_t *value, PROPVARIANT *variant) noexcept {
  variant->vt = VT_BSTR;
  variant->bstrVal = ::SysAllocString(value);
  return variant->bstrVal ? S_OK : E_OUTOFMEMORY;
}

HRESULT SetVariantFileTimeFromUnixNs(uint64_t unix_ns, PROPVARIANT *variant) noexcept {
  constexpr uint64_t kWindowsEpochDiff100ns = 116444736000000000ULL;
  const uint64_t filetime_100ns = kWindowsEpochDiff100ns + (unix_ns / 100ULL);
  variant->vt = VT_FILETIME;
  variant->filetime.dwLowDateTime = static_cast<DWORD>(filetime_100ns & 0xffffffffULL);
  variant->filetime.dwHighDateTime = static_cast<DWORD>(filetime_100ns >> 32);
  variant->wReserved1 = 25;  // 16 + 9 => unix ns precision
  variant->wReserved2 = static_cast<WORD>((unix_ns % 100ULL) / 1ULL);
  variant->wReserved3 = 0;
  return S_OK;
}

HRESULT SetVariantPathFromUtf16Bytes(const std::vector<uint8_t> &bytes, PROPVARIANT *variant) noexcept {
  if (bytes.empty()) {
    variant->vt = VT_BSTR;
    variant->bstrVal = ::SysAllocStringLen(nullptr, 0);
    return variant->bstrVal ? S_OK : E_OUTOFMEMORY;
  }
  const auto *wide = reinterpret_cast<const wchar_t *>(bytes.data());
  const UINT len = static_cast<UINT>(bytes.size() / sizeof(wchar_t));
  variant->vt = VT_BSTR;
  variant->bstrVal = ::SysAllocStringLen(wide, len);
  return variant->bstrVal ? S_OK : E_OUTOFMEMORY;
}

std::wstring Utf8ToWide(const std::string &value) {
  if (value.empty()) {
    return {};
  }
  const int len = ::MultiByteToWideChar(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), nullptr, 0);
  std::wstring result(static_cast<size_t>(len), L'\0');
  ::MultiByteToWideChar(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), result.data(), len);
  return result;
}

std::string WideToUtf8(const std::wstring &value) {
  if (value.empty()) {
    return {};
  }
  const int len = ::WideCharToMultiByte(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), nullptr, 0, nullptr, nullptr);
  std::string result(static_cast<size_t>(len), '\0');
  ::WideCharToMultiByte(CP_UTF8, 0, value.data(), static_cast<int>(value.size()), result.data(), len, nullptr, nullptr);
  return result;
}

}  // namespace xunbak_utils
