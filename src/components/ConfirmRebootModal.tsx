import { useEffect, useState } from "react";
import type { BootEntry, SwitchMode } from "../types";
import { OsIcon } from "./OsIcon";
import { ToggleSwitch } from "./ToggleSwitch";

export interface RebootModalRequest {
  /** Null means reboot now using existing BootNext / BootOrder without mutating NVRAM */
  entry: BootEntry | null;
  initialMode: SwitchMode;
}

interface ConfirmRebootModalProps {
  request: RebootModalRequest | null;
  busy: boolean;
  isMock: boolean;
  onCancel: () => void;
  onConfirm: (mode: SwitchMode, forceReboot: boolean) => void;
}

export function ConfirmRebootModal({
  request,
  busy,
  isMock,
  onCancel,
  onConfirm,
}: ConfirmRebootModalProps) {
  const [mode, setMode] = useState<SwitchMode>("next");
  const [forceReboot, setForceReboot] = useState(false);

  useEffect(() => {
    if (request) {
      setMode(request.initialMode);
      setForceReboot(false);
    }
  }, [request]);

  useEffect(() => {
    if (!request) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !busy) {
        onCancel();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [request, busy, onCancel]);

  if (!request) return null;

  const { entry } = request;

  return (
    <div
      className="modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby="modal-title"
      onClick={(e) => {
        if (e.target === e.currentTarget && !busy) onCancel();
      }}
    >
      <div className="modal-card">
        <div className="modal-header">
          <div className="modal-header-main">
            {entry ? (
              <OsIcon kind={entry.os_kind} size="sm" />
            ) : (
              <div className="modal-power-icon" aria-hidden="true">
                <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
                  <path d="M12 2v10m5.6-6.6a8 8 0 1 1-11.2 0" />
                </svg>
              </div>
            )}
            <div>
              <div className="modal-title-row">
                <h2 id="modal-title">
                  {entry ? `切换到 ${entry.display_name} 并重启` : "确认立即重启电脑"}
                </h2>
                {entry && <span className="mono-pill">Boot{entry.id_hex}</span>}
              </div>
              <p className="modal-subtitle">
                {isMock
                  ? "模拟模式运行中 · 本次操作不会触发真实系统重启"
                  : "写入主板 UEFI NVRAM 后立即向系统发起安全重启"}
              </p>
            </div>
          </div>

          <button
            type="button"
            className="modal-close-btn"
            aria-label="关闭弹窗"
            disabled={busy}
            onClick={onCancel}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
              <path d="M6 6l12 12M18 6 6 18" />
            </svg>
          </button>
        </div>

        <div className="modal-body">
          {entry ? (
            <>
              <div className="radio-stack" role="radiogroup" aria-label="选择切换模式">
                <button
                  type="button"
                  role="radio"
                  aria-checked={mode === "next"}
                  className={`radio-option-card ${mode === "next" ? "is-selected" : ""}`}
                  onClick={() => setMode("next")}
                >
                  <span className="radio-ring" aria-hidden="true">
                    <span className="radio-ring-dot" />
                  </span>
                  <div className="radio-option-body">
                    <div className="radio-option-header">
                      <span className="radio-option-title">
                        仅下一次重启生效 (BootNext)
                      </span>
                      <span className="micro-badge micro-badge-cobalt">推荐</span>
                    </div>
                    <div className="radio-option-desc">
                      本次重启进入 {entry.display_name}；后续再次重启时自动恢复原有长期默认系统。
                    </div>
                  </div>
                </button>

                <button
                  type="button"
                  role="radio"
                  aria-checked={mode === "default"}
                  className={`radio-option-card ${mode === "default" ? "is-selected" : ""}`}
                  onClick={() => setMode("default")}
                >
                  <span className="radio-ring" aria-hidden="true">
                    <span className="radio-ring-dot" />
                  </span>
                  <div className="radio-option-body">
                    <div className="radio-option-header">
                      <span className="radio-option-title">
                        同时设为长期默认 (BootOrder)
                      </span>
                      <span className="micro-badge micro-badge-slate">防掉回</span>
                    </div>
                    <div className="radio-option-desc">
                      将 {entry.display_name} 置顶至 BootOrder #1，避免系统更新多次自动重启时掉回其他系统。
                    </div>
                  </div>
                </button>
              </div>

              {entry.risk_warning && (
                <div className="modal-risk-box" role="alert">
                  <strong>高风险提醒：</strong>
                  {entry.risk_warning}
                </div>
              )}
            </>
          ) : (
            <div className="modal-plain-box">
              将按当前已生效的 UEFI 引导配置立即重启计算机，请确认远程会话中的文档与终端任务已妥善保存。
            </div>
          )}

          <div className={`force-switch-card ${forceReboot ? "is-active" : ""}`}>
            <div className="force-switch-copy">
              <label htmlFor="force-reboot-switch" className="force-switch-title">
                强制重启（忽略未保存程序阻塞 / systemd 抑制锁）
              </label>
              <div className="force-switch-hint">
                默认关闭以保护未保存文档；远控场景如担心后台程序阻塞关机界面，可开启此项。
              </div>
            </div>
            <ToggleSwitch
              id="force-reboot-switch"
              checked={forceReboot}
              onChange={setForceReboot}
              disabled={busy}
              tone="danger"
              ariaLabel="强制重启"
            />
          </div>
        </div>

        <div className="modal-footer">
          <button
            type="button"
            className="pill-btn pill-btn-secondary"
            disabled={busy}
            onClick={onCancel}
          >
            取消
          </button>
          <button
            type="button"
            className={`pill-btn ${forceReboot ? "pill-btn-danger" : "pill-btn-ink"}`}
            disabled={busy}
            onClick={() => onConfirm(mode, forceReboot)}
          >
            {busy
              ? "正在执行..."
              : forceReboot
                ? "确认并强制重启"
                : "确认并立即重启"}
          </button>
        </div>
      </div>
    </div>
  );
}
