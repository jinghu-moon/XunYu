#![allow(unused_imports)]

mod annotations;
mod common;
mod doctor;
mod profile;
mod run;
mod schema;
mod snapshot;
mod vars;
mod ws;

pub(in crate::commands::dashboard) use annotations::*;
pub(super) use common::*;
pub(in crate::commands::dashboard) use doctor::*;
pub(in crate::commands::dashboard) use profile::*;
pub(in crate::commands::dashboard) use run::*;
pub(in crate::commands::dashboard) use schema::*;
pub(in crate::commands::dashboard) use snapshot::*;
pub(in crate::commands::dashboard) use vars::*;
pub(in crate::commands::dashboard) use ws::*;

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, OnceLock};

use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::json;
use tokio::sync::broadcast;

use crate::commands::env::web_dto::{
    AnnotationBody, AnnotationsPayload, ApiError, ApiSuccess, AuditPayload, AuditQuery,
    DiffPayload, DiffQuery, DoctorBody, DoctorFixPayload, DoctorPayload, ExportLiveQuery,
    ExportQuery, GraphPayload, GraphQuery, ImportBody, ImportPayload, PathUpdateBody, ProfileBody,
    ProfileCaptureBody, ProfilesPayload, RunBody, RunPayload, SchemaAddEnumBody,
    SchemaAddRegexBody, SchemaAddRequiredBody, SchemaPayload, SchemaRemoveBody, ScopeQuery,
    SetVarBody, SnapshotCreateBody, SnapshotPayload, SnapshotPrunePayload, SnapshotPruneQuery,
    SnapshotRestoreBody, StatusPayload, TemplateExpandBody, TemplatePayload, ValidateBody,
    ValidatePayload, VarHistoryPayload, VarHistoryQuery, VarsPayload,
};
use crate::env_core::EnvManager;
use crate::env_core::types::{
    EnvError, EnvEvent, EnvScope, ExportFormat, ImportStrategy, LiveExportFormat,
};

