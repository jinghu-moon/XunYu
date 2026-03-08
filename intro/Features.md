# 鍔熻兘涓庣紪璇戠壒鎬э紙features锛?
涓嬭〃浠?`Cargo.toml` 褰撳墠瀹氫箟涓哄噯锛屾弿杩板悇涓?feature 鎵撳紑鍚庝細鏂板浠€涔堣兘鍔涖€?
| Feature | 鍚敤鍚庢柊澧炲懡浠?/ 鑳藉姏 |
| --- | --- |
| 榛樿锛堝惈 `fs`锛?| 涔︾鍛戒护鏃忋€乣config`銆乣ctx`銆乣proxy`銆乣ports/kill/ps/pkill`銆乣bak`銆乣tree`銆乣find`銆乣delete/del`銆乣rm`銆乣env`銆乣video` |
| `alias` | `alias` 鍛戒护鏃忥紱鐢ㄤ簬 alias/shim 鐩稿叧鑳藉姏 |
| `alias-shell-extra` | 棰濆鐨?alias shell 闆嗘垚锛涗緷璧?`alias` |
| `lock` | `lock`銆乣mv`銆乣renfile` 绛変笌閿佸畾鏂囦欢澶勭悊鐩稿叧鑳藉姏 |
| `protect` | `protect set/clear/status`锛涢潰鍚戞枃浠朵繚鎶よ鍒?|
| `crypt` | `encrypt` / `decrypt` |
| `redirect` | `redirect`锛涚洰褰曢噸瀹氬悜 / 鍒嗙被瑙勫垯寮曟搸 |
| `dashboard` | `serve`锛涙湰鍦?Web Dashboard锛圚ome / Bookmarks / Ports / Proxy / Config / Env / Redirect / Audit锛?|
| `diff` | `diff` CLI锛涜嫢鍚屾椂寮€鍚?`dashboard`锛屽垯鎻愪緵 Dashboard Diff 鏂囦欢绠＄悊鑳藉姏锛坄/api/files`銆乣/api/diff`銆乣/api/content`銆乣/api/convert`銆乣/api/validate`銆乣/ws`锛?|
| `tui` | TUI 鍩虹缁勪欢锛涗緵 `delete_tui` / `batch_rename` / `cstat` 澶嶇敤 |
| `delete_tui` | `delete` 鐨?TUI 浜や簰鐣岄潰 |
| `batch_rename` | `brn`锛涙壒閲忛噸鍛藉悕 |
| `cstat` | `cstat`锛涢」鐩?浠ｇ爜缁熻涓庢壂鎻?|
| `img` | `img`锛涘浘鐗囧帇缂┿€佹牸寮忚浆鎹笌鐭㈤噺鍖栫浉鍏宠兘鍔?|
| `img-moz` | `img` 鐨?`mozjpeg` JPEG 鍚庣锛涗緷璧?`img` |
| `img-turbo` | `img` 鐨?Turbo JPEG 璺緞锛涗緷璧?`img` |

## 鎺ㄨ崘缁勫悎

- 鏃ュ父 CLI锛氶粯璁ゆ瀯寤哄嵆鍙€?- 鏂囦欢杩愮淮澧炲己锛歚--features "lock,protect,crypt,redirect"`
- 鏈湴 Web 绠＄悊锛歚--features "dashboard,diff"`
- alias 浣撶郴锛歚--features "alias,alias-shell-extra"`
- 鎵瑰鐞嗕笌缁熻锛歚--features "batch_rename,cstat"`
- 鍥惧儚澶勭悊锛歚--features "img,img-moz"` 鎴?`--features "img,img-turbo"`
- 鍏ㄥ姛鑳斤細`--all-features`

## Dashboard 缁勪欢鍒嗗眰

濡傛灉浣犲紑鍚簡 `dashboard`锛堝挨鍏舵槸 `dashboard,diff`锛夛紝鍓嶇缁勪欢缁撴瀯澶ц嚧鍒嗕负鍥涘眰锛?
1. **鍏ュ彛灞?*锛歚dashboard-ui/src/main.ts`銆乣dashboard-ui/src/App.vue`
2. **澹冲眰 / 閫氱敤缁勪欢**锛歚CapsuleTabs`銆乣CommandPalette`銆乣ThemeToggle`銆乣DensityToggle`銆乣GlobalFeedback`銆乣Button`銆乣SkeletonTable`
3. **椤跺眰涓氬姟闈㈡澘**锛歚HomePanel`銆乣BookmarksPanel`銆乣PortsPanel`銆乣ProxyPanel`銆乣ConfigPanel`銆乣EnvPanel`銆乣RedirectPanel`銆乣AuditPanel`銆乣DiffPanel`
4. **澶嶅悎瀛愮郴缁?*锛歚EnvPanel` 涓?13 涓瓙缁勪欢锛宍DiffPanel` 涓?9 涓瓙缁勪欢

璇︾粏閫愮粍浠惰鏄庤 `dashboard/Dashboard-Components.md`銆?

