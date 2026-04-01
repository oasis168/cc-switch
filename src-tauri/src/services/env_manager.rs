use super::env_checker::EnvConflict;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

#[cfg(target_os = "windows")]
fn broadcast_env_change() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    #[link(name = "user32")]
    extern "system" {
        fn SendMessageTimeoutW(
            hwnd: isize,
            msg: u32,
            wparam: usize,
            lparam: isize,
            flags: u32,
            timeout: u32,
            result: *mut usize,
        ) -> isize;
    }

    const HWND_BROADCAST: isize = 0xFFFF;
    const WM_SETTINGCHANGE: u32 = 0x001A;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;

    let env: Vec<u16> = OsStr::new("Environment").encode_wide().chain(Some(0)).collect();
    unsafe {
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0,
            env.as_ptr() as isize,
            SMTO_ABORTIFHUNG,
            5000,
            ptr::null_mut(),
        );
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub backup_path: String,
    pub timestamp: String,
    pub conflicts: Vec<EnvConflict>,
}

/// Delete environment variables with automatic backup
pub fn delete_env_vars(conflicts: Vec<EnvConflict>) -> Result<BackupInfo, String> {
    // Step 1: Create backup
    let backup_info = create_backup(&conflicts)?;

    // Step 2: Delete variables
    for conflict in &conflicts {
        match delete_single_env(conflict) {
            Ok(_) => {}
            Err(e) => {
                // If deletion fails, we keep the backup but return error
                return Err(format!(
                    "删除环境变量失败: {}. 备份已保存到: {}",
                    e, backup_info.backup_path
                ));
            }
        }
    }

    Ok(backup_info)
}

/// Create backup file before deletion
fn create_backup(conflicts: &[EnvConflict]) -> Result<BackupInfo, String> {
    // Get backup directory
    let backup_dir = get_backup_dir()?;
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    // Generate backup file name with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_file = backup_dir.join(format!("env-backup-{timestamp}.json"));

    // Create backup data
    let backup_info = BackupInfo {
        backup_path: backup_file.to_string_lossy().to_string(),
        timestamp: timestamp.clone(),
        conflicts: conflicts.to_vec(),
    };

    // Write backup file
    let json = serde_json::to_string_pretty(&backup_info)
        .map_err(|e| format!("序列化备份数据失败: {e}"))?;

    fs::write(&backup_file, json).map_err(|e| format!("写入备份文件失败: {e}"))?;

    Ok(backup_info)
}

/// Get backup directory path
fn get_backup_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
    Ok(home.join(".cc-switch").join("backups"))
}

/// Delete a single environment variable
#[cfg(target_os = "windows")]
fn delete_single_env(conflict: &EnvConflict) -> Result<(), String> {
    match conflict.source_type.as_str() {
        "system" => {
            if conflict.source_path.contains("HKEY_CURRENT_USER") {
                let hkcu = RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey_with_flags("Environment", KEY_ALL_ACCESS)
                    .map_err(|e| format!("打开注册表失败: {}", e))?;

                hkcu.delete_value(&conflict.var_name)
                    .map_err(|e| format!("删除注册表项失败: {}", e))?;
            } else if conflict.source_path.contains("HKEY_LOCAL_MACHINE") {
                let hklm = RegKey::predef(HKEY_LOCAL_MACHINE)
                    .open_subkey_with_flags(
                        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
                        KEY_ALL_ACCESS,
                    )
                    .map_err(|e| format!("打开系统注册表失败 (需要管理员权限): {}", e))?;

                hklm.delete_value(&conflict.var_name)
                    .map_err(|e| format!("删除系统注册表项失败: {}", e))?;
            }
            Ok(())
        }
        "file" => Err("Windows 系统不应该有文件类型的环境变量".to_string()),
        _ => Err(format!("未知的环境变量来源类型: {}", conflict.source_type)),
    }
}

#[cfg(not(target_os = "windows"))]
fn delete_single_env(conflict: &EnvConflict) -> Result<(), String> {
    match conflict.source_type.as_str() {
        "file" => {
            // Parse file path and line number from source_path (format: "path:line")
            let parts: Vec<&str> = conflict.source_path.split(':').collect();
            if parts.len() < 2 {
                return Err("无效的文件路径格式".to_string());
            }

            let file_path = parts[0];

            // Read file content
            let content = fs::read_to_string(file_path)
                .map_err(|e| format!("读取文件失败 {file_path}: {e}"))?;

            // Filter out the line containing the environment variable
            let new_content: Vec<String> = content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    let export_line = trimmed.strip_prefix("export ").unwrap_or(trimmed);

                    // Check if this line sets the target variable
                    if let Some(eq_pos) = export_line.find('=') {
                        let var_name = export_line[..eq_pos].trim();
                        var_name != conflict.var_name
                    } else {
                        true
                    }
                })
                .map(|s| s.to_string())
                .collect();

            // Write back to file
            fs::write(file_path, new_content.join("\n"))
                .map_err(|e| format!("写入文件失败 {file_path}: {e}"))?;

            Ok(())
        }
        "system" => {
            // On Unix, we can't directly delete process environment variables
            Ok(())
        }
        _ => Err(format!("未知的环境变量来源类型: {}", conflict.source_type)),
    }
}

/// Restore environment variables from backup
pub fn restore_from_backup(backup_path: String) -> Result<(), String> {
    // Read backup file
    let content = fs::read_to_string(&backup_path).map_err(|e| format!("读取备份文件失败: {e}"))?;

    let backup_info: BackupInfo =
        serde_json::from_str(&content).map_err(|e| format!("解析备份文件失败: {e}"))?;

    // Restore each variable
    for conflict in &backup_info.conflicts {
        restore_single_env(conflict)?;
    }

    Ok(())
}

/// Restore a single environment variable
#[cfg(target_os = "windows")]
fn restore_single_env(conflict: &EnvConflict) -> Result<(), String> {
    match conflict.source_type.as_str() {
        "system" => {
            if conflict.source_path.contains("HKEY_CURRENT_USER") {
                let (hkcu, _) = RegKey::predef(HKEY_CURRENT_USER)
                    .create_subkey("Environment")
                    .map_err(|e| format!("打开注册表失败: {}", e))?;

                hkcu.set_value(&conflict.var_name, &conflict.var_value)
                    .map_err(|e| format!("恢复注册表项失败: {}", e))?;
            } else if conflict.source_path.contains("HKEY_LOCAL_MACHINE") {
                let (hklm, _) = RegKey::predef(HKEY_LOCAL_MACHINE)
                    .create_subkey(
                        "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment",
                    )
                    .map_err(|e| format!("打开系统注册表失败 (需要管理员权限): {}", e))?;

                hklm.set_value(&conflict.var_name, &conflict.var_value)
                    .map_err(|e| format!("恢复系统注册表项失败: {}", e))?;
            }
            Ok(())
        }
        _ => Err(format!(
            "无法恢复类型为 {} 的环境变量",
            conflict.source_type
        )),
    }
}

#[cfg(not(target_os = "windows"))]
fn restore_single_env(conflict: &EnvConflict) -> Result<(), String> {
    match conflict.source_type.as_str() {
        "file" => {
            // Parse file path from source_path
            let parts: Vec<&str> = conflict.source_path.split(':').collect();
            if parts.is_empty() {
                return Err("无效的文件路径格式".to_string());
            }

            let file_path = parts[0];

            // Read file content
            let mut content = fs::read_to_string(file_path)
                .map_err(|e| format!("读取文件失败 {file_path}: {e}"))?;

            // Append the environment variable line
            let export_line = format!("\nexport {}={}", conflict.var_name, conflict.var_value);
            content.push_str(&export_line);

            // Write back to file
            fs::write(file_path, content).map_err(|e| format!("写入文件失败 {file_path}: {e}"))?;

            Ok(())
        }
        _ => Err(format!(
            "无法恢复类型为 {} 的环境变量",
            conflict.source_type
        )),
    }
}

// ===== CLI 工具代理环境变量管理 =====

const CLI_PROXY_VARS: [&str; 6] = [
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// 设置 CLI 工具代理环境变量
pub fn set_cli_proxy_env(proxy_url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_ALL_ACCESS)
            .map_err(|e| format!("打开注册表失败: {e}"))?;

        for var in &CLI_PROXY_VARS[..4] {
            env.set_value(var, &proxy_url)
                .map_err(|e| format!("设置 {var} 失败: {e}"))?;
        }

        let no_proxy = "localhost,127.0.0.1";
        env.set_value("NO_PROXY", &no_proxy)
            .map_err(|e| format!("设置 NO_PROXY 失败: {e}"))?;
        env.set_value("no_proxy", &no_proxy)
            .map_err(|e| format!("设置 no_proxy 失败: {e}"))?;

        broadcast_env_change();
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::env;
        let home = env::var("HOME").map_err(|_| "无法获取 HOME 环境变量")?;
        let shell_files = [
            format!("{home}/.bashrc"),
            format!("{home}/.zshrc"),
            format!("{home}/.profile"),
        ];

        for file_path in &shell_files {
            if std::path::Path::new(file_path).exists() {
                let mut content = fs::read_to_string(file_path)
                    .map_err(|e| format!("读取 {file_path} 失败: {e}"))?;

                for var in &CLI_PROXY_VARS[..4] {
                    let pattern = format!("export {var}=");
                    content = content
                        .lines()
                        .filter(|line| !line.trim_start().starts_with(&pattern))
                        .collect::<Vec<_>>()
                        .join("\n");
                    content.push_str(&format!("\nexport {var}={proxy_url}"));
                }

                let no_proxy = "localhost,127.0.0.1";
                for var in &["NO_PROXY", "no_proxy"] {
                    let pattern = format!("export {var}=");
                    content = content
                        .lines()
                        .filter(|line| !line.trim_start().starts_with(&pattern))
                        .collect::<Vec<_>>()
                        .join("\n");
                    content.push_str(&format!("\nexport {var}={no_proxy}"));
                }

                fs::write(file_path, content)
                    .map_err(|e| format!("写入 {file_path} 失败: {e}"))?;
            }
        }
    }

    set_tool_proxy_configs(proxy_url)?;
    Ok(())
}

/// 清除 CLI 工具代理环境变量
pub fn clear_cli_proxy_env() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu
            .open_subkey_with_flags("Environment", KEY_ALL_ACCESS)
            .map_err(|e| format!("打开注册表失败: {e}"))?;

        for var in &CLI_PROXY_VARS {
            let _ = env.delete_value(var);
        }

        broadcast_env_change();
    }

    #[cfg(not(target_os = "windows"))]
    {
        use std::env;
        let home = env::var("HOME").map_err(|_| "无法获取 HOME 环境变量")?;
        let shell_files = [
            format!("{home}/.bashrc"),
            format!("{home}/.zshrc"),
            format!("{home}/.profile"),
        ];

        for file_path in &shell_files {
            if std::path::Path::new(file_path).exists() {
                let content = fs::read_to_string(file_path)
                    .map_err(|e| format!("读取 {file_path} 失败: {e}"))?;

                let filtered: String = content
                    .lines()
                    .filter(|line| {
                        let trimmed = line.trim_start();
                        !CLI_PROXY_VARS
                            .iter()
                            .any(|var| trimmed.starts_with(&format!("export {var}=")))
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                fs::write(file_path, filtered)
                    .map_err(|e| format!("写入 {file_path} 失败: {e}"))?;
            }
        }
    }

    clear_tool_proxy_configs()?;
    Ok(())
}

/// 设置 git/pip/npm 等工具的代理配置
fn set_tool_proxy_configs(proxy_url: &str) -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;

    // git: ~/.gitconfig
    let _ = std::process::Command::new("git")
        .args(["config", "--global", "http.proxy", proxy_url])
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "--global", "https.proxy", proxy_url])
        .output();

    // pip: ~/pip/pip.ini (Windows) or ~/.config/pip/pip.conf (Unix)
    #[cfg(target_os = "windows")]
    let pip_dir = home.join("pip");
    #[cfg(not(target_os = "windows"))]
    let pip_dir = home.join(".config").join("pip");

    let _ = fs::create_dir_all(&pip_dir);
    #[cfg(target_os = "windows")]
    let pip_conf = pip_dir.join("pip.ini");
    #[cfg(not(target_os = "windows"))]
    let pip_conf = pip_dir.join("pip.conf");

    let pip_content = format!("[global]\nproxy = {proxy_url}\n");
    let _ = fs::write(&pip_conf, pip_content);

    // npm: npm config set proxy
    let _ = std::process::Command::new("npm")
        .args(["config", "set", "proxy", proxy_url])
        .output();
    let _ = std::process::Command::new("npm")
        .args(["config", "set", "https-proxy", proxy_url])
        .output();

    Ok(())
}

/// 清除 git/pip/npm 等工具的代理配置
fn clear_tool_proxy_configs() -> Result<(), String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;

    // git
    let _ = std::process::Command::new("git")
        .args(["config", "--global", "--unset", "http.proxy"])
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "--global", "--unset", "https.proxy"])
        .output();

    // pip
    #[cfg(target_os = "windows")]
    let pip_conf = home.join("pip").join("pip.ini");
    #[cfg(not(target_os = "windows"))]
    let pip_conf = home.join(".config").join("pip").join("pip.conf");

    if pip_conf.exists() {
        if let Ok(content) = fs::read_to_string(&pip_conf) {
            let filtered: String = content
                .lines()
                .filter(|l| !l.trim_start().starts_with("proxy"))
                .collect::<Vec<_>>()
                .join("\n");
            let _ = fs::write(&pip_conf, filtered);
        }
    }

    // npm
    let _ = std::process::Command::new("npm")
        .args(["config", "delete", "proxy"])
        .output();
    let _ = std::process::Command::new("npm")
        .args(["config", "delete", "https-proxy"])
        .output();

    Ok(())
}

/// 获取当前 CLI 代理环境变量及工具配置状态
pub fn get_cli_proxy_env() -> Option<String> {
    let mut parts: Vec<String> = Vec::new();

    // 环境变量
    #[cfg(target_os = "windows")]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(env) = hkcu.open_subkey("Environment") {
            if let Ok(value) = env.get_value::<String, _>("HTTP_PROXY") {
                parts.push(format!("ENV: {value}"));
            }
        }
        if parts.is_empty() {
            if let Ok(v) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
                parts.push(format!("ENV: {v}"));
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(v) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
            parts.push(format!("ENV: {v}"));
        }
    }

    // git
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--global", "http.proxy"])
        .output()
    {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !val.is_empty() {
            parts.push(format!("Git: {val}"));
        }
    }

    // npm
    if let Ok(output) = std::process::Command::new("npm")
        .args(["config", "get", "proxy"])
        .output()
    {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !val.is_empty() && val != "null" && val != "undefined" {
            parts.push(format!("npm: {val}"));
        }
    }

    // pip
    let home = dirs::home_dir();
    if let Some(home) = home {
        #[cfg(target_os = "windows")]
        let pip_conf = home.join("pip").join("pip.ini");
        #[cfg(not(target_os = "windows"))]
        let pip_conf = home.join(".config").join("pip").join("pip.conf");

        if let Ok(content) = fs::read_to_string(&pip_conf) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("proxy") {
                    if let Some(val) = trimmed.split('=').nth(1) {
                        let val = val.trim();
                        if !val.is_empty() {
                            parts.push(format!("pip: {val}"));
                        }
                    }
                }
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" | "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_dir_creation() {
        let backup_dir = get_backup_dir();
        assert!(backup_dir.is_ok());
    }
}
