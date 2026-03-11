pub(crate) mod acl_cmd;
#[cfg(feature = "alias")]
pub(crate) mod alias;
pub(crate) mod app_config;
pub(crate) mod bak;
pub(crate) mod bookmarks;
pub(crate) mod completion;
pub(crate) mod ctx;
pub(crate) mod delete;
pub(crate) mod env;
pub(crate) mod find;
pub(crate) mod ports;
pub(crate) mod proxy;
pub(crate) mod tree;
pub(crate) mod video;

#[cfg(feature = "redirect")]
pub(crate) mod redirect;

#[cfg(feature = "cstat")]
pub(crate) mod cstat;

#[cfg(feature = "batch_rename")]
pub(crate) mod batch_rename;

#[cfg(feature = "img")]
pub(crate) mod img;

#[cfg(feature = "crypt")]
pub(crate) mod crypt;
#[cfg(feature = "crypt")]
pub(crate) mod vault;
#[cfg(feature = "dashboard")]
pub(crate) mod dashboard;
#[cfg(feature = "diff")]
pub(crate) mod diff;
#[cfg(feature = "fs")]
pub(crate) mod fs;
#[cfg(feature = "lock")]
pub(crate) mod lock;
#[cfg(feature = "protect")]
pub(crate) mod protect;

mod dispatch;

pub(crate) use dispatch::dispatch;
