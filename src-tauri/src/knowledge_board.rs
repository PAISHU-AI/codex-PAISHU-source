use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

const DEFAULT_PORT: u16 = 8765;
const REQUEST_TIMEOUT_SECS: u64 = 8;
const DEFAULT_COLLECTION: &str = "paishu-global-v2";
const DEFAULT_ACCESS_TIER: &str = "internal";
const DEFAULT_STATUS: &str = "active";
const DEFAULT_OWNER: &str = "PAISHU";
const KNOWLEDGE_RETRIEVAL_ENV: &str = "PAISHU_KNOWLEDGE_RETRIEVAL_DIR";
const KNOWLEDGE_RETRIEVAL_LIST_ENV: &str = "PAISHU_KNOWLEDGE_RETRIEVAL_DIRS";
const KB_CLI_ENV: &str = "PAISHU_KB_CLI";

#[derive(Debug, Clone)]
struct KnowledgeServiceConfig {
    host: String,
    port: u16,
    api_token: String,
}

#[derive(Debug, Clone)]
struct KnowledgePackage {
    root: PathBuf,
    ingest_root: PathBuf,
    status: String,
    access_tier: String,
    owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct KnowledgeDocumentSummary {
    pub id: String,
    pub title: String,
    pub source_uri: String,
    pub owner: String,
    pub status: String,
    pub access_tier: String,
    pub enabled: bool,
    pub chunk_count: u64,
    pub approximate_tokens: u64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct KnowledgeBoard {
    pub refreshed_at: String,
    pub service_status: String,
    pub collection_count: u64,
    pub total_documents: u64,
    pub enabled_documents: u64,
    pub disabled_documents: u64,
    pub chunk_count: u64,
    pub database_bytes: u64,
    pub average_read_ms: u64,
    pub read_success_count: u64,
    pub read_failure_count: u64,
    pub documents: Vec<KnowledgeDocumentSummary>,
    #[serde(default)]
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub struct KnowledgeOverview {
    pub document_id: String,
    pub title: String,
    pub language: String,
    pub overview: String,
    pub source_uri: String,
    pub updated_at: String,
}

pub fn get_knowledge_board() -> AppResult<KnowledgeBoard> {
    request_board("GET", "/v1/dashboard", None)
}

pub fn sync_knowledge_sources() -> AppResult<KnowledgeBoard> {
    let mut messages = sync_default_knowledge_sources();
    let mut board = request_board("GET", "/v1/dashboard", None)?;
    board.messages.append(&mut messages);
    Ok(board)
}

pub fn get_knowledge_overview(document_id: &str) -> AppResult<KnowledgeOverview> {
    if !is_uuid(document_id) {
        return Err(AppError::Config("知识 ID 格式无效".to_string()));
    }
    let path = format!("/v1/documents/{document_id}/overview");
    let config = load_service_config()?;
    let response = request_local_service(&config, "GET", &path, None)?;
    Ok(serde_json::from_str(&response)?)
}

pub fn set_knowledge_enabled(document_id: &str, enabled: bool) -> AppResult<KnowledgeBoard> {
    if !is_uuid(document_id) {
        return Err(AppError::Config("知识 ID 格式无效".to_string()));
    }
    let path = format!("/v1/documents/{document_id}/enabled");
    let body = serde_json::json!({ "enabled": enabled }).to_string();
    request_board("PATCH", &path, Some(&body))
}

fn request_board(method: &str, path: &str, body: Option<&str>) -> AppResult<KnowledgeBoard> {
    let config = load_service_config()?;
    let response = request_local_service(&config, method, path, body)?;
    Ok(serde_json::from_str(&response)?)
}

fn sync_default_knowledge_sources() -> Vec<String> {
    let Some(cli) = detect_paishu_kb_cli() else {
        return vec!["未找到 paishu-kb，同步已跳过；请确认 PAISHU 知识服务已安装。".to_string()];
    };
    let source_roots = detect_knowledge_retrieval_dirs();
    if source_roots.is_empty() {
        return vec!["未找到 knowledge-retrieval 目录，同步已跳过。".to_string()];
    }

    let packages: Vec<KnowledgePackage> = source_roots
        .iter()
        .flat_map(|root| find_governed_knowledge_packages(root))
        .collect();
    if packages.is_empty() {
        return vec![
            "已找到 knowledge-retrieval，但没有发现带 metadata.yml 的知识包。".to_string(),
        ];
    }

    let mut ingested = 0_u64;
    let mut skipped = 0_u64;
    let mut failed = Vec::new();
    for package in packages {
        match run_ingest_package(&cli, &package) {
            Ok(summary) => {
                ingested += summary.ingested;
                skipped += summary.skipped;
                if summary.failed > 0 {
                    failed.push(format!(
                        "{}（{} 个文件失败）",
                        display_name(&package.root),
                        summary.failed
                    ));
                }
            }
            Err(err) => failed.push(format!("{}（{}）", display_name(&package.root), err)),
        }
    }

    let mut messages = vec![format!(
        "已同步本机知识库源：{} 个新增/更新，{} 个跳过。",
        ingested, skipped
    )];
    if !failed.is_empty() {
        messages.push(format!(
            "部分知识包同步失败，不影响其它知识显示：{}",
            failed.join("；")
        ));
    }
    messages
}

fn detect_paishu_kb_cli() -> Option<PathBuf> {
    let env_path = env::var_os(KB_CLI_ENV)
        .map(PathBuf::from)
        .filter(|path| is_executable_file(path));
    if env_path.is_some() {
        return env_path;
    }

    let home = dirs::home_dir()?;
    let bundled =
        home.join("Library/Application Support/PAISHU/knowledge-service/venv/bin/paishu-kb");
    if is_executable_file(&bundled) {
        return Some(bundled);
    }

    which::which("paishu-kb").ok()
}

fn detect_knowledge_retrieval_dirs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = env::var_os(KNOWLEDGE_RETRIEVAL_ENV) {
        candidates.push(PathBuf::from(path));
    }
    if let Some(paths) = env::var_os(KNOWLEDGE_RETRIEVAL_LIST_ENV) {
        candidates.extend(env::split_paths(&paths));
    }
    if let Ok(current_dir) = env::current_dir() {
        candidates.push(current_dir.join("knowledge-retrieval"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("Desktop/GUANGHE-PAISHU/knowledge-retrieval"));
        candidates.push(home.join("Documents/GUANGHE-PAISHU/knowledge-retrieval"));
    }

    dedupe_existing_dirs(candidates)
}

fn dedupe_existing_dirs(candidates: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for candidate in candidates {
        if !candidate.is_dir() {
            continue;
        }
        let canonical = fs::canonicalize(&candidate).unwrap_or(candidate);
        if !dirs.iter().any(|existing| existing == &canonical) {
            dirs.push(canonical);
        }
    }
    dirs
}

fn find_governed_knowledge_packages(root: &Path) -> Vec<KnowledgePackage> {
    let mut packages = Vec::new();
    collect_governed_packages(root, 0, &mut packages);
    packages.sort_by(|left, right| left.root.cmp(&right.root));
    packages
}

fn collect_governed_packages(dir: &Path, depth: usize, packages: &mut Vec<KnowledgePackage>) {
    if depth > 4 || is_hidden_path(dir) {
        return;
    }
    if dir.join("metadata.yml").is_file() && has_knowledge_payload(dir) {
        if let Some(package) = resolve_knowledge_package(dir) {
            packages.push(package);
        }
        return;
    }

    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_governed_packages(&path, depth + 1, packages);
        }
    }
}

fn has_knowledge_payload(dir: &Path) -> bool {
    dir.join("kb").is_dir()
        || dir.join("source").is_dir()
        || dir.join("governance.md").is_file()
        || dir.join("README.md").is_file()
}

fn resolve_knowledge_package(dir: &Path) -> Option<KnowledgePackage> {
    let metadata_path = dir.join("metadata.yml");
    let metadata = fs::read_to_string(metadata_path).ok()?;
    if !metadata_allows_ingestion(&metadata) {
        return None;
    }

    let mode = yaml_nested_value(&metadata, "ingestion", "mode")
        .unwrap_or_else(|| "package".to_string())
        .to_ascii_lowercase();
    let ingest_root = if mode == "kb_only" && dir.join("kb").is_dir() {
        dir.join("kb")
    } else {
        dir.to_path_buf()
    };

    Some(KnowledgePackage {
        root: dir.to_path_buf(),
        ingest_root,
        status: normalize_ingest_status(yaml_top_level_value(&metadata, "status").as_deref()),
        access_tier: normalize_access_tier(
            yaml_top_level_value(&metadata, "access_tier")
                .or_else(|| yaml_top_level_value(&metadata, "confidentiality"))
                .as_deref(),
        ),
        owner: yaml_top_level_value(&metadata, "owner")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_OWNER.to_string()),
    })
}

fn metadata_allows_ingestion(contents: &str) -> bool {
    let enabled = yaml_nested_bool(contents, "ingestion", "enabled");
    let mode = yaml_nested_value(contents, "ingestion", "mode")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if enabled == Some(false) || mode == "quarantine" {
        return false;
    }

    let status = yaml_top_level_value(contents, "status")
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        status.as_str(),
        "draft" | "deprecated" | "archived" | "quarantine"
    ) {
        return enabled == Some(true) && mode != "quarantine";
    }

    true
}

fn yaml_top_level_value(contents: &str, key: &str) -> Option<String> {
    for line in contents.lines() {
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((candidate, value)) = line.split_once(':') else {
            continue;
        };
        if candidate.trim() == key {
            return Some(clean_yaml_scalar(value));
        }
    }
    None
}

fn yaml_nested_value(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let is_top_level = !line.starts_with(char::is_whitespace);
        if is_top_level {
            in_section = trimmed == format!("{section}:");
            continue;
        }
        if !in_section {
            continue;
        }
        let Some((candidate, value)) = trimmed.split_once(':') else {
            continue;
        };
        if candidate.trim() == key {
            return Some(clean_yaml_scalar(value));
        }
    }
    None
}

fn yaml_nested_bool(contents: &str, section: &str, key: &str) -> Option<bool> {
    yaml_nested_value(contents, section, key).and_then(|value| {
        match value.to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    })
}

fn normalize_ingest_status(status: Option<&str>) -> String {
    match status
        .unwrap_or(DEFAULT_STATUS)
        .to_ascii_lowercase()
        .as_str()
    {
        "draft" => "draft",
        "review" => "review",
        "active" => "active",
        "deprecated" => "deprecated",
        _ => DEFAULT_STATUS,
    }
    .to_string()
}

fn normalize_access_tier(access_tier: Option<&str>) -> String {
    match access_tier
        .unwrap_or(DEFAULT_ACCESS_TIER)
        .to_ascii_lowercase()
        .as_str()
    {
        "public" => "public",
        "internal" => "internal",
        "confidential" => "confidential",
        _ => DEFAULT_ACCESS_TIER,
    }
    .to_string()
}

fn clean_yaml_scalar(value: &str) -> String {
    value
        .split('#')
        .next()
        .unwrap_or("")
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string()
}

fn is_hidden_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

#[derive(Debug, Default, Deserialize)]
struct IngestSummary {
    ingested: u64,
    skipped: u64,
    failed: u64,
}

fn run_ingest_package(cli: &Path, package: &KnowledgePackage) -> Result<IngestSummary, String> {
    let output = Command::new(cli)
        .arg("ingest")
        .arg(&package.ingest_root)
        .arg("--collection")
        .arg(DEFAULT_COLLECTION)
        .arg("--access-tier")
        .arg(&package.access_tier)
        .arg("--status")
        .arg(&package.status)
        .arg("--owner")
        .arg(&package.owner)
        .output()
        .map_err(|err| format!("启动 paishu-kb 失败: {err}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(compact_process_error(&stdout, &stderr));
    }

    Ok(parse_ingest_summary(&stdout).unwrap_or_default())
}

fn parse_ingest_summary(stdout: &str) -> Option<IngestSummary> {
    stdout
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<IngestSummary>(line.trim()).ok())
}

fn compact_process_error(stdout: &str, stderr: &str) -> String {
    let source = if stderr.trim().is_empty() {
        stdout
    } else {
        stderr
    };
    let mut message = source
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("未知错误")
        .trim()
        .to_string();
    message.truncate(180);
    message
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名知识包")
        .to_string()
}

fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

fn load_service_config() -> AppResult<KnowledgeServiceConfig> {
    let home =
        dirs::home_dir().ok_or_else(|| AppError::Config("无法定位用户主目录".to_string()))?;
    let path = home.join("Library/Application Support/PAISHU/knowledge-service/config/.env");
    parse_service_config(&fs::read_to_string(path)?)
}

fn parse_service_config(contents: &str) -> AppResult<KnowledgeServiceConfig> {
    let mut host = "127.0.0.1".to_string();
    let mut port = DEFAULT_PORT;
    let mut api_token = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "KNOWLEDGE_SERVICE_HOST" => host = value.trim().to_string(),
            "KNOWLEDGE_SERVICE_PORT" => {
                port = value
                    .trim()
                    .parse()
                    .map_err(|_| AppError::Config("知识服务端口无效".to_string()))?;
            }
            "KNOWLEDGE_SERVICE_API_TOKEN" => api_token = Some(value.trim().to_string()),
            _ => {}
        }
    }
    if host != "127.0.0.1" && host != "localhost" {
        return Err(AppError::Config("知识服务必须绑定本机回环地址".to_string()));
    }
    let api_token = api_token
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Config("知识服务 API Token 未配置".to_string()))?;
    Ok(KnowledgeServiceConfig {
        host,
        port,
        api_token,
    })
}

fn request_local_service(
    config: &KnowledgeServiceConfig,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> AppResult<String> {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
    let timeout = Duration::from_secs(REQUEST_TIMEOUT_SECS);
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let payload = body.unwrap_or("");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}:{}\r\nX-PAISHU-KB-Token: {}\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        config.host,
        config.port,
        config.api_token,
        payload.len(),
        payload
    );
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let (headers, response_body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| AppError::Process("知识服务返回了无效 HTTP 响应".to_string()))?;
    let status_code = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| AppError::Process("知识服务响应缺少状态码".to_string()))?;
    if !(200..300).contains(&status_code) {
        return Err(AppError::Process(format!(
            "知识服务请求失败（HTTP {status_code}）: {response_body}"
        )));
    }
    Ok(response_body.to_string())
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value
            .chars()
            .enumerate()
            .all(|(index, character)| match index {
                8 | 13 | 18 | 23 => character == '-',
                _ => character.is_ascii_hexdigit(),
            })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_local_service_configuration_without_exposing_database_credentials() {
        let config = parse_service_config(
            "DATABASE_URL=postgresql://secret\nKNOWLEDGE_SERVICE_API_TOKEN=test-token\nKNOWLEDGE_SERVICE_PORT=9876\n",
        )
        .unwrap();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9876);
        assert_eq!(config.api_token, "test-token");
    }

    #[test]
    fn rejects_non_local_knowledge_service_hosts() {
        let error = parse_service_config(
            "KNOWLEDGE_SERVICE_HOST=example.com\nKNOWLEDGE_SERVICE_API_TOKEN=test-token\n",
        )
        .unwrap_err();
        assert!(error.to_string().contains("回环地址"));
    }

    #[test]
    fn validates_document_ids_before_writing_state() {
        assert!(is_uuid("d50f8262-c19d-46cd-a001-5d634b692807"));
        assert!(!is_uuid("../../config/.env"));
    }

    #[test]
    fn deserializes_dashboard_contract() {
        let board: KnowledgeBoard = serde_json::from_str(
            r#"{
              "refreshed_at":"2026-07-15T00:00:00Z","service_status":"ok",
              "collection_count":1,"total_documents":49,"enabled_documents":48,
              "disabled_documents":1,"chunk_count":252,"database_bytes":1024,
              "average_read_ms":42,"read_success_count":128,"read_failure_count":3,
              "documents":[],"messages":[]
            }"#,
        )
        .unwrap();
        assert_eq!(board.total_documents, 49);
        assert_eq!(board.average_read_ms, 42);
    }

    #[test]
    fn deserializes_knowledge_overview_contract() {
        let overview: KnowledgeOverview = serde_json::from_str(
            r#"{
              "document_id":"d50f8262-c19d-46cd-a001-5d634b692807",
              "title":"客户素材需求清单","language":"zh-CN",
              "overview":"客户需要提供品牌资料。","source_uri":"/knowledge/client.md",
              "updated_at":"2026-07-15T00:00:00Z"
            }"#,
        )
        .unwrap();
        assert_eq!(overview.language, "zh-CN");
        assert!(overview.overview.contains("品牌资料"));
    }

    #[test]
    fn parses_ingest_summary_from_cli_output() {
        let summary = parse_ingest_summary(
            "ingested source.md revision=1 chunks=2\n{\"ingested\":2,\"skipped\":3,\"failed\":1}\n",
        )
        .unwrap();
        assert_eq!(summary.ingested, 2);
        assert_eq!(summary.skipped, 3);
        assert_eq!(summary.failed, 1);
    }

    #[test]
    fn discovers_governed_knowledge_package_roots() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("knowledge-retrieval");
        let package = root.join("09_RAG_MATERIALS/example");
        fs::create_dir_all(package.join("kb")).unwrap();
        fs::write(package.join("metadata.yml"), "slug: example\n").unwrap();
        fs::create_dir_all(root.join("09_RAG_MATERIALS/not-a-package/kb")).unwrap();

        let packages = find_governed_knowledge_packages(&root);

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].root, package);
    }

    #[test]
    fn kb_only_packages_ingest_the_clean_kb_layer() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp
            .path()
            .join("knowledge-retrieval/09_RAG_MATERIALS/example");
        fs::create_dir_all(package.join("kb")).unwrap();
        fs::create_dir_all(package.join("source")).unwrap();
        fs::write(
            package.join("metadata.yml"),
            "status: review\naccess_tier: confidential\nowner: PAISHU KB\ningestion:\n  enabled: true\n  mode: kb_only\n",
        )
        .unwrap();

        let packages = find_governed_knowledge_packages(&temp.path().join("knowledge-retrieval"));

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].ingest_root, package.join("kb"));
        assert_eq!(packages[0].status, "review");
        assert_eq!(packages[0].access_tier, "confidential");
        assert_eq!(packages[0].owner, "PAISHU KB");
    }

    #[test]
    fn skips_quarantined_or_disabled_packages() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("knowledge-retrieval");
        let disabled = root.join("09_RAG_MATERIALS/disabled");
        let quarantined = root.join("09_RAG_MATERIALS/quarantined");
        fs::create_dir_all(disabled.join("kb")).unwrap();
        fs::create_dir_all(quarantined.join("kb")).unwrap();
        fs::write(
            disabled.join("metadata.yml"),
            "status: review\ningestion:\n  enabled: false\n  mode: kb_only\n",
        )
        .unwrap();
        fs::write(
            quarantined.join("metadata.yml"),
            "status: draft\ningestion:\n  enabled: false\n  mode: quarantine\n",
        )
        .unwrap();

        let packages = find_governed_knowledge_packages(&root);

        assert!(packages.is_empty());
    }

    #[test]
    fn parses_nested_ingestion_metadata() {
        let metadata = "status: \"review\"\ningestion:\n  enabled: true\n  mode: kb_only\n";

        assert!(metadata_allows_ingestion(metadata));
        assert_eq!(
            yaml_nested_value(metadata, "ingestion", "mode"),
            Some("kb_only".to_string())
        );
        assert_eq!(
            normalize_ingest_status(Some("reviewed_for_structure")),
            "active"
        );
        assert_eq!(normalize_access_tier(Some("internal")), "internal");
    }

    #[test]
    fn deduplicates_existing_knowledge_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("knowledge-retrieval");
        fs::create_dir_all(&root).unwrap();

        let dirs = dedupe_existing_dirs(vec![
            root.clone(),
            root.clone(),
            temp.path().join("missing"),
        ]);

        assert_eq!(dirs.len(), 1);
        assert!(dirs[0].ends_with("knowledge-retrieval"));
    }

    #[test]
    #[ignore = "requires the deployed localhost PAISHU knowledge service"]
    fn reads_the_live_local_dashboard_contract() {
        let board = get_knowledge_board().unwrap();
        assert_eq!(board.service_status, "ok");
        assert!(board.total_documents > 0);
        assert_eq!(board.documents.len() as u64, board.total_documents);
        let overview = get_knowledge_overview(&board.documents[0].id).unwrap();
        assert_eq!(overview.document_id, board.documents[0].id);
        assert!(!overview.overview.trim().is_empty());
    }
}
