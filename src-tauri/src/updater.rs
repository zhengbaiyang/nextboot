use crate::ApiError;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

pub const GITHUB_LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/zhengbaiyang/nextboot/releases/latest";
pub const GITHUB_RELEASE_PAGE_PREFIX: &str = "https://github.com/zhengbaiyang/nextboot";
pub const UPDATE_PROGRESS_EVENT: &str = "update-progress";

const ALLOWED_DOWNLOAD_PREFIXES: &[&str] = &[
    "https://github.com/zhengbaiyang/nextboot/",
    "https://objects.githubusercontent.com/",
    "https://release-assets.githubusercontent.com/",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: Option<String>,
    pub html_url: String,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
    pub download_url: Option<String>,
    pub is_mock: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: u8,
    pub stage: String, // "downloading" | "installing" | "done"
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GitHubAsset {
    pub name: String,
    pub size: u64,
    pub browser_download_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TargetPlatform {
    Windows,
    Linux,
    Other,
}

impl TargetPlatform {
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Self::Other
        }
    }
}

pub fn normalize_version_str(raw: &str) -> &str {
    raw.trim().trim_start_matches(['v', 'V'])
}

pub fn is_newer_version(current: &str, latest: &str) -> bool {
    let Ok(cur) = semver::Version::parse(normalize_version_str(current)) else {
        return false;
    };
    let Ok(lat) = semver::Version::parse(normalize_version_str(latest)) else {
        return false;
    };
    lat > cur
}

pub fn select_best_asset<'a>(
    assets: &'a [GitHubAsset],
    platform: TargetPlatform,
) -> Option<&'a GitHubAsset> {
    match platform {
        TargetPlatform::Windows => {
            if let Some(setup) = assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().ends_with("-setup.exe"))
            {
                return Some(setup);
            }
            if let Some(msi) = assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().ends_with(".msi"))
            {
                return Some(msi);
            }
            assets
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case("NextBoot-portable-windows-x64.exe"))
        }
        TargetPlatform::Linux => assets
            .iter()
            .find(|a| a.name.to_ascii_lowercase().ends_with(".deb")),
        TargetPlatform::Other => None,
    }
}

pub fn validate_download_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("下载地址不能为空。".to_string());
    }
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err("下载地址包含非法空白或控制字符。".to_string());
    }
    if !ALLOWED_DOWNLOAD_PREFIXES
        .iter()
        .any(|prefix| trimmed.starts_with(prefix))
    {
        return Err(
            "仅允许从 NextBoot 官方 GitHub Releases 域名下载更新包。".to_string(),
        );
    }
    Ok(())
}

pub fn validate_release_page_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("发布页链接不能为空。".to_string());
    }
    if trimmed.chars().any(|c| {
        c.is_control()
            || c.is_whitespace()
            || matches!(c, '"' | '\'' | '`' | '|' | '<' | '>' | '^' | '&' | ';' | '$')
    }) {
        return Err("发布页链接包含非法字符。".to_string());
    }
    let Some(rest) = trimmed.strip_prefix(GITHUB_RELEASE_PAGE_PREFIX) else {
        return Err("仅允许打开 NextBoot 官方 GitHub 页面。".to_string());
    };
    if !rest.is_empty() && !rest.starts_with('/') && !rest.starts_with('?') && !rest.starts_with('#')
    {
        return Err("仅允许打开 NextBoot 官方 GitHub 页面。".to_string());
    }
    Ok(())
}

pub fn sanitize_asset_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return Err("更新包文件名无效。".to_string());
    }
    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains(':')
        || trimmed.chars().any(|c| {
            c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*')
        })
    {
        return Err("更新包文件名包含非法路径或特殊字符。".to_string());
    }

    let path = Path::new(trimmed);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(file_name)), None) if file_name == trimmed => {
            Ok(trimmed.to_string())
        }
        _ => Err("更新包文件名必须为单个安全文件名。".to_string()),
    }
}

pub fn compute_percent(downloaded: u64, total: Option<u64>) -> u8 {
    match total {
        Some(t) if t > 0 => {
            let pct = (downloaded as u128 * 100) / (t as u128);
            pct.min(100) as u8
        }
        _ => 0,
    }
}

pub fn mock_update_info(platform: TargetPlatform) -> UpdateInfo {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let latest_version = "0.3.0".to_string();
    let has_update = is_newer_version(&current_version, &latest_version);

    let (asset_name, asset_size, download_url) = match platform {
        TargetPlatform::Windows => (
            Some("NextBoot_0.3.0_x64-setup.exe".to_string()),
            Some(8_421_376),
            Some(
                "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot_0.3.0_x64-setup.exe"
                    .to_string(),
            ),
        ),
        TargetPlatform::Linux => (
            Some("NextBoot_0.3.0_amd64.deb".to_string()),
            Some(7_864_320),
            Some(
                "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot_0.3.0_amd64.deb"
                    .to_string(),
            ),
        ),
        TargetPlatform::Other => (None, None, None),
    };

    UpdateInfo {
        current_version,
        latest_version,
        has_update,
        release_name: "NextBoot v0.3.0（模拟预览）".to_string(),
        release_notes: concat!(
            "### 更新亮点\n",
            "- 扩展 20+ Linux 发行版自动识别与专属图标（Arch / Deepin / Manjaro / Fedora / 银河麒麟等）\n",
            "- 全新 Bento 极简界面，支持明暗主题切换与快速搜索筛选\n",
            "- 内置 GitHub Releases 更新检测、断点进度反馈与一键安装\n",
            "- 优化 Windows UAC 与 Linux Polkit 提权稳定性"
        )
        .to_string(),
        published_at: Some("2026-11-20T08:30:00Z".to_string()),
        html_url: "https://github.com/zhengbaiyang/nextboot/releases/tag/v0.3.0".to_string(),
        asset_name,
        asset_size,
        download_url,
        is_mock: true,
    }
}

fn build_http_client(timeout_secs: u64) -> Result<reqwest::Client, ApiError> {
    let ua = format!("NextBoot/{}", env!("CARGO_PKG_VERSION"));
    reqwest::Client::builder()
        .user_agent(ua)
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| ApiError {
            code: "http_client_error".to_string(),
            message: format!("初始化网络客户端失败: {e}"),
        })
}

pub async fn check_for_updates(use_mock: bool) -> Result<UpdateInfo, ApiError> {
    if use_mock || boot_core::env_requests_mock() {
        return Ok(mock_update_info(TargetPlatform::current()));
    }

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client = build_http_client(20)?;

    let response = client
        .get(GITHUB_LATEST_RELEASE_API)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| ApiError {
            code: "network_error".to_string(),
            message: format!("无法连接到 GitHub Releases，请检查网络或代理设置: {e}"),
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let message = match status.as_u16() {
            404 => "GitHub 仓库暂无已发布的正式 Release 版本。".to_string(),
            403 => "GitHub API 请求频率受限（403），请稍后再试或直接打开发布页。".to_string(),
            _ => format!("检查更新失败，GitHub 返回状态码 HTTP {status}。"),
        };
        return Err(ApiError {
            code: "github_api_error".to_string(),
            message,
        });
    }

    let release: GitHubRelease = response.json().await.map_err(|e| ApiError {
        code: "parse_error".to_string(),
        message: format!("解析 GitHub Release 数据失败: {e}"),
    })?;

    let latest_version = normalize_version_str(&release.tag_name).to_string();
    let has_update = is_newer_version(&current_version, &latest_version);
    let release_name = release
        .name
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| release.tag_name.clone());
    let release_notes = release.body.unwrap_or_default();
    let best_asset = select_best_asset(&release.assets, TargetPlatform::current());

    Ok(UpdateInfo {
        current_version,
        latest_version,
        has_update,
        release_name,
        release_notes,
        published_at: release.published_at,
        html_url: release.html_url,
        asset_name: best_asset.map(|a| a.name.clone()),
        asset_size: best_asset.map(|a| a.size),
        download_url: best_asset.map(|a| a.browser_download_url.clone()),
        is_mock: false,
    })
}

fn emit_progress(app: &tauri::AppHandle, progress: UpdateProgress) {
    let _ = app.emit(UPDATE_PROGRESS_EVENT, progress);
}

fn update_download_dir() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/tmp/nextboot-update")
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::temp_dir().join("nextboot-update")
    }
}

pub async fn download_and_install_update(
    app: tauri::AppHandle,
    use_mock: bool,
    download_url: String,
    asset_name: String,
) -> Result<String, ApiError> {
    let safe_name = sanitize_asset_name(&asset_name).map_err(|message| ApiError {
        code: "invalid_asset".to_string(),
        message,
    })?;

    if use_mock || boot_core::env_requests_mock() {
        let total: u64 = 8_421_376;
        let steps: &[(u8, &str, &str, u64)] = &[
            (0, "downloading", "正在连接更新服务器（模拟模式）...", 180),
            (35, "downloading", "正在下载更新包（模拟模式）...", 260),
            (72, "downloading", "正在下载更新包（模拟模式）...", 260),
            (100, "downloading", "更新包校验通过，准备安装（模拟模式）...", 200),
            (100, "installing", "正在启动安装程序（模拟模式）...", 320),
            (100, "done", "模拟更新完成（--mock 模式未执行真实下载与退出）。", 0),
        ];

        for &(percent, stage, msg, delay_ms) in steps {
            let downloaded = (total as u128 * percent as u128 / 100) as u64;
            emit_progress(
                &app,
                UpdateProgress {
                    downloaded,
                    total: Some(total),
                    percent,
                    stage: stage.to_string(),
                    message: msg.to_string(),
                },
            );
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }

        return Ok(format!(
            "模拟下载并安装 {} 完成（--mock 模式未修改系统或退出应用）。",
            safe_name
        ));
    }

    validate_download_url(&download_url).map_err(|message| ApiError {
        code: "invalid_url".to_string(),
        message,
    })?;

    let dir = update_download_dir();
    tokio::fs::create_dir_all(&dir).await.map_err(|e| ApiError {
        code: "io_error".to_string(),
        message: format!("创建更新临时目录失败: {e}"),
    })?;
    let dest_path = dir.join(&safe_name);

    let client = build_http_client(300)?;
    emit_progress(
        &app,
        UpdateProgress {
            downloaded: 0,
            total: None,
            percent: 0,
            stage: "downloading".to_string(),
            message: format!("正在连接下载 {safe_name}..."),
        },
    );

    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| ApiError {
            code: "download_error".to_string(),
            message: format!("下载更新包失败: {e}"),
        })?;

    if !response.status().is_success() {
        return Err(ApiError {
            code: "download_error".to_string(),
            message: format!("下载更新包失败，HTTP 状态码: {}", response.status()),
        });
    }

    let total = response.content_length();
    let mut file = tokio::fs::File::create(&dest_path)
        .await
        .map_err(|e| ApiError {
            code: "io_error".to_string(),
            message: format!("无法写入更新文件 {}: {e}", dest_path.display()),
        })?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_percent: u8 = 0;
    let mut last_emit = Instant::now();

    emit_progress(
        &app,
        UpdateProgress {
            downloaded: 0,
            total,
            percent: 0,
            stage: "downloading".to_string(),
            message: format!("正在下载 {safe_name}..."),
        },
    );

    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res.map_err(|e| ApiError {
            code: "download_error".to_string(),
            message: format!("下载数据流中断: {e}"),
        })?;

        file.write_all(&chunk).await.map_err(|e| ApiError {
            code: "io_error".to_string(),
            message: format!("写入更新包失败: {e}"),
        })?;

        downloaded = downloaded.saturating_add(chunk.len() as u64);
        let percent = compute_percent(downloaded, total);
        if percent != last_percent || last_emit.elapsed() >= Duration::from_millis(120) {
            last_percent = percent;
            last_emit = Instant::now();
            emit_progress(
                &app,
                UpdateProgress {
                    downloaded,
                    total,
                    percent,
                    stage: "downloading".to_string(),
                    message: format!("正在下载 {safe_name}..."),
                },
            );
        }
    }

    file.flush().await.map_err(|e| ApiError {
        code: "io_error".to_string(),
        message: format!("刷新更新文件缓冲失败: {e}"),
    })?;
    drop(file);

    emit_progress(
        &app,
        UpdateProgress {
            downloaded,
            total: total.or(Some(downloaded)),
            percent: 100,
            stage: "installing".to_string(),
            message: format!("下载完成，正在启动安装程序 {safe_name}..."),
        },
    );

    install_downloaded_asset(&app, &dest_path, &safe_name, downloaded, total).await
}

#[cfg(target_os = "windows")]
async fn install_downloaded_asset(
    app: &tauri::AppHandle,
    dest_path: &Path,
    safe_name: &str,
    downloaded: u64,
    total: Option<u64>,
) -> Result<String, ApiError> {
    let lower = safe_name.to_ascii_lowercase();
    let spawn_res = if lower.ends_with(".msi") {
        std::process::Command::new("msiexec")
            .arg("/i")
            .arg(dest_path)
            .spawn()
    } else {
        std::process::Command::new(dest_path).spawn()
    };

    spawn_res.map_err(|e| ApiError {
        code: "install_error".to_string(),
        message: format!("启动安装程序 {} 失败: {e}", dest_path.display()),
    })?;

    emit_progress(
        app,
        UpdateProgress {
            downloaded,
            total: total.or(Some(downloaded)),
            percent: 100,
            stage: "done".to_string(),
            message: "安装程序已启动，NextBoot 即将关闭以完成更新。".to_string(),
        },
    );

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(600)).await;
        app_handle.exit(0);
    });

    Ok(format!(
        "已启动安装程序 {}，NextBoot 即将自动关闭以完成更新。",
        safe_name
    ))
}

#[cfg(target_os = "linux")]
async fn install_downloaded_asset(
    app: &tauri::AppHandle,
    dest_path: &Path,
    safe_name: &str,
    downloaded: u64,
    total: Option<u64>,
) -> Result<String, ApiError> {
    let path_owned = dest_path.to_path_buf();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("pkexec")
            .args(["apt-get", "install", "-y"])
            .arg(&path_owned)
            .output()
    })
    .await
    .map_err(|e| ApiError {
        code: "install_error".to_string(),
        message: format!("执行安装任务失败: {e}"),
    })?
    .map_err(|e| ApiError {
        code: "install_error".to_string(),
        message: format!("调用 pkexec apt-get 安装失败: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(ApiError {
            code: "install_error".to_string(),
            message: if detail.is_empty() {
                format!("安装更新包失败（退出码 {:?}）。", output.status.code())
            } else {
                format!("安装更新包失败: {detail}")
            },
        });
    }

    emit_progress(
        app,
        UpdateProgress {
            downloaded,
            total: total.or(Some(downloaded)),
            percent: 100,
            stage: "done".to_string(),
            message: "更新安装完成，请重启 NextBoot 以启用新版本。".to_string(),
        },
    );

    Ok(format!(
        "已通过 pkexec 成功安装 {}，请重新启动 NextBoot。",
        safe_name
    ))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
async fn install_downloaded_asset(
    _app: &tauri::AppHandle,
    _dest_path: &Path,
    _safe_name: &str,
    _downloaded: u64,
    _total: Option<u64>,
) -> Result<String, ApiError> {
    Err(ApiError {
        code: "unsupported".to_string(),
        message: "当前平台暂不支持自动安装，请前往 GitHub Releases 页面手动下载。".to_string(),
    })
}

pub fn open_release_page(url: String) -> Result<(), ApiError> {
    validate_release_page_url(&url).map_err(|message| ApiError {
        code: "invalid_url".to_string(),
        message,
    })?;

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url.trim()])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| ApiError {
                code: "open_url_error".to_string(),
                message: format!("打开默认浏览器失败: {e}"),
            })?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url.trim())
            .spawn()
            .map_err(|e| ApiError {
                code: "open_url_error".to_string(),
                message: format!("调用 xdg-open 打开浏览器失败: {e}"),
            })?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url.trim())
            .spawn()
            .map_err(|e| ApiError {
                code: "open_url_error".to_string(),
                message: format!("调用 open 打开浏览器失败: {e}"),
            })?;
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(ApiError {
            code: "unsupported".to_string(),
            message: "当前平台不支持直接打开浏览器链接。".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_assets() -> Vec<GitHubAsset> {
        vec![
            GitHubAsset {
                name: "NextBoot-portable-windows-x64.exe".to_string(),
                size: 12_000_000,
                browser_download_url: "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot-portable-windows-x64.exe".to_string(),
            },
            GitHubAsset {
                name: "NextBoot_0.3.0_x64_en-US.msi".to_string(),
                size: 9_500_000,
                browser_download_url: "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot_0.3.0_x64_en-US.msi".to_string(),
            },
            GitHubAsset {
                name: "NextBoot_0.3.0_x64-setup.exe".to_string(),
                size: 8_400_000,
                browser_download_url: "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot_0.3.0_x64-setup.exe".to_string(),
            },
            GitHubAsset {
                name: "NextBoot_0.3.0_amd64.deb".to_string(),
                size: 7_800_000,
                browser_download_url: "https://github.com/zhengbaiyang/nextboot/releases/download/v0.3.0/NextBoot_0.3.0_amd64.deb".to_string(),
            },
        ]
    }

    #[test]
    fn semver_comparison_handles_tags_and_ordering() {
        assert!(is_newer_version("0.2.0", "0.3.0"));
        assert!(is_newer_version("0.2.0", "v0.2.1"));
        assert!(is_newer_version("v0.2.0", "V1.0.0"));
        assert!(is_newer_version("0.2.0-beta.1", "0.2.0"));
        assert!(!is_newer_version("0.2.0", "0.2.0"));
        assert!(!is_newer_version("0.2.0", "v0.2.0"));
        assert!(!is_newer_version("0.3.0", "0.2.9"));
        assert!(!is_newer_version("0.2.0", "not-a-version"));
        assert!(!is_newer_version("bad", "0.3.0"));
    }

    #[test]
    fn windows_asset_selection_follows_priority_order() {
        let all = sample_assets();
        let best = select_best_asset(&all, TargetPlatform::Windows).expect("should match setup");
        assert_eq!(best.name, "NextBoot_0.3.0_x64-setup.exe");

        // Without *-setup.exe, falls back to *.msi
        let no_setup: Vec<_> = all
            .iter()
            .filter(|a| !a.name.ends_with("-setup.exe"))
            .cloned()
            .collect();
        let msi = select_best_asset(&no_setup, TargetPlatform::Windows).expect("should match msi");
        assert_eq!(msi.name, "NextBoot_0.3.0_x64_en-US.msi");

        // Without setup or msi, falls back to portable exe
        let only_portable: Vec<_> = no_setup
            .iter()
            .filter(|a| !a.name.ends_with(".msi"))
            .cloned()
            .collect();
        let portable =
            select_best_asset(&only_portable, TargetPlatform::Windows).expect("should match portable");
        assert_eq!(portable.name, "NextBoot-portable-windows-x64.exe");

        // With only Linux assets, returns None
        let linux_only = vec![all[3].clone()];
        assert!(select_best_asset(&linux_only, TargetPlatform::Windows).is_none());
    }

    #[test]
    fn linux_asset_selection_prefers_deb() {
        let all = sample_assets();
        let deb = select_best_asset(&all, TargetPlatform::Linux).expect("should match deb");
        assert_eq!(deb.name, "NextBoot_0.3.0_amd64.deb");

        let windows_only = vec![all[0].clone(), all[1].clone(), all[2].clone()];
        assert!(select_best_asset(&windows_only, TargetPlatform::Linux).is_none());
        assert!(select_best_asset(&[], TargetPlatform::Linux).is_none());
    }

    #[test]
    fn download_url_validation_accepts_trusted_hosts_only() {
        assert!(validate_download_url(
            "https://github.com/zhengbaiyang/nextboot/releases/download/v0.2.0/NextBoot_0.2.0_x64-setup.exe"
        )
        .is_ok());
        assert!(validate_download_url(
            "https://objects.githubusercontent.com/github-production-release-asset/12345"
        )
        .is_ok());
        assert!(validate_download_url(
            "https://release-assets.githubusercontent.com/github-production-release-asset/67890"
        )
        .is_ok());

        assert!(validate_download_url(
            "http://github.com/zhengbaiyang/nextboot/releases/download/v0.2.0/a.exe"
        )
        .is_err());
        assert!(validate_download_url("https://github.com/otheruser/nextboot/releases/a.exe").is_err());
        assert!(validate_download_url("https://evil.example.com/NextBoot_0.2.0_x64-setup.exe").is_err());
        assert!(validate_download_url(
            "https://github.com/zhengbaiyang/nextboot/releases/a.exe --bad"
        )
        .is_err());
        assert!(validate_download_url("").is_err());
    }

    #[test]
    fn release_page_url_validation_enforces_repo_prefix_and_safe_chars() {
        assert!(validate_release_page_url("https://github.com/zhengbaiyang/nextboot").is_ok());
        assert!(
            validate_release_page_url("https://github.com/zhengbaiyang/nextboot/releases/latest")
                .is_ok()
        );
        assert!(validate_release_page_url(
            "https://github.com/zhengbaiyang/nextboot/releases/tag/v0.2.0"
        )
        .is_ok());

        assert!(
            validate_release_page_url("https://github.com/zhengbaiyang/nextboot.evil.com/x")
                .is_err()
        );
        assert!(validate_release_page_url("https://github.com/someone/else").is_err());
        assert!(validate_release_page_url(
            "https://github.com/zhengbaiyang/nextboot/releases/latest&calc.exe"
        )
        .is_err());
        assert!(validate_release_page_url(
            "https://github.com/zhengbaiyang/nextboot/releases/latest\"calc"
        )
        .is_err());
    }

    #[test]
    fn asset_name_sanitization_blocks_path_traversal() {
        assert_eq!(
            sanitize_asset_name("NextBoot_0.2.0_x64-setup.exe").unwrap(),
            "NextBoot_0.2.0_x64-setup.exe"
        );
        assert_eq!(
            sanitize_asset_name("NextBoot_0.2.0_amd64.deb").unwrap(),
            "NextBoot_0.2.0_amd64.deb"
        );

        assert!(sanitize_asset_name("").is_err());
        assert!(sanitize_asset_name(".").is_err());
        assert!(sanitize_asset_name("..").is_err());
        assert!(sanitize_asset_name("../evil.exe").is_err());
        assert!(sanitize_asset_name("..\\evil.exe").is_err());
        assert!(sanitize_asset_name("sub/file.exe").is_err());
        assert!(sanitize_asset_name("C:\\Windows\\cmd.exe").is_err());
        assert!(sanitize_asset_name("bad*name.exe").is_err());
        assert!(sanitize_asset_name("bad\0name.exe").is_err());
    }

    #[test]
    fn percent_and_mock_info_work_as_expected() {
        assert_eq!(compute_percent(0, Some(100)), 0);
        assert_eq!(compute_percent(35, Some(100)), 35);
        assert_eq!(compute_percent(150, Some(100)), 100);
        assert_eq!(compute_percent(50, Some(0)), 0);
        assert_eq!(compute_percent(50, None), 0);

        let win_mock = mock_update_info(TargetPlatform::Windows);
        assert!(win_mock.is_mock);
        assert_eq!(win_mock.current_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(win_mock.latest_version, "0.3.0");
        assert!(win_mock.has_update);
        assert!(win_mock
            .asset_name
            .as_deref()
            .unwrap()
            .ends_with("-setup.exe"));

        let linux_mock = mock_update_info(TargetPlatform::Linux);
        assert!(linux_mock.asset_name.as_deref().unwrap().ends_with(".deb"));
    }
}
