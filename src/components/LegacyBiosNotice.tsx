import type { FirmwareType } from "../types";

interface LegacyBiosNoticeProps {
  firmware: FirmwareType;
  hostOs: string;
}

export function LegacyBiosNotice({ firmware, hostOs }: LegacyBiosNoticeProps) {
  return (
    <section className="notice-bento" role="alert">
      <div className="notice-badge-icon" aria-hidden="true">
        <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
          <path d="M12 9v4m0 4h.01M10.3 3.9 2.6 17.5a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
        </svg>
      </div>
      <h2>当前电脑未运行在 UEFI 引导模式</h2>
      <p>
        检测到主板固件模式为{" "}
        <code>{firmware === "legacy_bios" ? "Legacy BIOS (MBR)" : "未知固件"}</code>
        （当前系统：{hostOs === "windows" ? "Windows" : "Linux"}）。
      </p>
      <p className="notice-sub">
        NextBoot 通过直接读写主板 UEFI NVRAM 来实现 Windows 与 Linux 多系统零侵入安全切换，因此当前暂不支持传统 Legacy BIOS 机器。
      </p>
      <div className="notice-tips-box">
        <div className="notice-tips-title">排查建议</div>
        <ul>
          <li>
            Windows 下可按 Win+R 输入 <code>msinfo32</code>，查看「BIOS 模式」是否为「传统」。
          </li>
          <li>
            Linux 下可检查 <code>/sys/firmware/efi</code> 目录是否存在。
          </li>
          <li>
            如主板支持 UEFI，建议使用 GPT 分区与 UEFI 引导安装多系统后再启用 NextBoot。
          </li>
        </ul>
      </div>
    </section>
  );
}
