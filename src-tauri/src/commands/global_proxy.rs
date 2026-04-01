//! 全局出站代理相关命令
//!
//! 提供获取、设置和测试全局代理的 Tauri 命令。

use crate::proxy::http_client;
use crate::store::AppState;
use serde::Serialize;
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::time::{Duration, Instant};

/// 获取全局代理 URL
///
/// 返回当前配置的代理 URL，null 表示直连。
#[tauri::command]
pub fn get_global_proxy_url(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    let result = state.db.get_global_proxy_url().map_err(|e| e.to_string())?;
    log::debug!(
        "[GlobalProxy] [GP-010] Read from database: {}",
        result
            .as_ref()
            .map(|u| http_client::mask_url(u))
            .unwrap_or_else(|| "None".to_string())
    );
    Ok(result)
}

/// 设置全局代理 URL
///
/// - 传入非空字符串：启用代理
/// - 传入空字符串：清除代理（直连）
///
/// 执行顺序：先验证 → 写 DB → 再应用
/// 这样确保 DB 写失败时不会出现运行态与持久化不一致的问题
#[tauri::command]
pub fn set_global_proxy_url(state: tauri::State<'_, AppState>, url: String) -> Result<(), String> {
    // 调试：显示接收到的 URL 信息（不包含敏感内容）
    let has_auth = url.contains('@') && (url.starts_with("http://") || url.starts_with("socks"));
    log::debug!(
        "[GlobalProxy] [GP-011] Received URL: length={}, has_auth={}",
        url.len(),
        has_auth
    );

    let url_opt = if url.trim().is_empty() {
        None
    } else {
        Some(url.as_str())
    };

    // 1. 先验证代理配置是否有效（不应用）
    http_client::validate_proxy(url_opt)?;

    // 2. 验证成功后保存到数据库
    state
        .db
        .set_global_proxy_url(url_opt)
        .map_err(|e| e.to_string())?;

    // 3. DB 写入成功后再应用到运行态
    http_client::apply_proxy(url_opt)?;

    log::info!(
        "[GlobalProxy] [GP-009] Configuration updated: {}",
        url_opt
            .map(http_client::mask_url)
            .unwrap_or_else(|| "direct connection".to_string())
    );

    // 如果 CLI 代理模式为 SyncOutbound，同步更新
    let settings = crate::settings::get_settings();
    if matches!(settings.cli_proxy.mode, crate::settings::CliProxyMode::SyncOutbound) {
        let _ = super::cli_proxy::apply_cli_proxy(state);
    }

    Ok(())
}

/// 代理测试结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyTestResult {
    /// 是否连接成功
    pub success: bool,
    /// 延迟（毫秒）
    pub latency_ms: u64,
    /// 错误信息
    pub error: Option<String>,
}

/// 测试代理连接
///
/// 通过指定的代理 URL 发送测试请求，返回连接结果和延迟。
/// 使用多个测试目标，任一成功即认为代理可用。
#[tauri::command]
pub async fn test_proxy_url(url: String) -> Result<ProxyTestResult, String> {
    if url.trim().is_empty() {
        return Err("Proxy URL is empty".to_string());
    }

    let start = Instant::now();

    // 构建带代理的临时客户端
    let proxy = reqwest::Proxy::all(&url).map_err(|e| format!("Invalid proxy URL: {e}"))?;

    let client = reqwest::Client::builder()
        .proxy(proxy)
        .timeout(std::time::Duration::from_secs(10))
        .connect_timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to build client: {e}"))?;

    // 使用多个测试目标，提高兼容性
    // 优先使用 httpbin（专门用于 HTTP 测试），回退到其他公共端点
    let test_urls = [
        "https://httpbin.org/get",
        "https://www.google.com",
        "https://api.anthropic.com",
    ];

    let mut last_error = None;

    for test_url in test_urls {
        match client.head(test_url).send().await {
            Ok(resp) => {
                let latency = start.elapsed().as_millis() as u64;
                log::debug!(
                    "[GlobalProxy] Test successful: {} -> {} via {} ({}ms)",
                    http_client::mask_url(&url),
                    test_url,
                    resp.status(),
                    latency
                );
                return Ok(ProxyTestResult {
                    success: true,
                    latency_ms: latency,
                    error: None,
                });
            }
            Err(e) => {
                log::debug!("[GlobalProxy] Test to {test_url} failed: {e}");
                last_error = Some(e);
            }
        }
    }

    // 所有测试目标都失败
    let latency = start.elapsed().as_millis() as u64;
    let error_msg = last_error
        .map(|e| e.to_string())
        .unwrap_or_else(|| "All test targets failed".to_string());

    log::debug!(
        "[GlobalProxy] Test failed: {} -> {} ({}ms)",
        http_client::mask_url(&url),
        error_msg,
        latency
    );

    Ok(ProxyTestResult {
        success: false,
        latency_ms: latency,
        error: Some(error_msg),
    })
}

/// 获取当前出站代理状态
///
/// 返回当前是否启用了出站代理以及代理 URL。
#[tauri::command]
pub fn get_upstream_proxy_status() -> UpstreamProxyStatus {
    let url = http_client::get_current_proxy_url();
    UpstreamProxyStatus {
        enabled: url.is_some(),
        proxy_url: url,
    }
}

/// 出站代理状态信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpstreamProxyStatus {
    /// 是否启用代理
    pub enabled: bool,
    /// 代理 URL
    pub proxy_url: Option<String>,
}

/// 检测到的代理信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProxy {
    /// 代理 URL
    pub url: String,
    /// 代理类型 (http/socks5)
    pub proxy_type: String,
    /// 端口
    pub port: u16,
}

/// 常见代理端口配置
/// 格式：(端口, 主要类型, 是否同时支持 http 和 socks5)
/// 对于 mixed 端口，会同时返回两种协议供用户选择
const PROXY_PORTS: &[(u16, &str, bool)] = &[
    (7890, "http", true),     // Clash (mixed mode)
    (7891, "socks5", false),  // Clash SOCKS only
    (1080, "socks5", false),  // 通用 SOCKS5
    (8080, "http", false),    // 通用 HTTP
    (8888, "http", false),    // Charles/Fiddler
    (3128, "http", false),    // Squid
    (10808, "socks5", false), // V2Ray SOCKS
    (10809, "http", false),   // V2Ray HTTP
];

/// 扫描本地代理
///
/// 检测常见端口是否有代理服务在运行。
/// 使用异步任务避免阻塞 UI 线程。
#[tauri::command]
pub async fn scan_local_proxies() -> Vec<DetectedProxy> {
    // 使用 spawn_blocking 避免阻塞主线程
    tokio::task::spawn_blocking(|| {
        let mut found = Vec::new();

        for &(port, primary_type, is_mixed) in PROXY_PORTS {
            let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
            if TcpStream::connect_timeout(&addr.into(), Duration::from_millis(100)).is_ok() {
                // 添加主要类型
                found.push(DetectedProxy {
                    url: format!("{primary_type}://127.0.0.1:{port}"),
                    proxy_type: primary_type.to_string(),
                    port,
                });
                // 对于 mixed 端口，同时添加另一种协议
                if is_mixed {
                    let alt_type = if primary_type == "http" {
                        "socks5"
                    } else {
                        "http"
                    };
                    found.push(DetectedProxy {
                        url: format!("{alt_type}://127.0.0.1:{port}"),
                        proxy_type: alt_type.to_string(),
                        port,
                    });
                }
            }
        }

        found
    })
    .await
    .unwrap_or_default()
}

/// 实际生效的代理诊断信息
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveProxyStatus {
    /// 代理来源："explicit"（用户配置）/ "system"（系统自动检测）/ "direct"（直连）
    pub source: String,
    /// 显式配置的代理 URL（脱敏）
    pub explicit_proxy: Option<String>,
    /// 连通性测试结果
    pub reachable: bool,
    /// 延迟（毫秒）
    pub latency_ms: u64,
    /// 错误信息
    pub error: Option<String>,
}

/// 获取当前实际生效的代理状态（含连通性测试）
///
/// 用于诊断代理问题：告诉用户当前出站请求走的是什么代理，能不能通。
#[tauri::command]
pub async fn get_effective_proxy_status() -> EffectiveProxyStatus {
    let explicit_url = http_client::get_current_proxy_url();
    let source = if explicit_url.is_some() {
        "explicit"
    } else {
        "system"
    };

    // 用当前全局客户端测试连通性
    let client = http_client::get();
    let start = Instant::now();

    let test_urls = [
        "https://api.anthropic.com",
        "https://httpbin.org/get",
    ];

    let mut last_error = None;
    for test_url in test_urls {
        match client
            .head(test_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(_) => {
                let latency = start.elapsed().as_millis() as u64;
                return EffectiveProxyStatus {
                    source: source.to_string(),
                    explicit_proxy: explicit_url.map(|u| http_client::mask_url(&u)),
                    reachable: true,
                    latency_ms: latency,
                    error: None,
                };
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    let latency = start.elapsed().as_millis() as u64;
    EffectiveProxyStatus {
        source: if explicit_url.is_some() {
            "explicit".to_string()
        } else {
            "system".to_string()
        },
        explicit_proxy: explicit_url.map(|u| http_client::mask_url(&u)),
        reachable: false,
        latency_ms: latency,
        error: last_error,
    }
}

/// 同步系统代理（智能检测：有代理就用，没有就直连）
///
/// 1. 扫描本地常见代理端口
/// 2. 如果找到可用代理，测试连通性后应用
/// 3. 如果没找到，切换为直连模式
#[tauri::command]
pub async fn sync_system_proxy(
    state: tauri::State<'_, AppState>,
) -> Result<SyncProxyResult, String> {
    // 1. 扫描本地代理
    let proxies = scan_local_proxies().await;

    if proxies.is_empty() {
        // 没找到代理，切换为直连
        http_client::apply_no_proxy().map_err(|e| e.to_string())?;
        state
            .db
            .set_global_proxy_url(None)
            .map_err(|e| e.to_string())?;

        log::info!("[GlobalProxy] Sync: no local proxy found, switched to direct");

        // 如果 CLI 代理模式为 SyncOutbound，同步更新
        let settings = crate::settings::get_settings();
        if matches!(settings.cli_proxy.mode, crate::settings::CliProxyMode::SyncOutbound) {
            let _ = super::cli_proxy::apply_cli_proxy(state.clone());
        }

        return Ok(SyncProxyResult {
            action: "direct".to_string(),
            proxy_url: None,
            message: None,
        });
    }

    // 2. 逐个测试找到的代理
    for proxy in &proxies {
        let result = test_proxy_url(proxy.url.clone()).await;
        if let Ok(ref r) = result {
            if r.success {
                // 找到可用代理，应用它
                http_client::validate_proxy(Some(&proxy.url))?;
                state
                    .db
                    .set_global_proxy_url(Some(&proxy.url))
                    .map_err(|e| e.to_string())?;
                http_client::apply_proxy(Some(&proxy.url))?;

                log::info!(
                    "[GlobalProxy] Sync: applied proxy {}",
                    http_client::mask_url(&proxy.url)
                );

                // 如果 CLI 代理模式为 SyncOutbound，同步更新
                let settings = crate::settings::get_settings();
                if matches!(settings.cli_proxy.mode, crate::settings::CliProxyMode::SyncOutbound) {
                    let _ = super::cli_proxy::apply_cli_proxy(state.clone());
                }

                return Ok(SyncProxyResult {
                    action: "proxy".to_string(),
                    proxy_url: Some(http_client::mask_url(&proxy.url)),
                    message: None,
                });
            }
        }
    }

    // 3. 找到代理但都不通，切换为直连
    http_client::apply_no_proxy().map_err(|e| e.to_string())?;
    state
        .db
        .set_global_proxy_url(None)
        .map_err(|e| e.to_string())?;

    log::info!("[GlobalProxy] Sync: found proxies but none reachable, switched to direct");

    // 如果 CLI 代理模式为 SyncOutbound，同步更新
    let settings = crate::settings::get_settings();
    if matches!(settings.cli_proxy.mode, crate::settings::CliProxyMode::SyncOutbound) {
        let _ = super::cli_proxy::apply_cli_proxy(state);
    }

    Ok(SyncProxyResult {
        action: "direct".to_string(),
        proxy_url: None,
        message: Some("Found local proxies but none reachable".to_string()),
    })
}

/// 同步代理结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProxyResult {
    /// 执行的动作："proxy"（应用了代理）/ "direct"（切换为直连）
    pub action: String,
    /// 应用的代理 URL（脱敏）
    pub proxy_url: Option<String>,
    /// 附加信息
    pub message: Option<String>,
}
