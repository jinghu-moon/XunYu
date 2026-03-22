#![cfg(all(windows, feature = "alias"))]

#[path = "alias_cases/mod.rs"]
mod alias;
#[path = "../support/mod.rs"]
mod common;
