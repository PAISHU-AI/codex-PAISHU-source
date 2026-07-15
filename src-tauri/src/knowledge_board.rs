use crate::error::{AppError, AppResult};
use chrono::Local;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    env, fs,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::{Component, Path, PathBuf},
    process::Command,
    time::Duration,
};
use url::Url;

const DEFAULT_PORT: u16 = 8765;
const REQUEST_TIMEOUT_SECS: u64 = 8;
const INGEST_TIMEOUT_SECS: u64 = 180;
const DEFAULT_COLLECTION: &str = "paishu-global-v2";
const DEFAULT_ACCESS_TIER: &str = "internal";
const DEFAULT_STATUS: &str = "active";
const DEFAULT_OWNER: &str = "PAISHU";
const KNOWLEDGE_RETRIEVAL_ENV: &str = "PAISHU_KNOWLEDGE_RETRIEVAL_DIR";
const KNOWLEDGE_RETRIEVAL_LIST_ENV: &str = "PAISHU_KNOWLEDGE_RETRIEVAL_DIRS";
const KNOWLEDGE_TRASH_DIR: &str = "trash";
const TOMBSTONE_FILE: &str = "tombstone.json";

#[derive(Debug, Clone)]
struct KnowledgeServiceConfig {
    host: String,
    port: u16,
    api_token: String,
}

#[derive(Debug, Clone)]
struct KnowledgePackage {
    namespace: String,
    title: String,
    root: PathBuf,
    ingest_root: PathBuf,
    status: String,
    access_tier: String,
    owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KnowledgeTombstone {
    document_id: String,
    title: String,
    source_uri: String,
    archived_path: Option<String>,
    deleted_at: String,
}

#[derive(Debug)]
struct ArchivedKnowledgeSource {
    original_path: PathBuf,
    archived_path: Option<PathBuf>,
    tombstone_path: PathBuf,
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
    #[serde(default)]
    pub package_name: String,
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

pub fn open_knowledge_source(document_id: &str) -> AppResult<String> {
    let document = resolve_visible_document(document_id)?;
    let (_, source_path) = resolve_governed_source(&document.source_uri, true)?;
    reveal_path(&source_path)?;
    Ok(source_path.to_string_lossy().to_string())
}

pub fn delete_knowledge(document_id: &str) -> AppResult<KnowledgeBoard> {
    let document = resolve_visible_document(document_id)?;
    let source_roots = detect_knowledge_retrieval_dirs();
    let archived = archive_knowledge_source(&document, &source_roots)?;

    let mut board = match set_knowledge_enabled(document_id, false) {
        Ok(board) => board,
        Err(error) => {
            rollback_archived_source(&archived);
            return Err(error);
        }
    };
    let message = if archived.archived_path.is_some() {
        format!("已删除知识“{}”，源文件已移至知识回收站。", document.title)
    } else {
        format!("已删除知识“{}”；源文件原本已不存在。", document.title)
    };
    board.messages.push(message);
    Ok(board)
}

fn request_board(method: &str, path: &str, body: Option<&str>) -> AppResult<KnowledgeBoard> {
    let config = load_service_config()?;
    let response = request_local_service(&config, method, path, body)?;
    let mut board: KnowledgeBoard = serde_json::from_str(&response)?;
    enrich_knowledge_packages(&mut board);
    hide_deleted_knowledge(&mut board);
    Ok(board)
}

fn sync_default_knowledge_sources() -> Vec<String> {
    let config = match load_service_config() {
        Ok(config) => config,
        Err(error) => return vec![format!("无法读取 PAISHU 知识服务配置：{error}")],
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

    let mut manifest = match load_document_manifest(&config) {
        Ok(manifest) => manifest,
        Err(error) => return vec![format!("无法读取知识内容清单，同步已跳过：{error}")],
    };

    let mut ingested = 0_u64;
    let mut skipped = 0_u64;
    let mut failed = Vec::new();
    for package in packages {
        match run_ingest_package(&config, &package, &mut manifest) {
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
        namespace: display_name(dir),
        title: yaml_top_level_value(&metadata, "title")
            .or_else(|| yaml_top_level_value(&metadata, "name"))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| display_name(dir)),
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

#[derive(Debug, Default)]
struct IngestSummary {
    ingested: u64,
    skipped: u64,
    failed: u64,
}

#[derive(Debug, Deserialize)]
struct KnowledgeManifest {
    content_hashes: HashMap<String, String>,
}

fn load_document_manifest(config: &KnowledgeServiceConfig) -> AppResult<HashMap<String, String>> {
    let path = format!("/v1/documents/manifest?collection_slug={DEFAULT_COLLECTION}");
    let response = request_local_service(config, "GET", &path, None)?;
    Ok(serde_json::from_str::<KnowledgeManifest>(&response)?.content_hashes)
}

fn run_ingest_package(
    config: &KnowledgeServiceConfig,
    package: &KnowledgePackage,
    manifest: &mut HashMap<String, String>,
) -> Result<IngestSummary, String> {
    let files = find_clean_knowledge_files(&package.ingest_root);
    if files.is_empty() {
        return Err("kb 目录内没有可同步的 UTF-8 文本文件".to_string());
    }

    let mut summary = IngestSummary::default();
    for path in files {
        match upsert_knowledge_file(config, package, &path, manifest) {
            Ok(true) => summary.ingested += 1,
            Ok(false) => summary.skipped += 1,
            Err(_) => summary.failed += 1,
        }
    }
    Ok(summary)
}

fn find_clean_knowledge_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_clean_knowledge_files(root, &mut files);
    files.sort();
    files
}

fn collect_clean_knowledge_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_hidden_path(&path) {
            continue;
        }
        if path.is_dir() {
            collect_clean_knowledge_files(&path, files);
            continue;
        }
        let supported = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "md" | "txt" | "json" | "yaml" | "yml" | "csv"
                )
            });
        if supported {
            files.push(path);
        }
    }
}

fn knowledge_external_id(package: &KnowledgePackage, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(&package.ingest_root)
        .map_err(|_| "知识文件不在包的入库目录内".to_string())?;
    Ok(format!(
        "{}/{}",
        package.namespace,
        relative.to_string_lossy().replace('\\', "/")
    ))
}

fn upsert_knowledge_file(
    config: &KnowledgeServiceConfig,
    package: &KnowledgePackage,
    path: &Path,
    manifest: &mut HashMap<String, String>,
) -> Result<bool, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let content = content.trim();
    if content.is_empty() {
        return Ok(false);
    }
    if content.len() > 2_000_000 {
        return Err(format!("{} 超过单文件 2 MB 限制", path.display()));
    }

    let external_id = knowledge_external_id(package, path)?;
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    if manifest.get(&external_id) == Some(&content_hash) {
        return Ok(false);
    }

    let relative = path
        .strip_prefix(&package.ingest_root)
        .map_err(|_| "知识文件不在包的入库目录内".to_string())?;
    let source_uri = Url::from_file_path(path)
        .map_err(|_| format!("无法生成知识文件 URI: {}", path.display()))?
        .to_string();
    let title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名知识")
        .replace('_', " ");
    let category = relative
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .unwrap_or("root");
    let body = serde_json::json!({
        "collection_slug": DEFAULT_COLLECTION,
        "external_id": external_id,
        "title": title,
        "content": content,
        "source_uri": source_uri,
        "metadata": {
            "package": package.namespace,
            "package_title": package.title,
            "relative_path": relative.to_string_lossy().replace('\\', "/"),
            "category": category,
            "source_extension": path.extension().and_then(|value| value.to_str()).unwrap_or("")
        },
        "owner": package.owner,
        "status": package.status,
        "access_tier": package.access_tier,
        "language": "zh-CN",
        "last_reviewed_at": Local::now().date_naive().to_string()
    })
    .to_string();
    request_local_service_with_timeout(
        config,
        "POST",
        "/v1/documents/upsert",
        Some(&body),
        INGEST_TIMEOUT_SECS,
    )
    .map_err(|error| error.to_string())?;
    manifest.insert(external_id, content_hash);
    Ok(true)
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("未命名知识包")
        .to_string()
}

fn enrich_knowledge_packages(board: &mut KnowledgeBoard) {
    let packages: Vec<KnowledgePackage> = detect_knowledge_retrieval_dirs()
        .iter()
        .flat_map(|root| find_governed_knowledge_packages(root))
        .collect();
    for document in &mut board.documents {
        let Ok(source_path) = source_path_from_uri(&document.source_uri) else {
            continue;
        };
        if let Some(package) = packages
            .iter()
            .filter(|package| source_path.starts_with(&package.ingest_root))
            .max_by_key(|package| package.ingest_root.components().count())
        {
            document.package_name = package.title.clone();
        }
    }
}

fn resolve_visible_document(document_id: &str) -> AppResult<KnowledgeDocumentSummary> {
    if !is_uuid(document_id) {
        return Err(AppError::Config("知识 ID 格式无效".to_string()));
    }
    get_knowledge_board()?
        .documents
        .into_iter()
        .find(|document| document.id == document_id)
        .ok_or_else(|| AppError::Config("未找到指定知识，可能已经被删除".to_string()))
}

fn source_path_from_uri(source_uri: &str) -> AppResult<PathBuf> {
    let source_path = if source_uri.starts_with("file:") {
        Url::parse(source_uri)
            .map_err(|_| AppError::Config("知识来源 URI 无效".to_string()))?
            .to_file_path()
            .map_err(|_| AppError::Config("知识来源不是本机文件".to_string()))?
    } else {
        PathBuf::from(source_uri)
    };
    if !source_path.is_absolute()
        || source_path
            .components()
            .any(|component| component == Component::ParentDir)
    {
        return Err(AppError::Config("知识来源路径无效".to_string()));
    }
    Ok(source_path)
}

fn resolve_governed_source(
    source_uri: &str,
    require_existing_file: bool,
) -> AppResult<(PathBuf, PathBuf)> {
    resolve_governed_source_with_roots(
        source_uri,
        &detect_knowledge_retrieval_dirs(),
        require_existing_file,
    )
}

fn resolve_governed_source_with_roots(
    source_uri: &str,
    roots: &[PathBuf],
    require_existing_file: bool,
) -> AppResult<(PathBuf, PathBuf)> {
    let source_path = source_path_from_uri(source_uri)?;
    let comparable_path = if source_path.exists() {
        fs::canonicalize(&source_path)?
    } else {
        source_path.clone()
    };
    let root = roots
        .iter()
        .find(|root| comparable_path.starts_with(root))
        .cloned()
        .ok_or_else(|| {
            AppError::Config("仅允许操作受治理 knowledge-retrieval 目录内的知识".to_string())
        })?;
    if require_existing_file && !comparable_path.is_file() {
        return Err(AppError::Config("知识源文件不存在，无法定位".to_string()));
    }
    if comparable_path.exists() && !comparable_path.is_file() {
        return Err(AppError::Config("知识来源不是文件".to_string()));
    }
    Ok((root, comparable_path))
}

fn reveal_path(path: &Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()?;
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-R").arg(path).spawn()?;
        Ok(())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = path
            .parent()
            .ok_or_else(|| AppError::Config("无法定位知识源文件夹".to_string()))?;
        Command::new("xdg-open").arg(parent).spawn()?;
        Ok(())
    }
}

fn knowledge_trash_root() -> AppResult<PathBuf> {
    let data_dir = dirs::data_local_dir()
        .ok_or_else(|| AppError::Config("无法定位本机应用数据目录".to_string()))?;
    Ok(data_dir
        .join("PAISHU")
        .join("knowledge-service")
        .join(KNOWLEDGE_TRASH_DIR))
}

fn archive_knowledge_source(
    document: &KnowledgeDocumentSummary,
    source_roots: &[PathBuf],
) -> AppResult<ArchivedKnowledgeSource> {
    archive_knowledge_source_with_roots(document, source_roots, &knowledge_trash_root()?)
}

fn archive_knowledge_source_with_roots(
    document: &KnowledgeDocumentSummary,
    source_roots: &[PathBuf],
    trash_root: &Path,
) -> AppResult<ArchivedKnowledgeSource> {
    let (source_root, source_path) =
        resolve_governed_source_with_roots(&document.source_uri, source_roots, false)?;
    let relative_path = source_path
        .strip_prefix(&source_root)
        .map_err(|_| AppError::Config("知识来源不在受治理目录内".to_string()))?;
    let timestamp = Local::now().format("%Y%m%d-%H%M%S-%3f").to_string();
    let archive_dir = trash_root.join(format!("{timestamp}-{}", &document.id[..8]));
    let archived_path = source_path
        .is_file()
        .then(|| archive_dir.join("files").join(relative_path));
    let tombstone_path = archive_dir.join(TOMBSTONE_FILE);
    let tombstone = KnowledgeTombstone {
        document_id: document.id.clone(),
        title: document.title.clone(),
        source_uri: document.source_uri.clone(),
        archived_path: archived_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        deleted_at: Local::now().to_rfc3339(),
    };
    let tombstone_json = serde_json::to_vec_pretty(&tombstone)?;

    fs::create_dir_all(&archive_dir)?;
    if let Some(destination) = &archived_path {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&source_path, destination)?;
    }
    if let Err(error) = fs::write(&tombstone_path, tombstone_json) {
        if let Some(destination) = &archived_path {
            let _ = fs::rename(destination, &source_path);
        }
        return Err(error.into());
    }

    Ok(ArchivedKnowledgeSource {
        original_path: source_path,
        archived_path,
        tombstone_path,
    })
}

fn rollback_archived_source(archived: &ArchivedKnowledgeSource) {
    let _ = fs::remove_file(&archived.tombstone_path);
    if let Some(archived_path) = &archived.archived_path {
        if archived_path.is_file() && !archived.original_path.exists() {
            let _ = fs::rename(archived_path, &archived.original_path);
        }
    }
}

fn deleted_document_ids() -> HashSet<String> {
    let Ok(trash_root) = knowledge_trash_root() else {
        return HashSet::new();
    };
    deleted_document_ids_from(&trash_root)
}

fn deleted_document_ids_from(trash_root: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();
    let Ok(entries) = fs::read_dir(trash_root) else {
        return ids;
    };
    for entry in entries.flatten() {
        let tombstone_path = entry.path().join(TOMBSTONE_FILE);
        let Ok(contents) = fs::read_to_string(tombstone_path) else {
            continue;
        };
        if let Ok(tombstone) = serde_json::from_str::<KnowledgeTombstone>(&contents) {
            ids.insert(tombstone.document_id);
        }
    }
    ids
}

fn hide_deleted_knowledge(board: &mut KnowledgeBoard) {
    hide_deleted_knowledge_by_ids(board, &deleted_document_ids());
}

fn hide_deleted_knowledge_by_ids(board: &mut KnowledgeBoard, ids: &HashSet<String>) {
    let mut removed_total = 0_u64;
    let mut removed_enabled = 0_u64;
    let mut removed_disabled = 0_u64;
    let mut removed_chunks = 0_u64;
    board.documents.retain(|document| {
        if !ids.contains(&document.id) {
            return true;
        }
        removed_total += 1;
        removed_chunks += document.chunk_count;
        if document.enabled {
            removed_enabled += 1;
        } else {
            removed_disabled += 1;
        }
        false
    });
    board.total_documents = board.total_documents.saturating_sub(removed_total);
    board.enabled_documents = board.enabled_documents.saturating_sub(removed_enabled);
    board.disabled_documents = board.disabled_documents.saturating_sub(removed_disabled);
    board.chunk_count = board.chunk_count.saturating_sub(removed_chunks);
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
    request_local_service_with_timeout(config, method, path, body, REQUEST_TIMEOUT_SECS)
}

fn request_local_service_with_timeout(
    config: &KnowledgeServiceConfig,
    method: &str,
    path: &str,
    body: Option<&str>,
    timeout_secs: u64,
) -> AppResult<String> {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), config.port);
    let timeout = Duration::from_secs(timeout_secs);
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

    fn document_summary(source_uri: String) -> KnowledgeDocumentSummary {
        KnowledgeDocumentSummary {
            id: "d50f8262-c19d-46cd-a001-5d634b692807".to_string(),
            title: "客户素材需求清单".to_string(),
            source_uri,
            owner: "PAISHU".to_string(),
            status: "active".to_string(),
            access_tier: "internal".to_string(),
            enabled: true,
            chunk_count: 2,
            approximate_tokens: 128,
            updated_at: "2026-07-15T00:00:00Z".to_string(),
            package_name: "客户资产知识库".to_string(),
        }
    }

    fn board_with_document(document: KnowledgeDocumentSummary) -> KnowledgeBoard {
        KnowledgeBoard {
            refreshed_at: "2026-07-15T00:00:00Z".to_string(),
            service_status: "ok".to_string(),
            collection_count: 1,
            total_documents: 1,
            enabled_documents: 1,
            disabled_documents: 0,
            chunk_count: document.chunk_count,
            database_bytes: 1024,
            average_read_ms: 5,
            read_success_count: 1,
            read_failure_count: 0,
            documents: vec![document],
            messages: vec![],
        }
    }

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
        assert_eq!(packages[0].namespace, "example");
        assert_eq!(packages[0].title, "example");
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
    fn namespaces_external_ids_by_knowledge_package() {
        let temp = tempfile::tempdir().unwrap();
        let first_root = temp.path().join("first/kb");
        let second_root = temp.path().join("second/kb");
        let relative = Path::new("cards/glossary.md");
        let first = KnowledgePackage {
            namespace: "first".to_string(),
            title: "First Knowledge".to_string(),
            root: temp.path().join("first"),
            ingest_root: first_root.clone(),
            status: "active".to_string(),
            access_tier: "internal".to_string(),
            owner: "PAISHU".to_string(),
        };
        let second = KnowledgePackage {
            namespace: "second".to_string(),
            title: "Second Knowledge".to_string(),
            root: temp.path().join("second"),
            ingest_root: second_root.clone(),
            status: "active".to_string(),
            access_tier: "internal".to_string(),
            owner: "PAISHU".to_string(),
        };

        assert_eq!(
            knowledge_external_id(&first, &first_root.join(relative)).unwrap(),
            "first/cards/glossary.md"
        );
        assert_eq!(
            knowledge_external_id(&second, &second_root.join(relative)).unwrap(),
            "second/cards/glossary.md"
        );
    }

    #[test]
    fn resolves_only_existing_files_inside_governed_knowledge_roots() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("knowledge-retrieval");
        let source = root.join("09_RAG_MATERIALS/example/kb/source.md");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "# Knowledge").unwrap();
        let canonical_root = fs::canonicalize(&root).unwrap();
        let canonical_source = fs::canonicalize(&source).unwrap();

        let (_, resolved) = resolve_governed_source_with_roots(
            canonical_source.to_str().unwrap(),
            &[canonical_root],
            true,
        )
        .unwrap();

        assert_eq!(resolved, canonical_source);
        let outside = temp.path().join("outside.md");
        fs::write(&outside, "outside").unwrap();
        assert!(resolve_governed_source_with_roots(
            outside.to_str().unwrap(),
            &[fs::canonicalize(&root).unwrap()],
            true,
        )
        .is_err());
    }

    #[test]
    fn archives_source_and_persists_a_tombstone() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("knowledge-retrieval");
        let source = root.join("09_RAG_MATERIALS/example/kb/source.md");
        let trash = temp.path().join("trash");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "# Knowledge").unwrap();
        let document = document_summary(source.to_string_lossy().to_string());

        let archived = archive_knowledge_source_with_roots(
            &document,
            &[fs::canonicalize(&root).unwrap()],
            &trash,
        )
        .unwrap();

        assert!(!source.exists());
        assert!(archived.archived_path.as_ref().unwrap().is_file());
        assert!(archived.tombstone_path.is_file());
        assert!(deleted_document_ids_from(&trash).contains(&document.id));

        rollback_archived_source(&archived);
        assert!(source.is_file());
        assert!(!archived.tombstone_path.exists());
    }

    #[test]
    fn hides_tombstoned_documents_from_dashboard_totals() {
        let document = document_summary("/knowledge/source.md".to_string());
        let mut board = board_with_document(document.clone());
        let ids = HashSet::from([document.id]);

        hide_deleted_knowledge_by_ids(&mut board, &ids);

        assert!(board.documents.is_empty());
        assert_eq!(board.total_documents, 0);
        assert_eq!(board.enabled_documents, 0);
        assert_eq!(board.chunk_count, 0);
    }

    #[test]
    #[ignore = "requires the deployed localhost PAISHU knowledge service"]
    fn syncs_and_reads_the_live_local_dashboard_contract() {
        let board = sync_knowledge_sources().unwrap();
        assert_eq!(board.service_status, "ok");
        assert!(board.total_documents > 0);
        assert_eq!(board.documents.len() as u64, board.total_documents);
        let overview = get_knowledge_overview(&board.documents[0].id).unwrap();
        assert_eq!(overview.document_id, board.documents[0].id);
        assert!(!overview.overview.trim().is_empty());
    }
}
