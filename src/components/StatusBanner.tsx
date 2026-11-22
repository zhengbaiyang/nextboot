import type { BootEntry, BootState, UpdateInfo } from "../types";
import { OsIcon } from "./OsIcon";
import { ToggleSwitch } from "./ToggleSwitch";

interface StatusBannerProps {
  state: BootState;
  busy: boolean;
  showAll: boolean;
  hiddenCount: number;
  updateInfo: UpdateInfo | null;
  checkingUpdate: boolean;
  appVersion: string;
  onToggleShowAll: (next: boolean) => void;
  onClearNext: () => void;
  onQuickReboot: () => void;
  onOpenUpdateTab: () => void;
}

function resolveEntry(state: BootState, idHex: string | null): BootEntry | null {
  if (!idHex) return null;
  return state.entries.find((e) => e.id_hex === idHex) ?? null;
}

export function StatusBanner({
  state,
  busy,
  showAll,
  hiddenCount,
  updateInfo,
  checkingUpdate,
  appVersion,
  onToggleShowAll,
  onClearNext,
  onQuickReboot,
  onOpenUpdateTab,
}: StatusBannerProps) {
  const currentEntry = resolveEntry(state, state.boot_current);
  const defaultEntry = resolveEntry(state, state.default_boot);
  const nextEntry = resolveEntry(state, state.boot_next);
  const primaryCount = state.entries.filter((e) => e.is_primary_os).length;

  const targetEntry = nextEntry ?? defaultEntry;
  const hasExplicitNext = Boolean(state.boot_next);

  return (
    <section className="bento-overview" aria-label="引导状态总览与快捷控制">
      {/* Left: Signature Deep Obsidian Hero Card */}
      <div className={`hero-bento-card ${hasExplicitNext ? "hero-has-next" : ""}`}>
        <div className="hero-dot-matrix" aria-hidden="true" />
        <div className="hero-glow" aria-hidden="true" />

        <div className="hero-inner">
          <div className="hero-top-row">
            <div className="uefi-live-pill">
              <span className="live-dot" aria-hidden="true" />
              <span>
                UEFI 已就绪 · {state.host_os === "windows" ? "Windows" : "Linux"}
              </span>
            </div>
            <span className="hero-count-pill">
              {primaryCount} 系统 · {state.entries.length} 项
            </span>
          </div>

          <div className="boot-flow">
            <div className="flow-node">
              <span className="flow-kicker">当前运行</span>
              <div className="flow-os-name" title={currentEntry?.display_name ?? ""}>
                {currentEntry
                  ? currentEntry.display_name
                  : state.boot_current
                    ? `Boot${state.boot_current}`
                    : "未知系统"}
              </div>
              <span className="hero-mono-chip">
                {state.boot_current ? `Boot${state.boot_current}` : "BootCurrent —"}
              </span>
            </div>

            <div className={`flow-arrow ${hasExplicitNext ? "flow-arrow-active" : ""}`} aria-hidden="true">
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                <path d="M5 12h14m-5-5 5 5-5 5" />
              </svg>
            </div>

            <div className={`flow-node flow-node-target ${hasExplicitNext ? "is-scheduled" : ""}`}>
              <div className="flow-kicker-row">
                <span className="flow-kicker">
                  {hasExplicitNext ? "下次重启目标" : "下次重启（跟随默认）"}
                </span>
                {hasExplicitNext && <span className="hero-next-tag">BootNext</span>}
              </div>
              <div className="flow-os-name" title={targetEntry?.display_name ?? ""}>
                {nextEntry
                  ? nextEntry.display_name
                  : state.boot_next
                    ? `Boot${state.boot_next}`
                    : defaultEntry
                      ? defaultEntry.display_name
                      : "跟随默认顺序"}
              </div>
              <span className="hero-mono-chip">
                {state.boot_next
                  ? `Boot${state.boot_next} · 重启后自动清除`
                  : state.default_boot
                    ? `Boot${state.default_boot} · BootOrder #1`
                    : "未指定默认项"}
              </span>
            </div>
          </div>

          <div className="hero-footer-row">
            {hasExplicitNext ? (
              <>
                <span className="hero-status-note">已写入主板 NVRAM，等待重启切换</span>
                <div className="hero-inline-actions">
                  <button
                    type="button"
                    className="pill-btn pill-btn-hero-ghost"
                    disabled={busy}
                    onClick={onClearNext}
                  >
                    取消指定
                  </button>
                  <button
                    type="button"
                    className="pill-btn pill-btn-cobalt"
                    disabled={busy}
                    onClick={onQuickReboot}
                  >
                    <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                      <path d="M12 2v10m5.6-6.6a8 8 0 1 1-11.2 0" />
                    </svg>
                    立即重启生效
                  </button>
                </div>
              </>
            ) : (
              <>
                <span className="hero-status-note">
                  单次指定重启一次后自动回退默认，远控切换更安心
                </span>
                <button
                  type="button"
                  className="pill-btn pill-btn-hero-ghost"
                  disabled={busy}
                  onClick={onQuickReboot}
                >
                  立即重启
                </button>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Right: Crisp White/Elevated Bento Control Card */}
      <div className="control-bento-card">
        <div className="control-row control-row-default">
          <div className="control-default-left">
            <span className="control-label">长期默认 (BootOrder #1)</span>
            <div className="control-default-value">
              <span className="default-os-title">
                {defaultEntry
                  ? defaultEntry.display_name
                  : state.default_boot
                    ? `Boot${state.default_boot}`
                    : "未设置"}
              </span>
              {state.default_boot && (
                <span className="mono-pill">Boot{state.default_boot}</span>
              )}
            </div>
          </div>
          {defaultEntry && <OsIcon kind={defaultEntry.os_kind} size="sm" />}
        </div>

        <div className="bento-divider" />

        <div className="control-row control-row-toggle">
          <div className="control-toggle-copy">
            <label htmlFor="toggle-all-uefi" className="control-toggle-title">
              显示全部 UEFI 项（高级）
            </label>
            <div className="control-toggle-sub">
              {!showAll && hiddenCount > 0 ? (
                <span className="micro-count-pill">已隐藏 {hiddenCount} 项非系统项</span>
              ) : (
                <span className="micro-count-pill micro-count-pill-active">
                  已展开全部 {state.entries.length} 项
                </span>
              )}
            </div>
          </div>
          <ToggleSwitch
            id="toggle-all-uefi"
            checked={showAll}
            onChange={onToggleShowAll}
            ariaLabel="显示全部 UEFI 项（高级）"
          />
        </div>

        <div className="bento-divider" />

        <div className="control-row control-row-update">
          <div className="update-compact-status">
            <span
              className={`status-indicator-dot ${
                updateInfo?.has_update
                  ? "dot-cobalt-pulse"
                  : checkingUpdate
                    ? "dot-amber"
                    : "dot-emerald"
              }`}
              aria-hidden="true"
            />
            <span className="update-compact-text">
              {checkingUpdate
                ? "正在检查版本更新..."
                : updateInfo?.has_update
                  ? `发现新版本 v${updateInfo.latest_version}`
                  : `已是最新 v${updateInfo?.current_version ?? appVersion}`}
            </span>
          </div>
          <button
            type="button"
            className={`compact-action-pill ${updateInfo?.has_update ? "is-highlight" : ""}`}
            onClick={onOpenUpdateTab}
          >
            {updateInfo?.has_update ? "查看更新 →" : "版本管理"}
          </button>
        </div>
      </div>
    </section>
  );
}
