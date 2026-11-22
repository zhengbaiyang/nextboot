import { useCallback, useEffect, useMemo, useState } from "react";
import {
  applyBootSwitch,
  clearBootNext,
  getBootState,
  normalizeError,
  rebootNow,
  relaunchElevated,
} from "./api";
import { BootEntryCard } from "./components/BootEntryCard";
import {
  ConfirmRebootModal,
  type RebootModalRequest,
} from "./components/ConfirmRebootModal";
import { LegacyBiosNotice } from "./components/LegacyBiosNotice";
import { StatusBanner } from "./components/StatusBanner";
import type { ApiError, BootEntry, BootState, SwitchMode } from "./types";

interface ToastMessage {
  kind: "success" | "error";
  text: string;
}

export function App() {
  const [state, setState] = useState<BootState | null>(null);
  const [loadError, setLoadError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const [modalReq, setModalReq] = useState<RebootModalRequest | null>(null);
  const [toast, setToast] = useState<ToastMessage | null>(null);

  const pushToast = useCallback((kind: "success" | "error", text: string) => {
    setToast({ kind, text });
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 5000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const refreshState = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const data = await getBootState();
      setState(data);
    } catch (err) {
      setLoadError(normalizeError(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshState();
  }, [refreshState]);

  const visibleEntries = useMemo(() => {
    if (!state) return [];
    if (showAll) return state.entries;
    return state.entries.filter(
      (e) => e.is_primary_os || e.is_current || e.is_default || e.is_next
    );
  }, [state, showAll]);

  const hiddenCount = useMemo(() => {
    if (!state) return 0;
    return state.entries.length - visibleEntries.length;
  }, [state, visibleEntries]);

  const handleApplyOnly = async (entry: BootEntry, mode: SwitchMode) => {
    setBusy(true);
    try {
      const res = await applyBootSwitch(entry.id_hex, mode, false, false);
      if (res.state) {
        setState(res.state);
      } else {
        await refreshState();
      }
      pushToast("success", res.message);
    } catch (err) {
      const parsed = normalizeError(err);
      pushToast("error", parsed.message);
    } finally {
      setBusy(false);
    }
  };

  const handleClearNext = async () => {
    setBusy(true);
    try {
      const res = await clearBootNext();
      if (res.state) {
        setState(res.state);
      } else {
        await refreshState();
      }
      pushToast("success", res.message);
    } catch (err) {
      pushToast("error", normalizeError(err).message);
    } finally {
      setBusy(false);
    }
  };

  const handleConfirmReboot = async (mode: SwitchMode, forceReboot: boolean) => {
    if (!modalReq) return;
    setBusy(true);
    try {
      const res = modalReq.entry
        ? await applyBootSwitch(modalReq.entry.id_hex, mode, true, forceReboot)
        : await rebootNow(forceReboot);
      setModalReq(null);
      if (res.state) {
        setState(res.state);
      }
      pushToast("success", res.message);
    } catch (err) {
      pushToast("error", normalizeError(err).message);
    } finally {
      setBusy(false);
    }
  };

  const handleRelaunchAdmin = async () => {
    try {
      await relaunchElevated();
    } catch (err) {
      pushToast("error", normalizeError(err).message);
    }
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark" aria-hidden="true">
            <span className="mark-bar mark-bar-win" />
            <span className="mark-bar mark-bar-ubu" />
          </div>
          <div>
            <div className="brand-title-row">
              <h1>NextBoot</h1>
              {state?.is_mock && <span className="badge badge-mock">模拟模式</span>}
              {state && (
                <span className="env-pill">
                  {state.host_os === "windows" ? "Windows" : "Ubuntu / Linux"} ·{" "}
                  {state.firmware === "uefi" ? "UEFI" : "非 UEFI"}
                </span>
              )}
            </div>
            <p className="brand-sub">跨平台 UEFI 多系统重启切换 · 远控防掉线助手</p>
          </div>
        </div>

        <button
          type="button"
          className="btn btn-ghost"
          disabled={loading || busy}
          onClick={() => void refreshState()}
        >
          {loading ? "读取中..." : "刷新状态"}
        </button>
      </header>

      <main className="main-content">
        {loading && !state && (
          <div className="empty-state">正在读取主板 UEFI 引导信息...</div>
        )}

        {loadError && (
          <section className="notice-panel notice-panel-error" role="alert">
            <div className="notice-icon" aria-hidden="true">
              !
            </div>
            <h2>
              {loadError.code === "permission_denied"
                ? "需要管理员权限读取 UEFI 引导项"
                : "读取主板引导信息失败"}
            </h2>
            <p>{loadError.message}</p>
            <div className="notice-actions">
              {loadError.code === "permission_denied" && (
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void handleRelaunchAdmin()}
                >
                  以管理员身份重新打开
                </button>
              )}
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => void refreshState()}
              >
                重试读取
              </button>
            </div>
          </section>
        )}

        {state && state.firmware !== "uefi" && (
          <LegacyBiosNotice firmware={state.firmware} hostOs={state.host_os} />
        )}

        {state && state.firmware === "uefi" && (
          <>
            <StatusBanner
              state={state}
              busy={busy}
              onClearNext={() => void handleClearNext()}
              onQuickReboot={() => setModalReq({ entry: null, initialMode: "next" })}
            />

            <section className="entries-section">
              <div className="section-header">
                <div>
                  <h2>操作系统启动项</h2>
                  <span className="section-hint">
                    已按主板 BootOrder 排序 · 点击可单独预设或立即重启切换
                  </span>
                </div>

                <label className="toggle-all">
                  <input
                    type="checkbox"
                    checked={showAll}
                    onChange={(e) => setShowAll(e.target.checked)}
                  />
                  <span>
                    显示全部 UEFI 项（高级）
                    {!showAll && hiddenCount > 0 ? ` · 已隐藏 ${hiddenCount} 项` : ""}
                  </span>
                </label>
              </div>

              {visibleEntries.length === 0 ? (
                <div className="empty-state">
                  未识别到常见操作系统启动项，请勾选上方「显示全部 UEFI 项（高级）」查看主板完整列表。
                </div>
              ) : (
                <div className="entries-list">
                  {visibleEntries.map((entry) => (
                    <BootEntryCard
                      key={entry.id_hex}
                      entry={entry}
                      busy={busy}
                      onApplyOnly={(e, m) => void handleApplyOnly(e, m)}
                      onRequestReboot={(e, m) =>
                        setModalReq({ entry: e, initialMode: m })
                      }
                    />
                  ))}
                </div>
              )}
            </section>
          </>
        )}
      </main>

      <footer className="footer-bar">
        <span>
          提示：「仅下次生效」重启一次后自动恢复默认；如担心 Windows 自动更新多次重启掉回 Ubuntu，可使用「设为长期默认」。
        </span>
      </footer>

      {toast && (
        <div
          className={`toast ${toast.kind === "error" ? "toast-error" : "toast-success"}`}
          role="status"
        >
          <span>{toast.text}</span>
          <button
            type="button"
            className="toast-close"
            aria-label="关闭提示"
            onClick={() => setToast(null)}
          >
            ×
          </button>
        </div>
      )}

      <ConfirmRebootModal
        request={modalReq}
        busy={busy}
        isMock={Boolean(state?.is_mock)}
        onCancel={() => setModalReq(null)}
        onConfirm={(mode, force) => void handleConfirmReboot(mode, force)}
      />
    </div>
  );
}
