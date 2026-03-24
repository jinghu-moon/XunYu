#include "xunbak_handler.h"
#include "xunbak_utils.h"
#include <algorithm>

#define STDAPI_LIB EXTERN_C EXPORTED HRESULT STDAPICALLTYPE

namespace {

// {0bd5e6f4-6417-4d47-8f39-9443f1c8d451}
Z7_DEFINE_GUID(XunbakHandlerGuid, 0x0bd5e6f4, 0x6417, 0x4d47, 0x8f, 0x39, 0x94,
               0x43, 0xf1, 0xc8, 0xd4, 0x51);

struct ArchiveHandler {
  const wchar_t *name;
  GUID guid;
  const wchar_t *extension;
  const wchar_t *add_extension;
  UInt32 flags;
  char signature[8];
  void *(*factory)();
};

const ArchiveHandler kHandler = {
    L"XUNBAK",
    XunbakHandlerGuid,
    L"xunbak",
    L"",
    NArcInfoFlags::kFindSignature,
    {'X', 'U', 'N', 'B', 'A', 'K', '\0', '\0'},
    []() -> void * { return new XunbakInArchive(); },
};

API_FUNC_static_IsArc XunbakIsArc(const Byte *p, size_t size) {
  if (size < 8) {
    return k_IsArc_Res_NEED_MORE;
  }
  return std::equal(kHandler.signature, kHandler.signature + 8, p)
             ? k_IsArc_Res_YES
             : k_IsArc_Res_NO;
}}

}  // namespace

STDAPI_LIB CreateObject(const GUID *clsid, const GUID *iid, void **outObject) {
  if (!clsid || !iid || !outObject) {
    return E_INVALIDARG;
  }
  *outObject = nullptr;
  if (*clsid != kHandler.guid || *iid != IID_IInArchive) {
    return CLASS_E_CLASSNOTAVAILABLE;
  }
  *outObject = kHandler.factory();
  static_cast<IUnknown *>(*outObject)->AddRef();
  return S_OK;
}

STDAPI_LIB GetNumberOfFormats(UInt32 *numFormats) {
  if (!numFormats) {
    return E_INVALIDARG;
  }
  *numFormats = 1;
  return S_OK;
}

STDAPI_LIB GetHandlerProperty2(UInt32 formatIndex, PROPID propID, PROPVARIANT *value) {
  if (formatIndex != 0 || !value) {
    return E_INVALIDARG;
  }
  ::PropVariantInit(value);
  switch (propID) {
    case NArchive::NHandlerPropID::kName:
      return xunbak_utils::SetVariantWideString(kHandler.name, value);
    case NArchive::NHandlerPropID::kClassID:
      return xunbak_utils::SetVariantGuid(kHandler.guid, value);
    case NArchive::NHandlerPropID::kExtension:
      return xunbak_utils::SetVariantWideString(kHandler.extension, value);
    case NArchive::NHandlerPropID::kAddExtension:
      return xunbak_utils::SetVariantWideString(kHandler.add_extension, value);
    case NArchive::NHandlerPropID::kUpdate:
      return xunbak_utils::SetVariantBool(false, value);
    case NArchive::NHandlerPropID::kFlags:
      return xunbak_utils::SetVariantUInt32(kHandler.flags, value);
    case NArchive::NHandlerPropID::kSignature:
      return xunbak_utils::SetVariantBinary(kHandler.signature, sizeof(kHandler.signature), value);
    case NArchive::NHandlerPropID::kSignatureOffset:
      return xunbak_utils::SetVariantUInt32(0, value);
    default:
      return S_OK;
  }
}

STDAPI_LIB GetHandlerProperty(PROPID propID, PROPVARIANT *value) {
  return GetHandlerProperty2(0, propID, value);
}

STDAPI_LIB GetIsArc(UInt32 formatIndex, Func_IsArc *isArc) {
  if (formatIndex != 0 || !isArc) {
    return E_INVALIDARG;
  }
  *isArc = XunbakIsArc;
  return S_OK;
}
