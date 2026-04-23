use crate::services::env_manager;
use crate::settings::{get_settings, mutate_settings, CliProxyConfig, CliProxyMode};
use tauri::State;

#[tauri::command]
pub fn get_cli_proxy_config() -> CliProxyConfig {
    get_settings().cli_proxy.clone()
}

#[tauri::command]
pub fn set_cli_proxy_config(config: CliProxyConfig) -> Result<(), String> {
    mutate_settings(|settings| {
        settings.cli_proxy = config;
    })?;
    Ok(())
}

#[tauri::command]
pub fn apply_cli_proxy(state: State<crate::AppState>) -> Result<(), String> {
    let settings = get_settings();
    let config = &settings.cli_proxy;

    match config.mode {
        CliProxyMode::Disabled => {
            env_manager::clear_cli_proxy_env()?;
        }
        CliProxyMode::SyncOutbound => {
            let proxy_url = state.db.get_global_proxy_url().map_err(|e| e.to_string())?;
            if let Some(url) = proxy_url {
                if !url.is_empty() {
                    env_manager::set_cli_proxy_env(&url)?;
                } else {
                    env_manager::clear_cli_proxy_env()?;
                }
            } else {
                env_manager::clear_cli_proxy_env()?;
            }
        }
        CliProxyMode::Independent => {
            if let Some(url) = &config.independent_url {
                if !url.is_empty() {
                    env_manager::set_cli_proxy_env(url)?;
                } else {
                    env_manager::clear_cli_proxy_env()?;
                }
            } else {
                env_manager::clear_cli_proxy_env()?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn get_current_cli_proxy_env() -> Option<String> {
    env_manager::get_cli_proxy_env()
}

#[tauri::command]
pub async fn scan_and_clear_local_git_proxy(
    folder: String,
) -> Result<env_manager::LocalGitProxyResult, String> {
    let root = std::path::PathBuf::from(&folder);
    tokio::task::spawn_blocking(move || {
        env_manager::scan_and_clear_local_git_proxy(&root, 5)
    })
    .await
    .map_err(|e| format!("扫描任务失败: {e}"))?
}
