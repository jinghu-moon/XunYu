#ifndef XUNBAK_7Z_PLUGIN_HANDLER_H
#define XUNBAK_7Z_PLUGIN_HANDLER_H

#include "Export.h"
#include "xunbak_ffi.h"
#include <memory>
#include <string>
#include <vector>

class XunbakInArchive : public CMyUnknownImp, public IInArchive {
 public:
  Z7_IFACES_IMP_UNK_1(IInArchive)
  public:

  struct StreamHandle;
  struct BridgeContext;

  XunbakInArchive();
  ~XunbakInArchive();

 private:
  std::unique_ptr<BridgeContext> bridge_;
  XunbakArchiveHandle *archive_ = nullptr;

  HRESULT OpenCore(IInStream *stream, IArchiveOpenCallback *openCallback) noexcept;
  HRESULT FillProperty(UInt32 index, PROPID propID, PROPVARIANT *value) noexcept;
};

#endif
