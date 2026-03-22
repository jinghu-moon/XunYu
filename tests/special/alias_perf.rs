#![cfg(all(windows, feature = "alias"))]

#[path = "../support/mod.rs"]
mod common;

mod alias {
    #[path = "../../modules/alias_cases/common.rs"]
    pub mod common;
    #[path = "../../modules/alias_cases/perf.rs"]
    pub mod perf;
}
