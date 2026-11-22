import { useEffect, useState } from "react";
import type { BootEntry, SwitchMode } from "../types";

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

  if (!request) return null;

  const { entry } = request;

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" aria-labelledby="modal-title">
      <div className="modal-card">
        <div className="modal-header">
          <h2 id="modal-title">
            {entry ? `确认切换到 ${entry.display_name} 并重启` : "确认立即重启电脑"}
          </h2>
          {isMock && <span className="badge badge-mock">模拟模式（不会真实重启）</span>}
        </div>

        <div className="modal-body">
          {entry ? (
            <>
              <p className="modal-desc">
                目标启动项：<strong>{entry.display_name}</strong>{" "}
                <span className="boot-id-chip">Boot{entry.id_hex}</span>
              </p>

              <div className="mode-selector" role="radiogroup" aria-label="选择切换模式">
                <label className={`mode-option ${mode === "next" ? "mode-option-active" : ""}`}>
                  <input
                    type="radio"
                    name="switch-mode"
                    checked={mode === "next"}
                    onChange={() => setMode("next")}
                  />
                  <div>
                    <div className="mode-title">仅下一次重启生效 (BootNext)</div>
                    <div className="mode-hint">
                      本次重启进入 {entry.display_name}；后续再次重启时恢复原长期默认系统。
                    </div>
                  </div>
                </label>

                <label className={`mode-option ${mode === "default" ? "mode-option-active" : ""}`}>
                  <input
                    type="radio"
                    name="switch-mode"
                    checked={mode === "default"}
                    onChange={() => setMode("default")}
                  />
                  <div>
                    <div className="mode-title">同时设为长期默认 (置顶 BootOrder)</div>
                    <div className="mode-hint">
                      此后无论系统更新或自动重启多少次，都会默认进入 {entry.display_name}，直到手动改回。
                    </div>
                  </div>
                </label>
              </div>

              {entry.risk_warning && (
                <div className="modal-warning">
                  <strong>高风险提醒：</strong>
                  {entry.risk_warning}
                </div>
              )}
            </>
          ) : (
            <p className="modal-desc">
              将按当前已生效的 UEFI 引导设置立即重启计算机，请确保远程连接下的重要文档已保存。
            </p>
          )}

          <label className="force-checkbox">
            <input
              type="checkbox"
              checked={forceReboot}
              onChange={(e) => setForceReboot(e.target.checked)}
            />
            <div>
              <div className="force-title">强制重启（忽略未保存程序阻塞 / systemd 阻止关机锁）</div>
              <div className="force-hint">
                默认不勾选以保护未保存文档；远控时如担心被记事本等后台程序卡死在重启界面，可勾选此项。
              </div>
            </div>
          </label>
        </div>

        <div className="modal-footer">
          <button type="button" className="btn btn-secondary" disabled={busy} onClick={onCancel}>
            取消
          </button>
          <button
            type="button"
            className="btn btn-danger"
            disabled={busy}
            onClick={() => onConfirm(mode, forceReboot)}
          >
            {busy ? "正在执行..." : forceReboot ? "确认并强制重启" : "确认并立即重启"}
          </button>
        </div>
      </div>
    </div>
  );
}
