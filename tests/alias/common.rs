#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use crate::common::*;

// ── 命令构造 ─────────────────────────────────────────────────────────────────

/// 构建 alias 子命令，注入隔离的 APPDATA 目录
pub fn alias_cmd(env: &TestEnv) -> Command {
    let mut cmd = env.cmd();
    let appdata = env.root.join("AppData");
    let _ = fs::create_dir_all(&appdata);
    cmd.env("APPDATA", &appdata);
    cmd
}

/// 返回测试环境中的 aliases.toml 路径
pub fn aliases_toml(env: &TestEnv) -> PathBuf {
    env.root
        .join("AppData")
        .join("xun")
        .join("aliases.toml")
}

/// 返回测试环境中的 shims 目录
pub fn shims_dir(env: &TestEnv) -> PathBuf {
    env.root
        .join("AppData")
        .join("xun")
        .join("shims")
}

/// 运行 `xun alias setup --core-only`
pub fn do_setup(env: &TestEnv) {
    run_ok(alias_cmd(env).args(["alias", "setup", "--core-only"]));
}

/// 在测试目录下创建一个真实的可执行文件（复制 cmd.exe 占位）
pub fn make_fake_exe(env: &TestEnv, name: &str) -> PathBuf {
    let dir = env.root.join("fake_bins");
    let _ = fs::create_dir_all(&dir);
    let src = PathBuf::from(r"C:\Windows\System32\cmd.exe");
    let dst = dir.join(format!("{name}.exe"));
    if !dst.exists() {
        fs::copy(&src, &dst).unwrap();
    }
    dst
}

/// 读取 aliases.toml 文本内容，失败返回空字符串
pub fn read_toml(env: &TestEnv) -> String {
    fs::read_to_string(aliases_toml(env)).unwrap_or_default()
}

/// 断言 shim .exe 和 .shim 文件均存在
pub fn assert_shim_exists(env: &TestEnv, name: &str) {
    let dir = shims_dir(env);
    assert!(
        dir.join(format!("{name}.exe")).exists(),
        "shim exe missing: {name}.exe"
    );
    assert!(
        dir.join(format!("{name}.shim")).exists(),
        "shim file missing: {name}.shim"
    );
}

/// 断言 shim 文件均不存在
pub fn assert_shim_absent(env: &TestEnv, name: &str) {
    let dir = shims_dir(env);
    assert!(
        !dir.join(format!("{name}.exe")).exists(),
        "shim exe should be absent: {name}.exe"
    );
    assert!(
        !dir.join(format!("{name}.shim")).exists(),
        "shim file should be absent: {name}.shim"
    );
}

/// 读取 .shim 文件内容
pub fn read_shim_content(env: &TestEnv, name: &str) -> String {
    let path = shims_dir(env).join(format!("{name}.shim"));
    fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("cannot read shim file: {}", path.display()))
}

/// 断言 .shim 文件包含指定字符串
pub fn assert_shim_contains(env: &TestEnv, name: &str, needle: &str) {
    let content = read_shim_content(env, name);
    assert!(
        content.contains(needle),
        ".shim file for {name} should contain {needle:?}\nactual:\n{content}"
    );
}

/// stdout 字符串
pub fn stdout_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// stderr 字符串
pub fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

/// combined stdout + stderr
pub fn combined_str(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}
