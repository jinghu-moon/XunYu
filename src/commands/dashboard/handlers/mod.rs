#![allow(unused_imports)]

mod audit;
mod bookmarks;
mod common;
#[path = "config.rs"]
mod config_handlers;
mod diagnostics;
#[cfg(feature = "diff")]
mod files;
#[path = "ports.rs"]
mod ports_handlers;
#[path = "proxy.rs"]
mod proxy_handlers;
mod recipes;
mod workspaces;
mod ws;

pub(in crate::commands::dashboard) use audit::*;
pub(in crate::commands::dashboard) use bookmarks::*;
pub(in crate::commands::dashboard) use config_handlers::*;
pub(in crate::commands::dashboard) use diagnostics::*;
#[cfg(feature = "diff")]
pub(in crate::commands::dashboard) use files::*;
pub(in crate::commands::dashboard) use ports_handlers::*;
pub(in crate::commands::dashboard) use proxy_handlers::*;
pub(in crate::commands::dashboard) use recipes::*;
pub(in crate::commands::dashboard) use workspaces::*;
pub(in crate::commands::dashboard) use ws::*;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant, UNIX_EPOCH};

use win_icon_extractor::{
    IconCache, ImageFormat, StockIcon, encode_webp, extract_icon_with_size,
    extract_stock_icon_sized,
};

use crate::commands::proxy;
use crate::config;
use crate::model::{Entry, ImportMode, IoFormat, ListItem, parse_import_mode, parse_io_format};
use crate::ports;
use crate::store;
use crate::util::{has_cmd, parse_tags};
