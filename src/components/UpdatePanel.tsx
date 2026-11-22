import type { ApiError, UpdateInfo, UpdateProgress } from "../types";
import { ToggleSwitch } from "./ToggleSwitch";

interface UpdatePanelProps {
  appVersion: string;
  updateInfo: UpdateInfo | null;
  checking: boolean;
  installing: boolean;
  progress: UpdateProgress | null;
  error: ApiError | null;
  autoCheck: boolean;
  onToggleAutoCheck: (next: boolean) => void;
  onCheckNow: () => void;
  onDownloadAndInstall: () => void;
  onOpenReleasePage: (url: string) => void;
}

function formatBytes(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes) || bytes < 0) return "未知大小";
  if (bytes < 1024) return `${bytes} B`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  if (mb < 1024) return `${mb.toFixed(2)} MB`;
  return `${(mb / 1024).toFixed(2)} GB`;
}

function formatPublishedDate(raw: string | null | undefined): string {
  if (!raw) return "发布日期未知";
  const parsed = new Date(raw);
  if (Number.isNaN(parsed.getTime())) return raw;
  return parsed.toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

function stageLabel(stage: string): string {
  switch (stage) {
    case "downloading":
      return "正在下载";
    case "installing":
      return "正在安装";
    case "done":
      return "已完成";
    default:
      return "处理中";
  }
}

function renderNotesLines(notes: string) {
  const lines = notes
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean);

  if (lines.length === 0) {
    return <p className="notes-empty">此版本暂无详细更新日志，可前往 GitHub Release 页面查看完整变更。</p>;
  }

  return (
    <div className="notes-list">
      {lines.map((line, idx) => {
        if (line.startsWith("#")) {
          const clean = line.replace(/^#+\s*/, "");
          return (
            <div key={idx} className="notes-heading">
              {clean}
            </div>
          );
        }
        if (line.startsWith("- ") || line.startsWith("* ")) {
          return (
            <div key={idx} className="notes-bullet">
              <span className="notes-bullet-dot" aria-hidden="true" />
              <span>{line.slice(2)}</span>
            </div>
          );
        }
        return (
          <p key={idx} className="notes-paragraph">
            {line}
          </p>
        );
      })}
    </div>
  );
}

export function UpdatePanel({
  appVersion,
  updateInfo,
  checking,
  installing,
  progress,
  error,
  autoCheck,
  onToggleAutoCheck,
  onCheckNow,
  onDownloadAndInstall,
  onOpenReleasePage,
}: UpdatePanelProps) {
  const currentVersion = updateInfo?.current_version ?? appVersion;
  const latestVersion = updateInfo?.latest_version ?? currentVersion;
  const hasUpdate = Boolean(updateInfo?.has_update);
  const canInstall = Boolean(
    hasUpdate && updateInfo?.download_url && updateInfo?.asset_name && !installing
  );
  const releaseUrl =
    updateInfo?.html_url ?? "https://github.com/zhengbaiyang/nextboot/releases/latest";

  return (
    <div className="update-layout">
      {/* Top Bento Grid: Obsidian Version Card + Auto-check / Asset Card */}
      <section className="update-bento-top" aria-label="版本概览与更新设置">
        <div className={`hero-bento-card update-hero-card ${hasUpdate ? "hero-has-next" : ""}`}>
          <div className="hero-dot-matrix" aria-hidden="true" />
          <div className="hero-glow" aria-hidden="true" />

          <div className="hero-inner">
            <div className="hero-top-row">
              <div className="uefi-live-pill">
                <span
                  className={`live-dot ${hasUpdate ? "live-dot-cobalt" : ""}`}
                  aria-hidden="true"
                />
                <span>
                  {checking
                    ? "正在连接 GitHub Releases..."
                    : hasUpdate
                      ? "发现新版本可用"
                      : "当前已是最新版本"}
                </span>
              </div>
              {updateInfo?.is_mock && (
                <span className="hero-count-pill">模拟更新源</span>
              )}
            </div>

            <div className="version-compare-row">
              <div className="version-block">
                <span className="flow-kicker">当前版本</span>
                <div className="version-number">v{currentVersion}</div>
              </div>

              <div className={`flow-arrow ${hasUpdate ? "flow-arrow-active" : ""}`} aria-hidden="true">
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M5 12h14m-5-5 5 5-5 5" />
                </svg>
              </div>

              <div className={`version-block ${hasUpdate ? "version-block-latest" : ""}`}>
                <span className="flow-kicker">GitHub 最新版本</span>
                <div className="version-number">
                  {updateInfo ? `v${latestVersion}` : "—"}
                </div>
              </div>
            </div>

            <div className="hero-footer-row">
              <span className="hero-status-note">
                {updateInfo?.published_at
                  ? `发布日期：${formatPublishedDate(updateInfo.published_at)}`
                  : "数据源：github.com/zhengbaiyang/nextboot"}
              </span>
              <span className="hero-mono-chip">GitHub Releases</span>
            </div>
          </div>
        </div>

        <div className="control-bento-card update-settings-card">
          <div className="control-row control-row-toggle">
            <div className="control-toggle-copy">
              <label htmlFor="auto-check-updates" className="control-toggle-title">
                启动时自动检查 GitHub Release 更新
              </label>
              <div className="control-toggle-sub">
                <span className="micro-count-pill">
                  仅在后台静默比对版本号，不会自动重启或安装
                </span>
              </div>
            </div>
            <ToggleSwitch
              id="auto-check-updates"
              checked={autoCheck}
              onChange={onToggleAutoCheck}
              ariaLabel="启动时自动检查 GitHub Release 更新"
            />
          </div>

          <div className="bento-divider" />

          <div className="asset-box">
            <div className="asset-box-header">
              <span className="control-label">匹配的安装包资源</span>
              {updateInfo?.asset_size != null && (
                <span className="mono-pill">{formatBytes(updateInfo.asset_size)}</span>
              )}
            </div>
            {updateInfo?.asset_name ? (
              <div className="asset-filename" title={updateInfo.asset_name}>
                <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                  <path d="M12 3v12m0 0-4-4m4 4 4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
                </svg>
                <span>{updateInfo.asset_name}</span>
              </div>
            ) : (
              <div className="asset-empty">
                {checking
                  ? "正在匹配当前平台安装包..."
                  : "未检测到当前平台的直装包，可前往发布页手动下载"}
              </div>
            )}
          </div>
        </div>
      </section>

      {error && (
        <div className="update-error-banner" role="alert">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <circle cx="12" cy="12" r="9" />
            <path d="M12 8v5m0 3h.01" />
          </svg>
          <span>检查或下载更新失败：{error.message}</span>
        </div>
      )}

      {/* Action & Live Cobalt Progress Bento Card */}
      <section className="bento-card update-action-card" aria-label="下载与安装更新">
        <div className="update-action-top">
          <div>
            <h2 className="bento-card-title">
              {updateInfo?.release_name || `NextBoot v${latestVersion}`}
            </h2>
            <p className="bento-card-sub">
              {hasUpdate
                ? "点击下方按钮将通过安全校验的 GitHub 通道流式下载安装包并自动拉起安装向导。"
                : "你正在运行最新稳定版本，可随时手动重新检查或前往 GitHub 查看历史版本。"}
            </p>
          </div>

          <div className="update-btn-group">
            <button
              type="button"
              className="pill-btn pill-btn-secondary"
              disabled={checking || installing}
              onClick={onCheckNow}
            >
              {checking ? "检查中..." : "重新检查"}
            </button>
            <button
              type="button"
              className="pill-btn pill-btn-secondary"
              onClick={() => onOpenReleasePage(releaseUrl)}
            >
              打开发布页
            </button>
            <button
              type="button"
              className={`pill-btn ${hasUpdate ? "pill-btn-cobalt" : "pill-btn-ink"}`}
              disabled={!canInstall}
              onClick={onDownloadAndInstall}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M12 3v12m0 0-4-4m4 4 4-4M4 17v2a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-2" />
              </svg>
              {installing
                ? "正在下载安装..."
                : hasUpdate
                  ? "下载并安装更新"
                  : "已是最新版本"}
            </button>
          </div>
        </div>

        {progress && (
          <div className="progress-panel" aria-live="polite">
            <div className="progress-header">
              <div className="progress-stage-group">
                <span
                  className={`sem-pill ${
                    progress.stage === "done" ? "sem-pill-current" : "sem-pill-next"
                  }`}
                >
                  {stageLabel(progress.stage)}
                </span>
                <span className="progress-message">{progress.message}</span>
              </div>
              <div className="progress-metrics">
                <span className="progress-bytes">
                  {formatBytes(progress.downloaded)}
                  {progress.total != null ? ` / ${formatBytes(progress.total)}` : ""}
                </span>
                <span className="progress-percent">{progress.percent}%</span>
              </div>
            </div>

            <div
              className="cobalt-progress-track"
              role="progressbar"
              aria-valuenow={progress.percent}
              aria-valuemin={0}
              aria-valuemax={100}
            >
              <div
                className={`cobalt-progress-fill ${
                  progress.stage === "done" ? "is-done" : "is-animated"
                }`}
                style={{ width: `${Math.max(2, Math.min(100, progress.percent))}%` }}
              />
            </div>
          </div>
        )}
      </section>

      {/* Release Notes Bento Card */}
      <section className="bento-card release-notes-card" aria-label="更新说明">
        <div className="release-notes-header">
          <h3 className="bento-card-title">版本更新说明</h3>
          <span className="mono-pill">v{latestVersion}</span>
        </div>
        {renderNotesLines(updateInfo?.release_notes ?? "")}
      </section>
    </div>
  );
}
