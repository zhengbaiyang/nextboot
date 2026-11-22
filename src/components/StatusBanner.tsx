import type { BootEntry, BootState } from "../types";

interface StatusBannerProps {
  state: BootState;
  busy: boolean;
  onClearNext: () => void;
  onQuickReboot: () => void;
}

function resolveEntry(state: BootState, idHex: string | null): BootEntry | null {
  if (!idHex) return null;
  return state.entries.find((e) => e.id_hex === idHex) ?? null;
}

export function StatusBanner({
  state,
  busy,
  onClearNext,
  onQuickReboot,
}: StatusBannerProps) {
  const currentEntry = resolveEntry(state, state.boot_current);
  const defaultEntry = resolveEntry(state, state.default_boot);
  const nextEntry = resolveEntry(state, state.boot_next);

  return (
    <section className="status-grid" aria-label="当前引导状态概览">
      <div className="status-card">
        <div className="status-label">当前正在运行 (BootCurrent)</div>
        <div className="status-value">
          {currentEntry ? currentEntry.display_name : state.boot_current ? `Boot${state.boot_current}` : "未知"}
        </div>
        <div className="status-sub">
          {state.boot_current ? `Boot${state.boot_current}` : "未读取到当前项"}
        </div>
      </div>

      <div className="status-card">
        <div className="status-label">长期默认系统 (BootOrder #1)</div>
        <div className="status-value">
          {defaultEntry ? defaultEntry.display_name : state.default_boot ? `Boot${state.default_boot}` : "未设置"}
        </div>
        <div className="status-sub">
          {state.default_boot ? `Boot${state.default_boot} · 自动重启默认进入` : "无默认顺序"}
        </div>
      </div>

      <div className={`status-card ${nextEntry ? "status-card-highlight" : ""}`}>
        <div className="status-label">下一次重启指定 (BootNext)</div>
        <div className="status-value">
          {nextEntry
            ? nextEntry.display_name
            : state.boot_next
              ? `Boot${state.boot_next}`
              : "未指定（跟随默认）"}
        </div>
        <div className="status-actions-row">
          {state.boot_next ? (
            <>
              <button
                type="button"
                className="btn-link"
                disabled={busy}
                onClick={onClearNext}
              >
                取消指定
              </button>
              <button
                type="button"
                className="btn-link btn-link-accent"
                disabled={busy}
                onClick={onQuickReboot}
              >
                立即重启生效
              </button>
            </>
          ) : (
            <span className="status-sub">重启一次后自动清除</span>
          )}
        </div>
      </div>
    </section>
  );
}
