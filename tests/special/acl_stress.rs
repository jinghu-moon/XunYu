#![cfg(windows)]

#[path = "../support/mod.rs"]
mod common;

mod acl {
    #[path = "../../modules/acl_cases/common.rs"]
    pub mod common;
    #[path = "../../modules/acl_cases/stress.rs"]
    pub mod stress;
}
