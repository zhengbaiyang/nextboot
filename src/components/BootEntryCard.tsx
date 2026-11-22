import type { BootEntry, SwitchMode } from "../types";
import { OsIcon } from "./OsIcon";

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
  return (
    <article
      className={`entry-card ${entry.is_next ? "entry-card-next" : ""} ${
        !entry.is_primary_os ? "entry-card-advanced" : ""
      }`}
    >
      <div className="entry-main">
        <OsIcon kind={entry.os_kind} />

        <div className="entry-info">
          <div className="entry-title-row">
            <h3 className="entry-title">{entry.display_name}</h3>
            <span className="boot-id-chip">Boot{entry.id_hex}</span>
            {entry.is_current && <span className="badge badge-current">当前运行</span>}
            {entry.is_default && <span className="badge badge-default">长期默认</span>}
            {entry.is_next && <span className="badge badge-next">下次重启目标</span>}
            {!entry.is_primary_os && <span className="badge badge-advanced">高级 / 非系统</span>}
          </div>

          <div className="entry-meta">
            {entry.description && entry.description !== entry.display_name && (
              <span className="meta-item">原始名称: {entry.description}</span>
            )}
            {entry.efi_path ? (
              <span className="meta-path" title={entry.efi_path}>
                {entry.efi_path}
              </span>
            ) : (
              <span className="meta-item">无独立 EFI 文件路径（设备级引导项）</span>
            )}
          </div>

          {entry.risk_warning && (
            <div className="risk-warning" role="note">
              ⚠ {entry.risk_warning}
            </div>
          )}
        </div>
      </div>

      <div className="entry-actions">
        <button
          type="button"
          className="btn btn-secondary"
          disabled={busy || entry.is_next}
          onClick={() => onApplyOnly(entry, "next")}
          title="写入 UEFI BootNext，仅对下一次重启生效，不立即重启"
        >
          {entry.is_next ? "已指定下次" : "仅下次生效"}
        </button>

        <button
          type="button"
          className="btn btn-secondary"
          disabled={busy || entry.is_default}
          onClick={() => onApplyOnly(entry, "default")}
          title="修改 UEFI BootOrder 将此系统置顶为长期默认，防止多次自动重启掉回其他系统"
        >
          {entry.is_default ? "已是默认" : "设为长期默认"}
        </button>

        <button
          type="button"
          className="btn btn-primary"
          disabled={busy}
          onClick={() => onRequestReboot(entry, "next")}
        >
          切换并重启
        </button>
      </div>
    </article>
  );
}
