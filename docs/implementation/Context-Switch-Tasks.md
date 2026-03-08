# xun ctx 鈥?瀹炵幇璁″垝锛坱asks锛?
> 渚濇嵁锛歔Context-Switch-Design.md](./Context-Switch-Design.md)  
> 鏍囪锛歚[ ]` 寰呭姙銆€`[-]` 杩涜涓€€`[x]` 瀹屾垚
> 琛ュ厖锛?026-02锛夛細Dashboard Web UI 杩唬涓嶅奖鍝?ctx 浠诲姟娓呭崟銆?
---

## Phase 0锛氬熀纭€楠ㄦ灦涓庡瓨鍌?
### P0.1 CLI 鍏ュ彛涓庤矾鐢?
- [x] `src/cli.rs`锛氭柊澧?`CtxCmd` + `CtxSubCommand`
  - 瀛愬懡浠わ細`set` / `use` / `off` / `list` / `show` / `del` / `rename`
  - `ctx use <name>` 浣滀负婵€娲诲叆鍙ｏ紙瑙勯伩 `argh` 鍐茬獊锛?- [x] `src/commands/mod.rs`锛氭敞鍐?`ctx` 璺敱锛堥粯璁?feature锛?- [x] `src/commands/ctx.rs`锛氭柊澧炴ā鍧楁枃浠讹紙鍛戒护瀹炵幇锛?
### P0.2 鏁版嵁妯″瀷涓庢枃浠惰矾寰?
- [x] `src/ctx_store.rs`锛堟垨 `src/context.rs`锛夛細瀹氫箟 `CtxStore` / `CtxProfile` / `CtxProxy` / `CtxSession`
- [x] Profile 瀛樺偍璺緞锛歚~/.xun.ctx.json`锛屾敮鎸?`XUN_CTX_FILE` 瑕嗙洊
- [x] 浼氳瘽鏂囦欢璺緞锛?*鍙** `XUN_CTX_STATE`锛堢敱 shell wrapper 鐢熸垚锛?- [x] JSON 璇诲啓閲囩敤鍘熷瓙鍐欙紙`tmp + rename`锛?
---

## Phase 1锛氳矾寰?+ 浠ｇ悊 + 鏍囩锛堟牳蹇冨姛鑳斤級

### P1.1 ctx set / list / show / del / rename

- [x] `ctx set`锛氬悎骞舵洿鏂拌涔夛紙鏈紶瀛楁淇濈暀锛?  - `--path` 鏂板缓蹇呭～
  - `--proxy` 鏀寔 `keep|off|<url>`锛堟柊寤洪粯璁?`keep`锛?  - `--tag -` 娓呯┖鏍囩
  - 淇濈暀瀛楁牎楠岋紙`set/use/off/list/show/del/delete/rename/help`锛?- [x] `ctx list`锛氬垪鍑?profile 鍚嶏紙鍙€夊姞 `--format`锛?- [x] `ctx show [name]`锛氭樉绀鸿鎯咃紙榛樿褰撳墠 active锛?- [x] `ctx del` / `ctx rename`

### P1.2 ctx use锛堟縺娲伙級

- [x] 璇诲彇 profile锛屾牎楠?path 瀛樺湪
- [x] `XUN_CTX_STATE` 鏈缃?鈫?鎶ラ敊骞舵彁绀哄姞杞?`xun init`
- [x] 鍐欏叆浼氳瘽鏂囦欢锛?  - `previous_dir = current_dir()`
  - `previous_env`锛歚XUN_DEFAULT_TAG`銆乣XUN_CTX`銆佷互鍙?profile.env 涓殑 key
  - `previous_proxy`锛氫紭鍏?env锛圚TTP_PROXY/NO_PROXY锛夛紝鍥為€€ `.xun.proxy.json`
  - `proxy_changed`锛氬綋 `proxy.mode = set/off` 缃?true
- [x] 杈撳嚭榄旀硶琛岋紙鎸夐『搴忥級锛?  - `__CD__:<profile.path>`
  - 浠ｇ悊锛氬鐢?`pon/poff` 鏍稿績閫昏緫杈撳嚭 `__ENV_SET__/__ENV_UNSET__`
  - `__ENV_SET__:XUN_DEFAULT_TAG=<tags>` 鎴?`__ENV_UNSET__`
  - `__ENV_SET__:XUN_CTX=<name>`

### P1.3 ctx off锛堣繕鍘燂級

- [x] `XUN_CTX_STATE` 鏈缃垨鏂囦欢涓嶅瓨鍦?鈫?鎻愮ず鈥滄棤婵€娲?profile鈥濆苟閫€鍑?0
- [x] 浠ｇ悊杩樺師锛?  - `proxy_changed=false` 鈫?涓嶅彉
  - 鏈?`previous_proxy` 鈫?`proxy set` 鎭㈠
  - 鍚﹀垯 鈫?`proxy del`
- [x] 鎭㈠ env锛坄previous_env` 鏈夊€?鈫?`__ENV_SET__`锛涙棤鍊?鈫?`__ENV_UNSET__`锛?- [x] `previous_dir` 瀛樺湪涓旀湁鏁?鈫?`__CD__:<previous_dir>`锛屽惁鍒?stderr 鎻愮ず骞惰烦杩?- [x] `__ENV_UNSET__:XUN_CTX_STATE` + 鍒犻櫎浼氳瘽鏂囦欢

### P1.4 榛樿鏍囩鐢熸晥

- [x] `list` / `z` / `recent`锛氭湭鏄惧紡浼?`--tag` 鏃惰鍙?`XUN_DEFAULT_TAG`

---

## Phase 1.5锛歋hell 闆嗘垚涓庤ˉ鍏?
### P1.5.1 Shell wrapper

- [x] `xun init` 杈撳嚭鑴氭湰锛氭柊澧?`ctx` 鍒悕锛屽苟璁剧疆 `XUN_CTX_STATE`
- [x] Bash/Zsh wrapper锛氬崌绾т负閫愯瑙ｆ瀽锛坄__CD__/__ENV_SET__/__ENV_UNSET__`锛?- [x] `xun.sh`锛氬悓姝ュ崌绾у琛岃В鏋?+ `ctx` 鍒悕锛堝吋瀹归潪 `xun init` 鐢ㄦ硶锛?
### P1.5.2 Completion

- [x] `__complete` 璺敱锛?  - `ctx <TAB>` 瀛愬懡浠よˉ鍏?  - `ctx use/del/show/rename <TAB>` profile 鍚嶈ˉ鍏紙璇诲彇 ctx store锛?  - `ctx set --proxy <TAB>` 琛ュ叏 `off|keep`
- [x] `xun init` 闈欐€佽ˉ鍏細杩藉姞 `ctx` 瀛愬懡浠ゅ垪琛?
---

## Phase 2锛氶€氱敤鐜鍙橀噺锛堟墿灞曪級

- [x] `ctx set --env KEY=VALUE`锛堝彲閲嶅锛?- [x] `ctx set --env-file <path>`锛坄.env` 瀵煎叆锛?- [x] `ctx use` 杈撳嚭 `__ENV_SET__` 璁剧疆鍙橀噺骞惰褰曟棫鍊?- [x] `ctx off` 鎸?`previous_env` 绮剧‘杩樺師

---

## Phase 3锛氭祴璇曚笌鏂囨。

### P3.1 娴嬭瘯

- [x] 鍗曞厓娴嬭瘯锛歝tx store 璇诲啓/merge 璇箟/淇濈暀瀛楁牎楠?- [x] 闆嗘垚娴嬭瘯锛歚ctx set/use/off` 榄旀硶琛屽簭鍒椼€乻ession 杩樺師銆侀粯璁?tag 鐢熸晥
- [x] 杈圭紭鐢ㄤ緥锛氳矾寰勫惈绌烘牸銆乣ctx off` 浜屾璋冪敤

### P3.2 鏂囨。鍚屾

- [x] `../../README.md` 涓?`../../intro/cli/Commands.md` 琛ュ厖 `ctx` 鍛戒护璇存槑
- [x] `./Test-Env.md` 澧炲姞 ctx 娴嬭瘯姝ラ

---

## 渚濊禆鍏崇郴锛堢畝鍥撅級

```
P0.1 鈹€鈫?P0.2 鈹€鈫?P1.1 鈹€鈫?P1.2 鈹€鈫?P1.3 鈹€鈹攢鈫?P1.4
                                      鈹斺攢鈫?P1.5 (Shell/Completion)
P1.* 鈹€鈫?P2 (env 鎵╁睍) 鈹€鈫?P3 (娴嬭瘯/鏂囨。)
```

