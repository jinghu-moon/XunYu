use super::*;

// --- Ports ---

#[derive(Serialize)]
pub(in crate::commands::dashboard) struct PortsResponse {
    tcp: Vec<ports::PortInfo>,
    udp: Vec<ports::PortInfo>,
}

pub(in crate::commands::dashboard) async fn list_ports() -> Json<PortsResponse> {
    let tcp = ports::list_tcp_listeners();
    let udp = ports::list_udp_endpoints();
    update_port_icon_cache(&tcp, &udp);
    Json(PortsResponse { tcp, udp })
}

pub(in crate::commands::dashboard) async fn kill_port(Path(port): Path<u16>) -> StatusCode {
    let all: Vec<_> = ports::list_tcp_listeners()
        .into_iter()
        .chain(ports::list_udp_endpoints())
        .filter(|p| p.port == port)
        .collect();
    if all.is_empty() {
        return StatusCode::NOT_FOUND;
    }
    for p in &all {
        let _ = ports::terminate_pid(p.pid);
    }
    StatusCode::OK
}

pub(in crate::commands::dashboard) async fn kill_pid(Path(pid): Path<u32>) -> Response {
    match ports::terminate_pid(pid) {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            let code = match e.as_str() {
                "not found" => StatusCode::NOT_FOUND,
                "access denied" => StatusCode::FORBIDDEN,
                _ => StatusCode::BAD_REQUEST,
            };
            (code, e).into_response()
        }
    }
}

// --- Port Icons ---

const ICON_CACHE_CONTROL: &str = "private, max-age=300";

#[derive(Deserialize)]
pub(in crate::commands::dashboard) struct IconQuery {
    size: Option<u32>,
}

fn port_icon_cache() -> &'static RwLock<HashMap<u32, String>> {
    static CACHE: OnceLock<RwLock<HashMap<u32, String>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn update_port_icon_cache(tcp: &[ports::PortInfo], udp: &[ports::PortInfo]) {
    let mut map = port_icon_cache().write().unwrap_or_else(|e| e.into_inner());
    map.clear();
    for p in tcp.iter().chain(udp.iter()) {
        if !p.exe_path.is_empty() {
            map.insert(p.pid, p.exe_path.clone());
        }
    }
}

fn icon_bytes_cache() -> &'static RwLock<HashMap<String, Vec<u8>>> {
    static CACHE: OnceLock<RwLock<HashMap<String, Vec<u8>>>> = OnceLock::new();
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

fn icon_cache() -> &'static IconCache {
    static CACHE: OnceLock<IconCache> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut cache = IconCache::with_app_name("xun").unwrap_or_else(|_| {
            let dir = std::env::temp_dir().join("xun").join("icon_cache");
            IconCache::new(dir).expect("icon cache")
        });
        cache.set_format(ImageFormat::Webp);
        cache
    })
}

fn normalize_icon_size(size: Option<u32>) -> u32 {
    match size {
        Some(s) if s > 0 => s.clamp(16, 256),
        _ => 0,
    }
}

fn exe_mtime_secs(path: &str) -> u64 {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    let modified = match meta.modified() {
        Ok(t) => t,
        Err(_) => return 0,
    };
    match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(_) => 0,
    }
}

fn sized_cache_key(path: &str, size: u32) -> String {
    let mtime = exe_mtime_secs(path);
    format!("{path}|{size}|{mtime}")
}

fn fallback_icon_webp(size: u32) -> Result<Vec<u8>, win_icon_extractor::IconError> {
    let size = if size > 0 { size } else { 32 };
    let data = extract_stock_icon_sized(StockIcon::Application, size)?;
    encode_webp(&data.rgba, data.width, data.height)
}

pub(in crate::commands::dashboard) async fn port_icon(
    Path(pid): Path<u32>,
    Query(q): Query<IconQuery>,
) -> Response {
    let size = normalize_icon_size(q.size);
    let exe_path = {
        let map = port_icon_cache().read().unwrap_or_else(|e| e.into_inner());
        map.get(&pid).cloned().unwrap_or_default()
    };

    let result =
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, win_icon_extractor::IconError> {
            if !exe_path.is_empty() {
                if size > 0 {
                    let key = sized_cache_key(&exe_path, size);
                    if let Some(bytes) = icon_bytes_cache()
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&key)
                        .cloned()
                    {
                        return Ok(bytes);
                    }
                    let data = extract_icon_with_size(&exe_path, size)?;
                    let bytes = encode_webp(&data.rgba, data.width, data.height)?;
                    icon_bytes_cache()
                        .write()
                        .unwrap_or_else(|e| e.into_inner())
                        .insert(key, bytes.clone());
                    return Ok(bytes);
                }

                if let Ok(file) = icon_cache().extract_to_file(&exe_path) {
                    if let Ok(bytes) = std::fs::read(&file) {
                        return Ok(bytes);
                    }
                }
            }
            fallback_icon_webp(size)
        })
        .await;

    match result {
        Ok(Ok(bytes)) => (
            [
                (header::CONTENT_TYPE, "image/webp"),
                (header::CACHE_CONTROL, ICON_CACHE_CONTROL),
            ],
            bytes,
        )
            .into_response(),
        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
