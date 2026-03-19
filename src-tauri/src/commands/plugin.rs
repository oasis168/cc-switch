#![allow(non_snake_case)]

use crate::config::ConfigStatus;

/// Claude 插件：获取 ~/.claude/config.json 状态
#[tauri::command]
pub async fn get_claude_plugin_status() -> Result<ConfigStatus, String> {
    crate::claude_plugin::claude_config_status()
        .map(|(exists, path)| ConfigStatus {
            exists,
            path: path.to_string_lossy().to_string(),
        })
        .map_err(|e| e.to_string())
}

/// Claude 插件：读取配置内容（若不存在返回 Ok(None)）
#[tauri::command]
pub async fn read_claude_plugin_config() -> Result<Option<String>, String> {
    crate::claude_plugin::read_claude_config().map_err(|e| e.to_string())
}

/// Claude 插件：写入/清除固定配置
#[tauri::command]
pub async fn apply_claude_plugin_config(official: bool) -> Result<bool, String> {
    if official {
        crate::claude_plugin::clear_claude_config().map_err(|e| e.to_string())
    } else {
        crate::claude_plugin::write_claude_config().map_err(|e| e.to_string())
    }
}

/// Claude 插件：检测是否已写入目标配置
#[tauri::command]
pub async fn is_claude_plugin_applied() -> Result<bool, String> {
    crate::claude_plugin::is_claude_config_applied().map_err(|e| e.to_string())
}

/// Claude Code：跳过初次安装确认（写入 ~/.claude.json 的 hasCompletedOnboarding=true）
#[tauri::command]
pub async fn apply_claude_onboarding_skip() -> Result<bool, String> {
    crate::claude_mcp::set_has_completed_onboarding().map_err(|e| e.to_string())
}

/// Claude Code：恢复初次安装确认（删除 ~/.claude.json 的 hasCompletedOnboarding 字段）
#[tauri::command]
pub async fn clear_claude_onboarding_skip() -> Result<bool, String> {
    crate::claude_mcp::clear_has_completed_onboarding().map_err(|e| e.to_string())
}

/// VSCode 插件：获取探测到的 VSCode settings.json 路径
#[tauri::command]
pub async fn get_vscode_settings_path() -> Result<Option<String>, String> {
    Ok(crate::vscode_settings::get_vscode_settings_path()
        .map(|p| p.to_string_lossy().to_string()))
}

/// VSCode 插件：检测 settings.json 与当前供应商是否匹配
#[tauri::command]
pub async fn check_vscode_config_status(
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<crate::vscode_settings::VscodeConfigStatus, String> {
    use crate::app_config::AppType;

    let app_settings = crate::settings::get_settings();
    if !app_settings.enable_claude_plugin_integration {
        return Ok(crate::vscode_settings::VscodeConfigStatus::IntegrationDisabled);
    }

    // 获取代理运行状态和端口
    let proxy_running = state.proxy_service.is_running().await;
    let proxy_port = crate::proxy::http_client::get_cc_switch_proxy_port();

    // 获取当前供应商 token/base_url
    let provider_id = crate::settings::get_effective_current_provider(state.db.as_ref(), &AppType::Claude)
        .map_err(|e| e.to_string())?;

    let (current_token, current_base_url) = if let Some(id) = provider_id {
        let all = state.db.get_all_providers(AppType::Claude.as_str()).map_err(|e| e.to_string())?;
        if let Some(provider) = all.get(&id) {
            let env = provider.settings_config.get("env");
            let token = env
                .and_then(|e: &serde_json::Value| e.get("ANTHROPIC_AUTH_TOKEN"))
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or("")
                .to_string();
            let base_url = env
                .and_then(|e: &serde_json::Value| e.get("ANTHROPIC_BASE_URL"))
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or("")
                .to_string();
            (token, base_url)
        } else {
            (String::new(), String::new())
        }
    } else {
        (String::new(), String::new())
    };

    Ok(crate::vscode_settings::check_vscode_config_status(
        proxy_running,
        proxy_port,
        &current_token,
        &current_base_url,
    ))
}

/// VSCode 插件：手动将当前激活供应商同步到 VSCode settings.json
#[tauri::command]
pub async fn sync_vscode_settings(
    state: tauri::State<'_, crate::store::AppState>,
) -> Result<(), String> {
    use crate::app_config::AppType;

    let app_settings = crate::settings::get_settings();
    if !app_settings.enable_claude_plugin_integration {
        return Ok(());
    }

    let provider_id = crate::settings::get_effective_current_provider(state.db.as_ref(), &AppType::Claude)
        .map_err(|e| e.to_string())?;

    if let Some(id) = provider_id {
        let all = state.db.get_all_providers(AppType::Claude.as_str()).map_err(|e| e.to_string())?;
        if let Some(provider) = all.get(&id) {
            // 判断 claude 代理接管是否激活（proxy_config.enabled）
            // 而非判断"在主页面显示本地代理开关"（enable_local_proxy）
            let proxy_takeover_active = state.db
                .get_proxy_config_for_app(AppType::Claude.as_str())
                .await
                .map(|c| c.enabled)
                .unwrap_or(false);
            let (token, base_url) = if proxy_takeover_active {
                let port = crate::proxy::http_client::get_cc_switch_proxy_port();
                ("proxy-placeholder".to_string(), format!("http://127.0.0.1:{port}"))
            } else {
                let env = provider.settings_config.get("env");
                let token = env
                    .and_then(|e: &serde_json::Value| e.get("ANTHROPIC_AUTH_TOKEN"))
                    .and_then(|v: &serde_json::Value| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let base_url = env
                    .and_then(|e: &serde_json::Value| e.get("ANTHROPIC_BASE_URL"))
                    .and_then(|v: &serde_json::Value| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (token, base_url)
            };
            crate::vscode_settings::update_claude_env_vars(&token, &base_url)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
