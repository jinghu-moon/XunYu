# 瀹夎涓庢瀯寤?
## 鐜瑕佹眰

- Windows 10 / 11
- Rust stable锛圡SVC 宸ュ叿閾撅級
- 濡傛灉瑕佹瀯寤?Dashboard锛歂ode.js + `pnpm`

## 鏂瑰紡 A锛氫粠婧愮爜鏋勫缓

```bash
git clone <your-repo-url>
cd Xun

# 榛樿鑳藉姏
cargo build --release

# 鏂囦欢杩愮淮澧炲己
cargo build --release --features "lock,protect,crypt,redirect"

# Dashboard + Diff锛堟帹鑽愶級
cargo build --release --features "dashboard,diff"

# alias 浣撶郴
cargo build --release --features "alias,alias-shell-extra"

# 鍥惧儚澶勭悊锛圝PEG 璧?mozjpeg锛?cargo build --release --features "img,img-moz"

# 鍥惧儚澶勭悊锛圝PEG 璧?turbo 璺緞锛?cargo build --release --features "img,img-turbo"

# 鍏ㄥ姛鑳?cargo build --release --all-features
```

浜х墿浣嶇疆锛歚target/release/xun.exe`

## 鏂瑰紡 B锛氬畨瑁呭埌 Cargo bin

```bash
cd Xun

# 榛樿鑳藉姏
cargo install --path . --locked

# Dashboard + Diff
cargo install --path . --locked --features "dashboard,diff"

# alias 浣撶郴
cargo install --path . --locked --features "alias,alias-shell-extra"

# 鍏ㄥ姛鑳?cargo install --path . --locked --all-features
```

瀹夎鍚庨€氬父浣嶄簬锛歚%USERPROFILE%\.cargo\bin\xun.exe`

## 鏋勫缓 Dashboard 鍓嶇

`--features dashboard` 浼氭妸 `dashboard-ui/dist/` 闈欐€佽祫婧愬祵鍏ュ埌 Rust 浜岃繘鍒朵腑銆傚洜姝や粠婧愮爜鏋勫缓 Dashboard 鏃讹紝璇峰厛鐢熸垚鍓嶇浜х墿锛?
```bash
corepack enable
pnpm -C dashboard-ui install
pnpm -C dashboard-ui build
```

闅忓悗鍐嶆墽琛?Rust 鏋勫缓锛屼緥濡傦細

```bash
cargo build --release --features "dashboard,diff"
```

## 閫夋嫨寤鸿

- 鍙敤 CLI锛氶粯璁ゆ瀯寤哄嵆鍙€?- 瑕佺湅瀹屾暣 Web UI锛氫紭鍏?`dashboard,diff`锛屽洜涓?`DiffPanel` 鍙婂叾鏂囦欢绠＄悊鑳藉姏渚濊禆 `diff`銆?- 瑕佽 Dashboard 缁勪欢鑱岃矗锛氭瀯寤哄畬鎴愬悗锛岄厤鍚?`dashboard/Dashboard-Components.md` 涓€璧风湅鏈€鐪佹椂闂淬€?

