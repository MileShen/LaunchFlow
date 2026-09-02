use serde::{Deserialize, Serialize};
use std::os::windows::{ffi::OsStrExt, process::CommandExt};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;
use walkdir::WalkDir;
use windows::{
    core::{Interface, PCWSTR},
    Win32::{
        Storage::FileSystem::WIN32_FIND_DATAW,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
            COINIT_APARTMENTTHREADED, STGM_READ,
        },
        UI::Shell::{IShellLinkW, ShellLink, SLGP_RAWPATH},
    },
};
use winreg::{enums::*, RegKey};

const APP_NAME: &str = "一键启动器";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_TASK: &str = "LaunchFlow OneClickLauncher";
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Serialize, Deserialize)]
struct AppEntry {
    name: String,
    path: String,
    #[serde(default)]
    args: String,
    #[serde(default)]
    working_dir: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    run_as_admin: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct Profile {
    id: String,
    name: String,
    #[serde(default)]
    apps: Vec<AppEntry>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    profiles: Vec<Profile>,
    #[serde(default)]
    auto_launch_enabled: bool,
    #[serde(default)]
    auto_launch_profile_id: String,
    #[serde(default = "default_theme")]
    theme: String,
}

fn default_true() -> bool {
    true
}
fn default_theme() -> String {
    "system".into()
}

fn default_config() -> Config {
    Config {
        profiles: vec![Profile {
            id: Uuid::new_v4().simple().to_string(),
            name: "默认配置".into(),
            apps: vec![],
        }],
        auto_launch_enabled: false,
        auto_launch_profile_id: String::new(),
        theme: default_theme(),
    }
}

fn config_path() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|p| p.join("OneClickLauncher").join("config.json"))
        .ok_or_else(|| "无法获取用户配置目录".into())
}

#[tauri::command]
fn load_config() -> Result<Config, String> {
    let path = config_path()?;
    if !path.exists() {
        return Ok(default_config());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut config: Config = serde_json::from_str(&text).unwrap_or_else(|_| default_config());
    if config.profiles.is_empty() {
        config.profiles = default_config().profiles;
    }
    Ok(config)
}

#[tauri::command]
fn save_config(config: Config) -> Result<(), String> {
    let path = config_path()?;
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    fs::write(
        &temp,
        serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}

struct ShortcutDetails {
    target: String,
    arguments: String,
    working_dir: String,
}

fn wide_string(buffer: &[u16]) -> String {
    let length = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..length])
}

fn resolve_shortcut(path: &Path) -> windows::core::Result<ShortcutDetails> {
    unsafe {
        let shell_link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        let persist: IPersistFile = shell_link.cast()?;
        let wide_path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        persist.Load(PCWSTR(wide_path.as_ptr()), STGM_READ)?;

        let mut target = [0u16; 32768];
        let mut arguments = [0u16; 4096];
        let mut working_dir = [0u16; 32768];
        let mut find_data = WIN32_FIND_DATAW::default();
        shell_link.GetPath(&mut target, &mut find_data, SLGP_RAWPATH.0 as u32)?;
        shell_link.GetArguments(&mut arguments)?;
        shell_link.GetWorkingDirectory(&mut working_dir)?;
        Ok(ShortcutDetails {
            target: wide_string(&target),
            arguments: wide_string(&arguments),
            working_dir: wide_string(&working_dir),
        })
    }
}

fn is_uninstall_shortcut(name: &str) -> bool {
    let normalized = name.trim().to_lowercase();
    const UNINSTALL_KEYWORDS: &[&str] = &[
        "uninstall",
        "uninstaller",
        "unins0",
        "卸载",
        "解除安装",
        "解除安裝",
        "desinstalar",
        "désinstaller",
        "deinstallieren",
        "удалить",
    ];
    UNINSTALL_KEYWORDS
        .iter()
        .any(|keyword| normalized.contains(keyword))
}

fn is_uninstall_target(details: &ShortcutDetails) -> bool {
    let filename = Path::new(&details.target)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if is_uninstall_shortcut(&filename) {
        return true;
    }
    let arguments = details.arguments.to_lowercase();
    (filename == "msiexec.exe" || filename == "msiexec")
        && (arguments.contains("/x") || arguments.contains("/uninstall"))
}

#[tauri::command]
async fn installed_applications() -> Result<Vec<AppEntry>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let com_initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
        let mut items = vec![];
        let mut seen = HashSet::new();
        let roots = [
            std::env::var("APPDATA")
                .ok()
                .map(|v| PathBuf::from(v).join("Microsoft/Windows/Start Menu/Programs")),
            std::env::var("PROGRAMDATA")
                .ok()
                .map(|v| PathBuf::from(v).join("Microsoft/Windows/Start Menu/Programs")),
        ];
        for root in roots.into_iter().flatten().filter(|p| p.exists()) {
            for entry in WalkDir::new(root)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                let p = entry.path();
                if p.extension()
                    .and_then(|v| v.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk"))
                {
                    let name = p.file_stem().unwrap_or_default().to_string_lossy();
                    if is_uninstall_shortcut(&name) {
                        continue;
                    }
                    let Ok(details) = resolve_shortcut(p) else {
                        continue;
                    };
                    if details.target.is_empty() || is_uninstall_target(&details) {
                        continue;
                    }
                    let key = format!(
                        "{}\0{}",
                        details.target.to_lowercase(),
                        details.arguments.to_lowercase()
                    );
                    if !seen.insert(key) {
                        continue;
                    }
                    let working_dir = if details.working_dir.is_empty() {
                        Path::new(&details.target)
                            .parent()
                            .unwrap_or(Path::new(""))
                            .to_string_lossy()
                            .into()
                    } else {
                        details.working_dir
                    };
                    items.push(AppEntry {
                        name: name.into_owned(),
                        path: p.to_string_lossy().into(),
                        args: String::new(),
                        working_dir,
                        enabled: true,
                        run_as_admin: false,
                    });
                }
            }
        }
        items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        if com_initialized {
            unsafe {
                CoUninitialize();
            }
        }
        items
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn launch_apps(apps: Vec<AppEntry>) -> Vec<String> {
    let mut errors = vec![];
    for app in apps.into_iter().filter(|a| a.enabled) {
        let path = Path::new(&app.path);
        if !path.exists() {
            errors.push(format!("{}：文件不存在", app.name));
            continue;
        }
        let work_dir = if app.working_dir.is_empty() {
            path.parent().unwrap_or(Path::new(""))
        } else {
            Path::new(&app.working_dir)
        };
        let result = if !app.run_as_admin {
            Command::new("powershell.exe")
                .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command"])
                .arg("$shell = New-Object -ComObject Shell.Application; $shell.ShellExecute($env:LAUNCHFLOW_PATH, $env:LAUNCHFLOW_ARGS, $env:LAUNCHFLOW_CWD, 'open', 1)")
                .env("LAUNCHFLOW_PATH", &app.path)
                .env("LAUNCHFLOW_ARGS", &app.args)
                .env("LAUNCHFLOW_CWD", work_dir)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
        } else {
            let ext = path
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ["lnk", "url", "bat", "cmd"].contains(&ext.as_str()) {
                Command::new("cmd")
                    .args(["/C", "start", "", &app.path])
                    .current_dir(work_dir)
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
            } else {
                let mut command = Command::new(path);
                command.current_dir(work_dir);
                if !app.args.trim().is_empty() {
                    command.args(split_windows_args(&app.args));
                }
                command.spawn()
            }
        };
        if let Err(e) = result {
            errors.push(format!("{}：{}", app.name, e));
        }
    }
    errors
}

fn split_windows_args(value: &str) -> Vec<String> {
    let mut result = vec![];
    let mut current = String::new();
    let mut quoted = false;
    for ch in value.chars() {
        match ch {
            '"' => quoted = !quoted,
            ' ' | '\t' if !quoted => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

#[tauri::command]
fn startup_enabled() -> bool {
    let exists = Command::new("schtasks.exe")
        .args(["/Query", "/TN", STARTUP_TASK])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .is_ok_and(|status| status.success());
    if exists {
        return true;
    }
    let legacy = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .ok()
        .and_then(|key| key.get_value::<String, _>(APP_NAME).ok())
        .is_some();
    legacy && set_startup(true).is_ok()
}

#[tauri::command]
fn set_startup(enabled: bool) -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    let _ = key.delete_value(APP_NAME);
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let command = format!("\"{}\"", exe.display());
        let output = Command::new("schtasks.exe")
            .args([
                "/Create",
                "/TN",
                STARTUP_TASK,
                "/SC",
                "ONLOGON",
                "/RL",
                "HIGHEST",
                "/TR",
                &command,
                "/F",
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    } else {
        let output = Command::new("schtasks.exe")
            .args(["/Delete", "/TN", STARTUP_TASK, "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        if output.status.success() || output.status.code() == Some(1) {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

#[tauri::command]
fn browse_executable() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("应用程序", &["exe", "lnk", "bat", "cmd", "url"])
        .add_filter("所有文件", &["*"])
        .pick_file()
        .map(|p| p.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            installed_applications,
            launch_apps,
            startup_enabled,
            set_startup,
            browse_executable
        ])
        .run(tauri::generate_context!())
        .expect("error while running application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_uninstall_shortcut_names() {
        for name in ["卸载软件", "Uninstall App", "unins000", "解除安裝程式"] {
            assert!(is_uninstall_shortcut(name), "should filter {name}");
        }
        assert!(!is_uninstall_shortcut("Visual Studio Code"));
    }

    #[test]
    fn filters_msi_uninstall_targets() {
        let uninstall = ShortcutDetails {
            target: r"C:\Windows\System32\msiexec.exe".into(),
            arguments: "/x {PRODUCT-ID}".into(),
            working_dir: String::new(),
        };
        assert!(is_uninstall_target(&uninstall));
    }
}
