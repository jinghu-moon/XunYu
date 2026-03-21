# 常用示例（复制即可用）

```bash
# 书签
xun set proj D:\Repo\MyProj -t work,rust
xun z proj
xun list --tag work -f table
xun recent --limit 5 -f tsv
xun check --days 90 -f json

# 配置
xun config get proxy.defaultUrl
xun config set tree.defaultDepth 3

# ctx
xun ctx set work --path D:\Repo\MyProj --proxy http://127.0.0.1:7890 --tag work --env RUST_LOG=info
xun ctx set work --env-file .\.env
xun ctx use work
xun ctx show --format json
xun ctx off

# 代理
xun proxy set "http://127.0.0.1:7890" -o cargo,git
xun pon --no-test
xun proxy test http://127.0.0.1:7890 --targets proxy,github.com
xun pst -f table

# 端口
xun ports --range 3000-3999 -f tsv
xun kill 5173,8080 --force
xun ps cargo
xun pkill node --force

# find / acl
xun find D:\Repo --include "*.rs" --exclude target -f table
xun find D:\Repo --size ">=1m" --mtime "<=7d" -f json
xun acl view -p D:\Repo\MyProj
xun acl diff -p D:\Repo\MyProj -r D:\Repo\MyProj.ref

# redirect
xun redirect D:\Downloads --dry-run -f table
xun redirect D:\Downloads --review --dry-run
xun redirect --plan .\xun.plan.json
xun redirect --apply .\xun.plan.json

# 备份与树
xun backup -m "baseline"
xun bak list
xun rst v12-2026-02-23_1030
xun tree -d 2 --size --no-clip

# lock / protect / crypt
xun lock who D:\Repo\MyProj -f table
xun rm D:\Repo\MyProj --unlock --dry-run
xun protect set D:\Repo\MyProj --deny delete,rename --require force,reason
xun encrypt D:\Repo\MyProj --to age1exampleexampleexampleexampleexample

# dashboard
xun serve --port 9527

# diff（CLI）
xun diff .\a.toml .\b.toml --mode auto --diff-algorithm histogram
xun diff .\a.ts .\b.ts --mode ast --format json
xun diff .\a.txt .\b.txt --mode line --ignore-all-space --strip-trailing-cr

# alias（feature: alias）
xun alias add ll "ls -la" --shell ps,bash --tag dev
xun alias ls --json
xun alias app scan --source all --filter code

# brn / cstat / img
xun brn .\docs --regex "^draft_(.*)$" --replace "$1"
xun brn .\docs --seq --apply --yes
xun cstat . --all -f json
xun img -i D:\Images -o D:\Images\webp -f webp -q 82
```

```powershell
# dashboard diff API（示例）
curl "http://127.0.0.1:9527/api/files?path=D:/100_Projects/110_Daily/Xun"
curl "http://127.0.0.1:9527/api/files/search?root=D:/100_Projects/110_Daily/Xun&query=diff&limit=100"
curl -Method POST "http://127.0.0.1:9527/api/validate" `
  -ContentType "application/json" `
  -Body '{"path":"D:/100_Projects/110_Daily/Xun/Cargo.toml"}'
curl -Method POST "http://127.0.0.1:9527/api/convert" `
  -ContentType "application/json" `
  -Body '{"path":"D:/100_Projects/110_Daily/Xun/Cargo.toml","to_format":"json","preview":true}'
```
