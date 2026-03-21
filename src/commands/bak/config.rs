use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct BakConfig {
    #[serde(default)]
    pub(crate) storage: StorageCfg,
    #[serde(default)]
    pub(crate) naming: NamingCfg,
    #[serde(default)]
    pub(crate) retention: RetentionCfg,
    #[serde(default)]
    pub(crate) include: Vec<String>,
    #[serde(default)]
    pub(crate) exclude: Vec<String>,
    #[serde(default, rename = "useGitignore")]
    pub(crate) use_gitignore: bool,
}

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct StorageCfg {
    #[serde(rename = "backupsDir")]
    pub(crate) backups_dir: String,
    pub(crate) compress: bool,
}
impl Default for StorageCfg {
    fn default() -> Self {
        Self {
            backups_dir: "A_backups".into(),
            compress: true,
        }
    }
}

#[derive(Deserialize)]
#[serde(default)]
pub(crate) struct NamingCfg {
    pub(crate) prefix: String,
    #[serde(rename = "dateFormat")]
    pub(crate) date_format: String,
    #[serde(rename = "defaultDesc")]
    pub(crate) default_desc: String,
}
impl Default for NamingCfg {
    fn default() -> Self {
        Self {
            prefix: "v".into(),
            date_format: "yyyy-MM-dd_HHmm".into(),
            default_desc: "backup".into(),
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub(crate) struct RetentionCfg {
    #[serde(rename = "maxBackups")]
    pub(crate) max_backups: usize,
    #[serde(rename = "deleteCount")]
    pub(crate) delete_count: usize,
    /// 每天最多保留几个备份（0=不限）
    #[serde(rename = "keepDaily", default)]
    pub(crate) keep_daily: usize,
    /// 每周最多保留几个备份（0=不限）
    #[serde(rename = "keepWeekly", default)]
    pub(crate) keep_weekly: usize,
    /// 每月最多保留几个备份（0=不限）
    #[serde(rename = "keepMonthly", default)]
    pub(crate) keep_monthly: usize,
}
impl Default for RetentionCfg {
    fn default() -> Self {
        Self {
            max_backups: 50,
            delete_count: 10,
            keep_daily: 0,
            keep_weekly: 0,
            keep_monthly: 0,
        }
    }
}

impl Default for BakConfig {
    fn default() -> Self {
        Self {
            storage: StorageCfg::default(),
            naming: NamingCfg::default(),
            retention: RetentionCfg::default(),
            include: vec![
                "src",
                "public",
                "package.json",
                "package-lock.json",
                "tsconfig.json",
                "tsconfig.node.json",
                "vite.config.ts",
                "index.html",
                ".env",
                ".env.local",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            exclude: vec![
                "node_modules",
                "dist",
                "build",
                ".git",
                ".DS_Store",
                ".log",
                "src/assets/font",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            use_gitignore: false,
        }
    }
}

const DEFAULT_CONFIG_JSON: &str = r#"{
  \"storage\": {
    \"backupsDir\": \"A_backups\",
    \"compress\": true
  },
  \"naming\": {
    \"prefix\": \"v\",
    \"dateFormat\": \"yyyy-MM-dd_HHmm\",
    \"defaultDesc\": \"backup\"
  },
  \"retention\": {
    \"maxBackups\": 50,
    \"deleteCount\": 10
  },
  \"include\": [
    \"src\", \"public\", \"package.json\", \"package-lock.json\",
    \"tsconfig.json\", \"tsconfig.node.json\", \"vite.config.ts\",
    \"index.html\", \".env\", \".env.local\"
  ],
  \"exclude\": [
    \"node_modules\", \"dist\", \"build\", \".git\", \".DS_Store\", \".log\",
    \"src/assets/font\"
  ],
  \"useGitignore\": false
}"#;

pub(crate) const CONFIG_FILE: &str = ".xun-bak.json";
const CONFIG_FILE_LEGACY: &str = ".svconfig.json";

pub(crate) fn load_config(root: &Path) -> BakConfig {
    let cfg_path = root.join(CONFIG_FILE);
    let legacy_path = root.join(CONFIG_FILE_LEGACY);

    // 自动迁移旧配置文件名
    if !cfg_path.exists() && legacy_path.exists()
        && fs::rename(&legacy_path, &cfg_path).is_ok() {
            ui_println!("ℹ Migrated config: .svconfig.json → .xun-bak.json");
        }

    if !cfg_path.exists() {
        let _ = fs::write(&cfg_path, DEFAULT_CONFIG_JSON);
        ui_println!("ℹ Auto-created default config: .xun-bak.json");
    }
    match fs::read_to_string(&cfg_path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => BakConfig::default(),
    }
}
