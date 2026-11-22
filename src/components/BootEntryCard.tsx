import type { BootEntry, SwitchMode } from "../types";
import { getFallbackOsLabel, OsIcon } from "./OsIcon";

interface BootEntryCardProps {
  entry: BootEntry;
  busy: boolean;
  onApplyOnly: (entry: BootEntry, mode: SwitchMode) => void;
  onRequestReboot: (entry: BootEntry, defaultMode: SwitchMode) => void;
}

export function BootEntryCard({
  entry,
  busy,
  onApplyOnly,
  onRequestReboot,
}: BootEntryCardProps) {
  const osLabel = entry.os_label?.trim() || getFallbackOsLabel(entry.os_kind);
  const showOsLabelBadge =
    Boolean(osLabel) &&
    osLabel.toLowerCase() !== entry.display_name.trim().toLowerCase();

  return (
    <article
      className={`entry-card ${entry.is_next ? "entry-card-next" : ""} ${
        entry.is_current ? "entry-card-current" : ""
      } ${!entry.is_primary_os ? "entry-card-advanced" : ""}`}
    >
      <div className="entry-main">
        <OsIcon kind={entry.os_kind} size="md" />

        <div className="entry-info">
          <div className="entry-title-row">
            <h3 className="entry-title">{entry.display_name}</h3>
            <span className="mono-pill">Boot{entry.id_hex}</span>
            {entry.is_current && (
              <span className="sem-pill sem-pill-current">
                <span className="sem-dot" aria-hidden="true" />
                当前运行
              </span>
            )}
            {entry.is_default && (
              <span className="sem-pill sem-pill-default">长期默认</span>
            )}
            {entry.is_next && (
              <span className="sem-pill sem-pill-next">下次重启</span>
            )}
            {showOsLabelBadge && (
              <span className="sem-pill sem-pill-label">{osLabel}</span>
            )}
            {!entry.is_primary_os && (
              <span className="sem-pill sem-pill-advanced">高级项</span>
            )}
          </div>

          <div className="entry-meta">
            {entry.efi_path ? (
              <span className="meta-efi-path" title={entry.efi_path}>
                {entry.efi_path}
              </span>
            ) : (
              <span className="meta-plain">设备级 UEFI 引导项（无独立 EFI 路径）</span>
            )}
            {entry.description &&
              entry.description.toLowerCase() !==
                entry.display_name.trim().toLowerCase() && (
                <span className="meta-origin" title={entry.description}>
                  原始: {entry.description}
                </span>
              )}
          </div>

          {entry.risk_warning && (
            <div className="risk-callout" role="note">
              <svg
                viewBox="0 0 24 24"
                width="14"
                height="14"
                fill="none"
                stroke="currentColor"
                strokeWidth="2.1"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                <path d="M12 9v4m0 4h.01M10.3 3.9 2.6 17.5a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
              </svg>
              <span>{entry.risk_warning}</span>
            </div>
          )}
        </div>
      </div>

      <div className="entry-actions">
        <button
          type="button"
          className={`pill-btn pill-btn-secondary ${entry.is_next ? "pill-btn-checked" : ""}`}
          disabled={busy || entry.is_next}
          onClick={() => onApplyOnly(entry, "next")}
          title="写入 UEFI BootNext，仅对下一次重启生效，不立即重启"
        >
          {entry.is_next ? "已指定下次" : "仅下次生效"}
        </button>

        <button
          type="button"
          className="pill-btn pill-btn-secondary"
          disabled={busy || entry.is_default}
          onClick={() => onApplyOnly(entry, "default")}
          title="修改 UEFI BootOrder 将此系统置顶为长期默认，防止多次自动重启掉回其他系统"
        >
          {entry.is_default ? "已是默认" : "设为长期默认"}
        </button>

        <button
          type="button"
          className="pill-btn pill-btn-ink"
          disabled={busy}
          onClick={() => onRequestReboot(entry, "next")}
        >
          <svg
            viewBox="0 0 24 24"
            width="13"
            height="13"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.3"
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden="true"
          >
            <path d="M12 2v10m5.6-6.6a8 8 0 1 1-11.2 0" />
          </svg>
          切换并重启
        </button>
      </div>
    </article>
  );
}
