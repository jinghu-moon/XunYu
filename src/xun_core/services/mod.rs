//! Services — 业务逻辑层
//!
//! 每个 service 封装一个命令域的核心业务逻辑，
//! CommandSpec / Operation 实现通过 service 调用具体功能。

pub mod proxy;
pub mod config;
pub mod ctx;
pub mod backup;
pub mod bookmark;
pub mod env;
pub mod acl;

#[cfg(feature = "alias")]
pub mod alias;
#[cfg(feature = "batch_rename")]
pub mod brn;
#[cfg(feature = "crypt")]
pub mod vault;
