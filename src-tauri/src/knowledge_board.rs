use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    time::Duration,
};

const DEFAULT_PORT: u16 = 8765;
const REQUEST_TIMEOUT_SECS: u64 = 8;

#[derive(Debug, Clone)]
struct KnowledgeServiceConfig {
    host: String,
    port: u16,
    api_token: String,
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
