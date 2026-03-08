# 寮€鍙戜笌娴嬭瘯

- 缁熶竴娴嬭瘯鐜锛堥殧绂?db/config/鏂囦欢锛夛細`./Test-Env.md`
- 涓€閿垱寤虹幆澧冿細`tools/test-env.ps1 -Reset -Bin .\target\debug\xun.exe`
- 鎵归噺鍛戒护鍥炲綊锛歚tools/run-test-env.ps1`
- 鐢熸垚鍛戒护绀轰緥锛堣嚜鍔ㄧ敓鎴愶級锛歚python tools/gen_readme_commands.py`锛堣緭鍑哄埌 `../../intro/cli/Commands-Generated.md`锛?- 鐢熸垚鍛戒护閫熸煡琛細`python tools/gen_commands_md.py`锛堣緭鍑哄埌 `../../intro/cli/Commands.md`锛?- Dashboard 鍓嶇鏋勫缓锛歚cd dashboard-ui && npm install && npm run build`
- Dashboard 鑱旇皟寮€鍙戯細`cargo run --features dashboard -- serve` + `cd dashboard-ui && npm run dev`

