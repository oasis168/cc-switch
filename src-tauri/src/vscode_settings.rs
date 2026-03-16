//! VSCode/Cursor/Windsurf settings.json 管理模块
//!
//! 在切换供应商时，同步更新编辑器的 claudeCode.environmentVariables 配置。
//! 支持直连模式（真实 key）和代理模式（proxy-placeholder）。
//! 支持 VSCode、Cursor、Windsurf 三种 IDE。

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::config::write_json_file;

/// 获取 VSCode settings.json 的默认路径（跨平台）
pub fn get_default_vscode_settings_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(appdata).join("Code").join("User").join("settings.json"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        Some(home.join("Library").join("Application Support").join("Code").join("User").join("settings.json"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let home = dirs::home_dir()?;
        Some(home.join(".config").join("Code").join("User").join("settings.json"))
    }
}

/// 获取 Cursor settings.json 的默认路径（跨平台）
pub fn get_default_cursor_settings_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(appdata).join("Cursor").join("User").join("settings.json"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        Some(home.join("Library").join("Application Support").join("Cursor").join("User").join("settings.json"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let home = dirs::home_dir()?;
        Some(home.join(".config").join("Cursor").join("User").join("settings.json"))
    }
}

/// 获取 Windsurf settings.json 的默认路径（跨平台）
pub fn get_default_windsurf_settings_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA").ok()?;
        Some(PathBuf::from(appdata).join("Windsurf").join("User").join("settings.json"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = dirs::home_dir()?;
        Some(home.join("Library").join("Application Support").join("Windsurf").join("User").join("settings.json"))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let home = dirs::home_dir()?;
        Some(home.join(".config").join("Windsurf").join("User").join("settings.json"))
    }
}

/// 根据 ide_type 获取对应 IDE 的默认路径
pub fn get_default_path_for_ide(ide_type: &str) -> Option<PathBuf> {
    match ide_type {
        "cursor" => get_default_cursor_settings_path(),
        "windsurf" => get_default_windsurf_settings_path(),
        _ => get_default_vscode_settings_path(),
    }
}

/// 获取有效的 IDE settings.json 路径
/// 优先使用用户自定义路径（settings 中的 vscode_settings_path），
/// 否则根据 ide_type 自动探测对应 IDE 的默认路径
pub fn get_vscode_settings_path() -> Option<PathBuf> {
    let settings = crate::settings::get_settings();
    // 用户手动指定路径优先
    if let Some(custom) = &settings.vscode_settings_path {
        if !custom.trim().is_empty() {
            return Some(PathBuf::from(custom));
        }
    }
    // 根据 ide_type 自动探测
    let ide_type = settings.ide_type.as_deref().unwrap_or("vscode");
    get_default_path_for_ide(ide_type)
}

/// 更新 VSCode settings.json 中的 claudeCode.environmentVariables
/// 增量修改：只改目标字段，保留其他所有字段
pub fn update_claude_env_vars(token: &str, base_url: &str) -> Result<(), crate::error::AppError> {
    let path = match get_vscode_settings_path() {
        Some(p) => p,
        None => {
            log::warn!("[VSCodeSettings] 无法确定 VSCode settings.json 路径，跳过同步");
            return Ok(());
        }
    };

    // 文件不存在时静默跳过
    if !path.exists() {
        log::debug!("[VSCodeSettings] settings.json 不存在，跳过: {}", path.display());
        return Ok(());
    }

    // 读取现有配置
    let mut settings: Value = if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                log::warn!("[VSCodeSettings] 解析 settings.json 失败: {e}，将使用空对象");
                serde_json::json!({})
            }),
            Err(e) => {
                log::warn!("[VSCodeSettings] 读取 settings.json 失败: {e}，跳过同步");
                return Ok(());
            }
        }
    } else {
        serde_json::json!({})
    };

    let obj = match settings.as_object_mut() {
        Some(o) => o,
        None => {
            log::warn!("[VSCodeSettings] settings.json 不是 JSON 对象，跳过同步");
            return Ok(());
        }
    };

    // 构建新的 claudeCode.environmentVariables
    // 保留非 Claude 相关的环境变量条目
    let existing_vars = obj
        .get("claudeCode.environmentVariables")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut new_vars: Vec<Value> = existing_vars
        .into_iter()
        .filter(|var| {
            let name = var.get("name").and_then(|n| n.as_str()).unwrap_or("");
            name != "ANTHROPIC_AUTH_TOKEN" && name != "ANTHROPIC_BASE_URL"
        })
        .collect();

    new_vars.push(serde_json::json!({ "name": "ANTHROPIC_AUTH_TOKEN", "value": token }));
    new_vars.push(serde_json::json!({ "name": "ANTHROPIC_BASE_URL", "value": base_url }));

    obj.insert(
        "claudeCode.environmentVariables".to_string(),
        Value::Array(new_vars),
    );

    write_json_file(&path, &settings)?;
    log::info!("[VSCodeSettings] 已同步 claudeCode.environmentVariables -> {}", path.display());
    Ok(())
}

/// 清除 VSCode settings.json 中的 claudeCode.environmentVariables
/// 切换回 CLI 模式时调用
pub fn clear_claude_env_vars() -> Result<(), crate::error::AppError> {
    let path = match get_vscode_settings_path() {
        Some(p) => p,
        None => return Ok(()),
    };

    if !path.exists() {
        return Ok(());
    }

    let mut settings: Value = match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({})),
        Err(e) => {
            log::warn!("[VSCodeSettings] 读取失败: {e}");
            return Ok(());
        }
    };

    if let Some(obj) = settings.as_object_mut() {
        obj.remove("claudeCode.environmentVariables");
    }

    write_json_file(&path, &settings)?;
    log::info!("[VSCodeSettings] 已清除 claudeCode.environmentVariables");
    Ok(())
}

/// VSCode 配置匹配状态
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum VscodeConfigStatus {
    /// 匹配正常
    Ok,
    /// settings.json 不存在或未配置
    NotFound,
    /// 代理模式：settings.json 指向代理但代理未启动
    ProxyNotRunning {
        #[serde(rename = "baseUrl")]
        base_url: String
    },
    /// 直连模式：settings.json 中的 key/url 与当前供应商不匹配
    Mismatch {
        #[serde(rename = "currentBaseUrl")]
        current_base_url: String,
        #[serde(rename = "vscodeBaseUrl")]
        vscode_base_url: String
    },
    /// 插件联动未启用
    IntegrationDisabled,
}

/// 启动时检测 VSCode settings.json 与当前激活供应商是否匹配
/// 返回匹配状态，供前端决定是否显示提示
pub fn check_vscode_config_status(
    proxy_running: bool,
    proxy_port: u16,
    current_token: &str,
    current_base_url: &str,
) -> VscodeConfigStatus {
    let app_settings = crate::settings::get_settings();
    if !app_settings.enable_claude_plugin_integration {
        return VscodeConfigStatus::IntegrationDisabled;
    }

    let path = match get_vscode_settings_path() {
        Some(p) => p,
        None => return VscodeConfigStatus::NotFound,
    };

    if !path.exists() {
        return VscodeConfigStatus::NotFound;
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return VscodeConfigStatus::NotFound,
    };

    let json: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return VscodeConfigStatus::NotFound,
    };

    let env_vars = match json.get("claudeCode.environmentVariables").and_then(|v| v.as_array()) {
        Some(v) => v.clone(),
        None => return VscodeConfigStatus::NotFound,
    };

    let vscode_token = env_vars.iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("ANTHROPIC_AUTH_TOKEN"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let vscode_base_url = env_vars.iter()
        .find(|v| v.get("name").and_then(|n| n.as_str()) == Some("ANTHROPIC_BASE_URL"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let expected_proxy_url = format!("http://127.0.0.1:{proxy_port}");

    // 判断 VSCode 当前指向代理
    if vscode_token == "proxy-placeholder" || vscode_base_url == expected_proxy_url {
        if !proxy_running {
            return VscodeConfigStatus::ProxyNotRunning {
                base_url: vscode_base_url.to_string(),
            };
        }
        return VscodeConfigStatus::Ok;
    }

    // 直连模式：对比 base_url
    if !current_base_url.is_empty() && vscode_base_url != current_base_url {
        return VscodeConfigStatus::Mismatch {
            current_base_url: current_base_url.to_string(),
            vscode_base_url: vscode_base_url.to_string(),
        };
    }

    VscodeConfigStatus::Ok
}
