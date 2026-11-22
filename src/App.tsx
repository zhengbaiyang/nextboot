import { useCallback, useEffect, useMemo, useState } from "react";
import {
  APP_VERSION,
  applyBootSwitch,
  checkForUpdates,
  clearBootNext,
  downloadAndInstallUpdate,
  getBootState,
  normalizeError,
  openReleasePage,
  rebootNow,
  relaunchElevated,
  subscribeUpdateProgress,
} from "./api";
import { BootEntryCard } from "./components/BootEntryCard";
import {
  ConfirmRebootModal,
  type RebootModalRequest,
} from "./components/ConfirmRebootModal";
import { LegacyBiosNotice } from "./components/LegacyBiosNotice";
import { getFallbackOsLabel } from "./components/OsIcon";
import { StatusBanner } from "./components/StatusBanner";
import { UpdatePanel } from "./components/UpdatePanel";
import type {
  ActiveTab,
  ApiError,
  BootEntry,
  BootState,
  EntryFilter,
  SwitchMode,
  ThemeMode,
  UpdateInfo,
  UpdateProgress,
} from "./types";

const THEME_STORAGE_KEY = "nextboot.theme";
const AUTO_CHECK_STORAGE_KEY = "nextboot.autoCheckUpdate";

interface ToastMessage {
  kind: "success" | "error";
  text: string;
}

function readInitialTheme(): ThemeMode {
  try {
    const saved = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (saved === "dark" || saved === "light") return saved;
  } catch {
    // ignore storage errors
  }
  return "light";
}

function readInitialAutoCheck(): boolean {
  try {
    const saved = window.localStorage.getItem(AUTO_CHECK_STORAGE_KEY);
    if (saved === "false") return false;
  } catch {
    // ignore storage errors
  }
  return true;
}

export function App() {
  const [theme, setTheme] = useState<ThemeMode>(readInitialTheme);
  const [activeTab, setActiveTab] = useState<ActiveTab>("boot");

  const [state, setState] = useState<BootState | null>(null);
  const [loadError, setLoadError] = useState<ApiError | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);

  const [showAll, setShowAll] = useState(false);
  const [filter, setFilter] = useState<EntryFilter>("os");
  const [searchQuery, setSearchQuery] = useState("");

  const [modalReq, setModalReq] = useState<RebootModalRequest | null>(null);
  const [toast, setToast] = useState<ToastMessage | null>(null);

  // Auto-updater state
  const [autoCheckUpdate, setAutoCheckUpdate] = useState<boolean>(readInitialAutoCheck);
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<UpdateProgress | null>(null);
  const [updateError, setUpdateError] = useState<ApiError | null>(null);

  const pushToast = useCallback((kind: "success" | "error", text: string) => {
    setToast({ kind, text });
  }, []);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // ignore storage errors
    }
  }, [theme]);

  useEffect(() => {
    try {
      window.localStorage.setItem(
        AUTO_CHECK_STORAGE_KEY,
        autoCheckUpdate ? "true" : "false"
      );
    } catch {
      // ignore storage errors
    }
  }, [autoCheckUpdate]);

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

  const runUpdateCheck = useCallback(
    async (silent: boolean) => {
      setCheckingUpdate(true);
      if (!silent) setUpdateError(null);
      try {
        const info = await checkForUpdates();
        setUpdateInfo(info);
        setUpdateError(null);
        if (!silent) {
          pushToast(
            "success",
            info.has_update
              ? `发现新版本 v${info.latest_version}，可立即下载安装。`
              : `当前已是最新版本 v${info.current_version}。`
          );
        }
      } catch (err) {
        const parsed = normalizeError(err);
        if (!silent) {
          setUpdateError(parsed);
          pushToast("error", `检查更新失败：${parsed.message}`);
        }
      } finally {
        setCheckingUpdate(false);
      }
    },
    [pushToast]
  );

  useEffect(() => {
    void refreshState();
  }, [refreshState]);

  // Silent update check on startup when auto-check is enabled
  useEffect(() => {
    if (autoCheckUpdate) {
      void runUpdateCheck(true);
    }
    // Only trigger on initial mount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Subscribe to Tauri "update-progress" event
  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | null = null;

    void subscribeUpdateProgress((progress) => {
      if (active) {
        setUpdateProgress(progress);
      }
    }).then((fn) => {
      if (active) {
        unlisten = fn;
      } else {
        fn();
      }
    });

    return () => {
      active = false;
      if (unlisten) unlisten();
    };
  }, []);

  // Sync showAll switch <-> quick category filter pills seamlessly
  const handleToggleShowAll = useCallback((nextShowAll: boolean) => {
    setShowAll(nextShowAll);
    setFilter(nextShowAll ? "all" : "os");
  }, []);

  const handleSelectFilter = useCallback((nextFilter: EntryFilter) => {
    setFilter(nextFilter);
    setShowAll(nextFilter !== "os");
  }, []);

  const entryCounts = useMemo(() => {
    if (!state) return { all: 0, os: 0, advanced: 0 };
    const os = state.entries.filter(
      (e) => e.is_primary_os || e.is_current || e.is_default || e.is_next
    ).length;
    const advanced = state.entries.filter((e) => !e.is_primary_os).length;
    return { all: state.entries.length, os, advanced };
  }, [state]);

  const hiddenCount = useMemo(() => {
    if (!state) return 0;
    const primaryVisible = state.entries.filter(
      (e) => e.is_primary_os || e.is_current || e.is_default || e.is_next
    ).length;
    return Math.max(0, state.entries.length - primaryVisible);
  }, [state]);

  const visibleEntries = useMemo(() => {
    if (!state) return [];

    let list = state.entries;
    if (filter === "os" && !showAll) {
      list = list.filter(
        (e) => e.is_primary_os || e.is_current || e.is_default || e.is_next
      );
    } else if (filter === "advanced") {
      list = list.filter((e) => !e.is_primary_os);
    }

    const q = searchQuery.trim().toLowerCase();
    if (!q) return list;

    return list.filter((entry) => {
      const label = (entry.os_label ?? getFallbackOsLabel(entry.os_kind)).toLowerCase();
      return (
        entry.display_name.toLowerCase().includes(q) ||
        entry.description.toLowerCase().includes(q) ||
        entry.id_hex.toLowerCase().includes(q) ||
        `boot${entry.id_hex.toLowerCase()}`.includes(q) ||
        label.includes(q) ||
        (entry.efi_path?.toLowerCase().includes(q) ?? false)
      );
    });
  }, [state, filter, showAll, searchQuery]);

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
      pushToast("error", normalizeError(err).message);
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

  const handleDownloadAndInstall = async () => {
    if (!updateInfo?.download_url || !updateInfo.asset_name) return;
    setInstallingUpdate(true);
    setUpdateError(null);
    setUpdateProgress({
      downloaded: 0,
      total: updateInfo.asset_size,
      percent: 0,
      stage: "downloading",
      message: `正在准备下载 ${updateInfo.asset_name}...`,
    });
    try {
      const msg = await downloadAndInstallUpdate(
        updateInfo.download_url,
        updateInfo.asset_name
      );
      pushToast("success", msg);
    } catch (err) {
      const parsed = normalizeError(err);
      setUpdateError(parsed);
      pushToast("error", parsed.message);
    } finally {
      setInstallingUpdate(false);
    }
  };

  const handleOpenReleasePage = async (url: string) => {
    try {
      await openReleasePage(url);
    } catch (err) {
      pushToast("error", normalizeError(err).message);
    }
  };

  const handleOpenUpdateTab = () => {
    setActiveTab("update");
    if (!updateInfo && !checkingUpdate) {
      void runUpdateCheck(false);
    }
  };

  const currentVersionLabel = updateInfo?.current_version ?? APP_VERSION;

  return (
    <div className="app-shell">
      {/* Top Bar */}
      <header className="topbar">
        <div className="topbar-left">
          <div className="brand-mark" aria-hidden="true">
            <svg viewBox="0 0 24 24" width="20" height="20" fill="none">
              <path
                d="M5 8.5h11.5m0 0-3-3m3 3-3 3"
                stroke="#ffffff"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                d="M19 15.5H7.5m0 0 3-3m-3 3 3 3"
                stroke="#6b8cff"
                strokeWidth="2.2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          </div>
          <div className="brand-copy">
            <div className="brand-title-row">
              <h1>NextBoot</h1>
              <button
                type="button"
                className={`version-pill ${updateInfo?.has_update ? "has-update" : ""}`}
                onClick={handleOpenUpdateTab}
                title={
                  updateInfo?.has_update
                    ? `发现新版本 v${updateInfo.latest_version}，点击前往更新`
                    : "查看版本与更新"
                }
              >
                {updateInfo?.has_update && (
                  <span className="version-pulse-dot" aria-hidden="true" />
                )}
                <span>v{currentVersionLabel}</span>
              </button>
              {state?.is_mock && <span className="mock-pill">模拟</span>}
            </div>
            <p className="brand-sub">UEFI 多系统精密引导切换</p>
          </div>
        </div>

        {/* Center: Segmented Pill Switcher */}
        <nav className="segmented-nav" aria-label="主功能切换">
          <button
            type="button"
            className={`seg-btn ${activeTab === "boot" ? "is-active" : ""}`}
            onClick={() => setActiveTab("boot")}
          >
            引导切换
          </button>
          <button
            type="button"
            className={`seg-btn ${activeTab === "update" ? "is-active" : ""}`}
            onClick={handleOpenUpdateTab}
          >
            <span>版本更新</span>
            {updateInfo?.has_update && (
              <span className="seg-update-dot" aria-hidden="true" />
            )}
          </button>
        </nav>

        {/* Right: Appearance Pill Switch + Refresh Pill Button */}
        <div className="topbar-right">
          <div
            className="appearance-switch"
            role="group"
            aria-label="界面外观主题切换"
          >
            <button
              type="button"
              className={`appearance-btn ${theme === "light" ? "is-active" : ""}`}
              onClick={() => setTheme("light")}
              title="浅色 Bento 主题"
              aria-label="浅色主题"
              aria-pressed={theme === "light"}
            >
              <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2.5v2.2M12 19.3v2.2M4.9 4.9l1.6 1.6M17.5 17.5l1.6 1.6M2.5 12h2.2M19.3 12h2.2M4.9 19.1l1.6-1.6M17.5 6.5l1.6-1.6" />
              </svg>
            </button>
            <button
              type="button"
              className={`appearance-btn ${theme === "dark" ? "is-active" : ""}`}
              onClick={() => setTheme("dark")}
              title="深色 Obsidian 主题"
              aria-label="深色主题"
              aria-pressed={theme === "dark"}
            >
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" strokeWidth="2.1" strokeLinecap="round" strokeLinejoin="round">
                <path d="M20.5 14.5A8.5 8.5 0 1 1 9.5 3.5a6.8 6.8 0 0 0 11 11Z" />
              </svg>
            </button>
          </div>

          <button
            type="button"
            className="pill-btn pill-btn-surface"
            disabled={activeTab === "boot" ? loading || busy : checkingUpdate || installingUpdate}
            onClick={() =>
              activeTab === "boot"
                ? void refreshState()
                : void runUpdateCheck(false)
            }
          >
            <svg
              viewBox="0 0 24 24"
              width="14"
              height="14"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.1"
              strokeLinecap="round"
              strokeLinejoin="round"
              className={
                (activeTab === "boot" && loading) ||
                (activeTab === "update" && checkingUpdate)
                  ? "icon-spin"
                  : ""
              }
              aria-hidden="true"
            >
              <path d="M20 12a8 8 0 1 1-2.3-5.6L20 8.5" />
              <path d="M20 3.5v5h-5" />
            </svg>
            <span>
              {activeTab === "boot"
                ? loading
                  ? "读取中"
                  : "刷新"
                : checkingUpdate
                  ? "检查中"
                  : "检查"}
            </span>
          </button>
        </div>
      </header>

      {/* Main Content */}
      <main className="main-content">
        {activeTab === "update" ? (
          <UpdatePanel
            appVersion={APP_VERSION}
            updateInfo={updateInfo}
            checking={checkingUpdate}
            installing={installingUpdate}
            progress={updateProgress}
            error={updateError}
            autoCheck={autoCheckUpdate}
            onToggleAutoCheck={setAutoCheckUpdate}
            onCheckNow={() => void runUpdateCheck(false)}
            onDownloadAndInstall={() => void handleDownloadAndInstall()}
            onOpenReleasePage={(url) => void handleOpenReleasePage(url)}
          />
        ) : (
          <>
            {loading && !state && (
              <div className="empty-bento">正在读取主板 UEFI 引导信息...</div>
            )}

            {loadError && (
              <section className="notice-bento notice-bento-error" role="alert">
                <div className="notice-badge-icon" aria-hidden="true">
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
                      className="pill-btn pill-btn-cobalt"
                      onClick={() => void handleRelaunchAdmin()}
                    >
                      以管理员身份重新打开
                    </button>
                  )}
                  <button
                    type="button"
                    className="pill-btn pill-btn-secondary"
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
                  showAll={showAll}
                  hiddenCount={hiddenCount}
                  updateInfo={updateInfo}
                  checkingUpdate={checkingUpdate}
                  appVersion={APP_VERSION}
                  onToggleShowAll={handleToggleShowAll}
                  onClearNext={() => void handleClearNext()}
                  onQuickReboot={() =>
                    setModalReq({ entry: null, initialMode: "next" })
                  }
                  onOpenUpdateTab={handleOpenUpdateTab}
                />

                <section className="entries-section" aria-label="UEFI 启动项列表">
                  <div className="entries-toolbar">
                    <div className="search-pill-wrap">
                      <svg
                        className="search-pill-icon"
                        viewBox="0 0 24 24"
                        width="15"
                        height="15"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2.1"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        aria-hidden="true"
                      >
                        <circle cx="11" cy="11" r="7" />
                        <path d="m20 20-3.5-3.5" />
                      </svg>
                      <input
                        type="search"
                        className="search-pill-input"
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        placeholder="搜索系统、发行版或 Boot 编号..."
                        aria-label="搜索系统、发行版或 Boot 编号"
                      />
                      {searchQuery && (
                        <button
                          type="button"
                          className="search-clear-btn"
                          aria-label="清空搜索"
                          onClick={() => setSearchQuery("")}
                        >
                          ×
                        </button>
                      )}
                    </div>

                    <div className="filter-pills" role="group" aria-label="启动项分类筛选">
                      <button
                        type="button"
                        className={`filter-pill ${filter === "all" ? "is-active" : ""}`}
                        onClick={() => handleSelectFilter("all")}
                      >
                        全部 <span className="filter-count">{entryCounts.all}</span>
                      </button>
                      <button
                        type="button"
                        className={`filter-pill ${filter === "os" ? "is-active" : ""}`}
                        onClick={() => handleSelectFilter("os")}
                      >
                        操作系统 <span className="filter-count">{entryCounts.os}</span>
                      </button>
                      <button
                        type="button"
                        className={`filter-pill ${filter === "advanced" ? "is-active" : ""}`}
                        onClick={() => handleSelectFilter("advanced")}
                      >
                        高级项 <span className="filter-count">{entryCounts.advanced}</span>
                      </button>
                    </div>
                  </div>

                  {visibleEntries.length === 0 ? (
                    <div className="empty-bento">
                      {searchQuery
                        ? `未找到与「${searchQuery}」匹配的 UEFI 启动项，请尝试其他关键字或切换至「全部」。`
                        : "当前分类下暂无启动项，请开启上方「显示全部 UEFI 项（高级）」查看完整列表。"}
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
          </>
        )}
      </main>

      <footer className="footer-bar">
        <span>
          提示：「仅下次生效」重启一次后自动恢复默认；如担心系统自动更新多次重启掉回原系统，可使用「设为长期默认」。
        </span>
      </footer>

      {toast && (
        <div
          className={`toast ${toast.kind === "error" ? "toast-error" : "toast-success"}`}
          role="status"
        >
          <span className="toast-dot" aria-hidden="true" />
          <span className="toast-text">{toast.text}</span>
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
