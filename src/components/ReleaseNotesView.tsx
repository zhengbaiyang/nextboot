import type { ReactNode } from "react";

interface ReleaseNotesViewProps {
  markdown: string;
  onOpenLink: (url: string) => void;
}

type Alignment = "left" | "center" | "right";

type Block =
  | { type: "heading"; level: number; text: string }
  | { type: "paragraph"; text: string; isFullChangelog?: boolean }
  | { type: "ul"; items: string[] }
  | { type: "ol"; items: string[] }
  | { type: "blockquote"; lines: string[] }
  | { type: "table"; headers: string[]; alignments: Alignment[]; rows: string[][] }
  | { type: "code"; content: string }
  | { type: "hr" };

type SkipSectionKind = "demo" | "download";

const DEMO_SECTION_PATTERNS = [/界面演示与截图/i, /界面演示/i, /功能界面截图/i];
const DOWNLOAD_SECTION_PATTERNS = [/下载与安装指南/i, /下载与安装/i];
const FULL_CHANGELOG_RE = /^\*{0,2}Full Changelog\*{0,2}\s*:/i;
const OFFICIAL_GITHUB_URL_RE = /^https:\/\/github\.com\/zhengbaiyang\/nextboot(?:$|[/?#])/;

function isOfficialGitHubUrl(url: string): boolean {
  return OFFICIAL_GITHUB_URL_RE.test(url.trim());
}

function getSkippedSectionKind(headingText: string): SkipSectionKind | null {
  const normalized = headingText.replace(/[*_`~]/g, "").trim();
  if (DEMO_SECTION_PATTERNS.some((re) => re.test(normalized))) return "demo";
  if (DOWNLOAD_SECTION_PATTERNS.some((re) => re.test(normalized))) return "download";
  return null;
}

function protectCodeSpans(text: string, transform: (plain: string) => string): string {
  const parts = text.split(/(`[^`\n]+`)/g);
  return parts.map((part, idx) => (idx % 2 === 1 ? part : transform(part))).join("");
}

function stripRawHtml(raw: string): string {
  const normalized = raw.replace(/\r\n?/g, "\n");
  const segments = normalized.split(/(^```[\s\S]*?^```[ \t]*$)/gm);

  return segments
    .map((segment, idx) => {
      if (idx % 2 === 1) return segment;

      let s = segment;
      // Remove HTML comments and non-text containers
      s = s.replace(/<!--[\s\S]*?-->/g, "");
      s = s.replace(/<(script|style|svg|picture)\b[^>]*>[\s\S]*?<\/\1>/gi, "");
      // Drop HTML tables that embed images (e.g., screenshot galleries)
      s = s.replace(/<table\b[^>]*>[\s\S]*?<\/table>/gi, (tableHtml) =>
        /<img\b/i.test(tableHtml) ? "" : tableHtml
      );
      // Remove standalone <img> tags
      s = s.replace(/<img\b[^>]*\/?>/gi, "");

      // Convert useful inline HTML tags to Markdown, then strip all remaining tags outside code spans
      s = protectCodeSpans(s, (plain) =>
        plain
          .replace(
            /<a\b[^>]*href=["'](https?:\/\/[^"']+)["'][^>]*>([\s\S]*?)<\/a>/gi,
            (_, href: string, inner: string) => {
              const cleanLabel = inner.replace(/<[^>]+>/g, "").trim();
              return cleanLabel ? `[${cleanLabel}](${href})` : href;
            }
          )
          .replace(/<(strong|b)\b[^>]*>([\s\S]*?)<\/\1>/gi, (_, __, inner: string) => {
            const trimmed = inner.trim();
            return trimmed ? `**${trimmed}**` : "";
          })
          .replace(/<code\b[^>]*>([\s\S]*?)<\/code>/gi, (_, inner: string) => {
            const trimmed = inner.trim();
            return trimmed ? `\`${trimmed}\`` : "";
          })
          .replace(/<br\s*\/?>/gi, "\n")
          .replace(/<\/(?:p|div|section|article|header|footer|h[1-6]|li|tr|blockquote)>/gi, "\n")
          .replace(/<hr\s*\/?>/gi, "\n---\n")
          .replace(/<\/?[a-zA-Z][a-zA-Z0-9-]*(?:\s+[^<>]*)?\/?>/g, "")
          .replace(/&nbsp;/g, " ")
          .replace(/&amp;/g, "&")
          .replace(/&lt;/g, "<")
          .replace(/&gt;/g, ">")
          .replace(/&middot;/g, "·")
          .replace(/&mdash;/g, "—")
          .replace(/&ndash;/g, "–")
          .replace(/&quot;/g, '"')
          .replace(/&#39;/g, "'")
      );

      return s;
    })
    .join("");
}

function isImageOnlyText(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed.includes("![")) return false;
  const withoutImages = trimmed
    .replace(/\[!\[[^\]]*\]\([^)]+\)\]\([^)]+\)/g, "")
    .replace(/!\[[^\]]*\]\([^)]+\)/g, "")
    .trim();
  return withoutImages.length === 0;
}

function isTableSeparatorLine(line: string): boolean {
  const trimmed = line.trim();
  if (!trimmed.includes("-") || !trimmed.includes("|")) return false;
  return /^\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)+\|?$/.test(trimmed) ||
    /^\|\s*:?-{2,}:?\s*\|$/.test(trimmed);
}

function splitTableRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|")) s = s.slice(0, -1);

  const cells: string[] = [];
  let current = "";
  let inCode = false;

  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (ch === "\\" && i + 1 < s.length && s[i + 1] === "|") {
      current += "|";
      i++;
      continue;
    }
    if (ch === "`") {
      inCode = !inCode;
      current += ch;
      continue;
    }
    if (ch === "|" && !inCode) {
      cells.push(current.trim());
      current = "";
      continue;
    }
    current += ch;
  }
  cells.push(current.trim());
  return cells;
}

function parseAlignments(separatorLine: string, count: number): Alignment[] {
  const rawCells = splitTableRow(separatorLine);
  const result: Alignment[] = [];
  for (let i = 0; i < count; i++) {
    const cell = (rawCells[i] ?? "").trim();
    const startsColon = cell.startsWith(":");
    const endsColon = cell.endsWith(":");
    if (startsColon && endsColon) {
      result.push("center");
    } else if (endsColon) {
      result.push("right");
    } else {
      result.push("left");
    }
  }
  return result;
}

function isHrLine(trimmed: string): boolean {
  return /^(-{3,}|\*{3,}|_{3,})$/.test(trimmed);
}

function isBlockStarter(lines: string[], idx: number): boolean {
  const trimmed = lines[idx].trim();
  if (!trimmed) return true;
  if (/^#{1,6}\s+/.test(trimmed)) return true;
  if (/^```/.test(trimmed)) return true;
  if (isHrLine(trimmed)) return true;
  if (/^>\s?/.test(trimmed)) return true;
  if (/^[-*+]\s+/.test(trimmed)) return true;
  if (/^\d+\.\s+/.test(trimmed)) return true;
  if (FULL_CHANGELOG_RE.test(trimmed)) return true;
  if (trimmed.includes("|") && idx + 1 < lines.length && isTableSeparatorLine(lines[idx + 1])) {
    return true;
  }
  return false;
}

export function parseReleaseNotes(markdown: string): Block[] {
  if (!markdown.trim()) return [];

  const cleaned = stripRawHtml(markdown);
  const lines = cleaned.split("\n");
  const blocks: Block[] = [];

  let skipSection: { level: number; kind: SkipSectionKind } | null = null;
  let i = 0;

  while (i < lines.length) {
    const rawLine = lines[i];
    const trimmed = rawLine.trim();

    if (!trimmed) {
      i++;
      continue;
    }

    // Handle active skipped web-only section (e.g., 界面演示与截图 / 下载与安装指南)
    if (skipSection) {
      const headingMatch = trimmed.match(/^(#{1,6})\s+(.+)$/);
      if (headingMatch) {
        const level = headingMatch[1].length;
        const headingText = headingMatch[2].replace(/\s+#+$/, "").trim();
        if (level <= skipSection.level) {
          const nextSkipKind = getSkippedSectionKind(headingText);
          if (nextSkipKind) {
            skipSection = { level, kind: nextSkipKind };
            i++;
            continue;
          }
          skipSection = null;
          // Fall through to parse this heading
        } else {
          i++;
          continue;
        }
      } else if (isHrLine(trimmed)) {
        skipSection = null;
        i++;
        continue;
      } else if (FULL_CHANGELOG_RE.test(trimmed)) {
        skipSection = null;
        // Fall through to parse Full Changelog
      } else if (skipSection.kind === "download" && /^>\s?/.test(trimmed)) {
        // Preserve trailing summary/README blockquote after download table
        skipSection = null;
        // Fall through to parse blockquote
      } else {
        i++;
        continue;
      }
    }

    // Skip standalone Markdown images or badge strips
    if (isImageOnlyText(trimmed)) {
      i++;
      continue;
    }

    // Headings
    const headingMatch = trimmed.match(/^(#{1,6})\s+(.+)$/);
    if (headingMatch) {
      const level = headingMatch[1].length;
      const text = headingMatch[2].replace(/\s+#+$/, "").trim();
      const skipKind = getSkippedSectionKind(text);
      if (skipKind) {
        skipSection = { level, kind: skipKind };
      } else if (text && !isImageOnlyText(text)) {
        blocks.push({ type: "heading", level, text });
      }
      i++;
      continue;
    }

    // Fenced code block
    if (/^```/.test(trimmed)) {
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !/^```\s*$/.test(lines[i].trim())) {
        codeLines.push(lines[i]);
        i++;
      }
      if (i < lines.length) i++; // skip closing fence
      const content = codeLines.join("\n").trimEnd();
      if (content) {
        blocks.push({ type: "code", content });
      }
      continue;
    }

    // Horizontal rule (avoid leading or consecutive dividers)
    if (isHrLine(trimmed)) {
      if (blocks.length > 0 && blocks[blocks.length - 1].type !== "hr") {
        blocks.push({ type: "hr" });
      }
      i++;
      continue;
    }

    // Markdown table
    if (
      trimmed.includes("|") &&
      i + 1 < lines.length &&
      isTableSeparatorLine(lines[i + 1])
    ) {
      const headers = splitTableRow(lines[i]);
      const alignments = parseAlignments(lines[i + 1], headers.length);
      const rawRows: string[][] = [];
      i += 2;

      while (i < lines.length) {
        const rowLine = lines[i].trim();
        if (!rowLine || !rowLine.includes("|") || isTableSeparatorLine(rowLine)) {
          break;
        }
        const cells = splitTableRow(rowLine);
        while (cells.length < headers.length) cells.push("");
        rawRows.push(cells.slice(0, headers.length));
        i++;
      }

      const textRows = rawRows.filter((row) => {
        const nonEmpty = row.filter((cell) => cell.trim().length > 0);
        if (nonEmpty.length === 0) return false;
        const hasImage = nonEmpty.some((cell) => cell.includes("!["));
        if (hasImage && nonEmpty.every((cell) => isImageOnlyText(cell))) {
          return false;
        }
        return true;
      });

      const headerIsImageOnly =
        headers.some((h) => h.includes("![")) &&
        headers.every((h) => !h.trim() || isImageOnlyText(h));

      if (!headerIsImageOnly && textRows.length > 0) {
        blocks.push({ type: "table", headers, alignments, rows: textRows });
      }
      continue;
    }

    // Blockquote
    if (/^>\s?/.test(trimmed)) {
      const quoteLines: string[] = [];
      while (i < lines.length && /^>\s?/.test(lines[i].trim())) {
        const lineContent = lines[i].trim().replace(/^>\s?/, "").trim();
        if (lineContent && !isImageOnlyText(lineContent)) {
          quoteLines.push(lineContent);
        }
        i++;
      }
      if (quoteLines.length > 0) {
        blocks.push({ type: "blockquote", lines: quoteLines });
      }
      continue;
    }

    // Unordered list
    if (/^[-*+]\s+/.test(trimmed)) {
      const items: string[] = [];
      while (i < lines.length) {
        const currentTrimmed = lines[i].trim();
        const bulletMatch = currentTrimmed.match(/^[-*+]\s+(.+)$/);
        if (bulletMatch) {
          const itemText = bulletMatch[1].trim();
          if (itemText && !isImageOnlyText(itemText)) {
            items.push(itemText);
          }
          i++;
          continue;
        }
        // Handle indented continuation lines inside a list item
        if (/^\s{2,}\S/.test(lines[i]) && items.length > 0 && !isBlockStarter(lines, i)) {
          items[items.length - 1] += ` ${currentTrimmed}`;
          i++;
          continue;
        }
        break;
      }
      if (items.length > 0) {
        blocks.push({ type: "ul", items });
      }
      continue;
    }

    // Ordered list
    if (/^\d+\.\s+/.test(trimmed)) {
      const items: string[] = [];
      while (i < lines.length) {
        const currentTrimmed = lines[i].trim();
        const numMatch = currentTrimmed.match(/^\d+\.\s+(.+)$/);
        if (numMatch) {
          const itemText = numMatch[1].trim();
          if (itemText && !isImageOnlyText(itemText)) {
            items.push(itemText);
          }
          i++;
          continue;
        }
        if (/^\s{2,}\S/.test(lines[i]) && items.length > 0 && !isBlockStarter(lines, i)) {
          items[items.length - 1] += ` ${currentTrimmed}`;
          i++;
          continue;
        }
        break;
      }
      if (items.length > 0) {
        blocks.push({ type: "ol", items });
      }
      continue;
    }

    // Paragraph (group consecutive plain lines)
    const paraLines: string[] = [];
    while (i < lines.length) {
      const currentTrimmed = lines[i].trim();
      if (!currentTrimmed) break;
      if (paraLines.length > 0 && isBlockStarter(lines, i)) break;
      if (!isImageOnlyText(currentTrimmed)) {
        paraLines.push(currentTrimmed);
      }
      i++;
      if (paraLines.length === 1 && FULL_CHANGELOG_RE.test(paraLines[0])) break;
    }
    const paraText = paraLines.join(" ").trim();
    if (paraText) {
      blocks.push({
        type: "paragraph",
        text: paraText,
        isFullChangelog: FULL_CHANGELOG_RE.test(paraText),
      });
    }
  }

  // Remove any trailing horizontal rule
  while (blocks.length > 0 && blocks[blocks.length - 1].type === "hr") {
    blocks.pop();
  }

  return blocks;
}

function ExternalLinkIcon() {
  return (
    <svg
      className="rn-link-icon"
      viewBox="0 0 24 24"
      width="12"
      height="12"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.1"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M14 5h5v5" />
      <path d="M10 14 19 5" />
      <path d="M19 13v5a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5" />
    </svg>
  );
}

const INLINE_TOKEN_PATTERN =
  /\[!\[[^\]]*\]\([^)]+\)\]\([^)]+\)|!\[[^\]]*\]\([^)]+\)|`([^`\n]+)`|\[([^\]]+)\]\(([^)\s]+)\)|\*\*([^*]+(?:\*(?!\*)[^*]*)*)\*\*|(https?:\/\/[^\s<>()[\]]+)/;

const TRAILING_URL_PUNCT_RE = /[.,;:!?。，；：！？）】》」>]+$/;

function renderInline(
  text: string,
  onOpenLink: (url: string) => void,
  keyPrefix: string,
  allowLinks = true
): ReactNode[] {
  const nodes: ReactNode[] = [];
  let lastIndex = 0;
  let tokenIdx = 0;

  const tokenRe = new RegExp(INLINE_TOKEN_PATTERN.source, "g");
  let match: RegExpExecArray | null;

  while ((match = tokenRe.exec(text)) !== null) {
    if (match.index > lastIndex) {
      nodes.push(text.slice(lastIndex, match.index));
    }

    const full = match[0];
    const codeInner = match[1];
    const linkLabel = match[2];
    const linkUrl = match[3];
    const boldInner = match[4];
    const bareUrl = match[5];
    const key = `${keyPrefix}-${tokenIdx++}`;

    if (full.startsWith("![") || full.startsWith("[![")) {
      // Omit inline images
    } else if (codeInner !== undefined) {
      nodes.push(
        <code key={key} className="rn-inline-code">
          {codeInner}
        </code>
      );
    } else if (linkLabel !== undefined && linkUrl !== undefined) {
      if (allowLinks && isOfficialGitHubUrl(linkUrl)) {
        nodes.push(
          <button
            key={key}
            type="button"
            className="rn-link"
            title={linkUrl}
            onClick={() => onOpenLink(linkUrl)}
          >
            <span>{renderInline(linkLabel, onOpenLink, `${key}-l`, false)}</span>
            <ExternalLinkIcon />
          </button>
        );
      } else {
        nodes.push(
          <span key={key} className="rn-link-static" title={linkUrl}>
            {renderInline(linkLabel, onOpenLink, `${key}-l`, false)}
          </span>
        );
      }
    } else if (boldInner !== undefined) {
      nodes.push(
        <strong key={key} className="rn-strong">
          {renderInline(boldInner, onOpenLink, `${key}-b`, allowLinks)}
        </strong>
      );
    } else if (bareUrl !== undefined) {
      const cleanUrl = bareUrl.replace(TRAILING_URL_PUNCT_RE, "");
      const trailing = bareUrl.slice(cleanUrl.length);
      if (allowLinks && cleanUrl && isOfficialGitHubUrl(cleanUrl)) {
        nodes.push(
          <button
            key={key}
            type="button"
            className="rn-link rn-link-url"
            title={cleanUrl}
            onClick={() => onOpenLink(cleanUrl)}
          >
            <span>{cleanUrl}</span>
            <ExternalLinkIcon />
          </button>
        );
      } else {
        nodes.push(cleanUrl);
      }
      if (trailing) {
        nodes.push(trailing);
      }
    }

    lastIndex = match.index + full.length;
  }

  if (lastIndex < text.length) {
    nodes.push(text.slice(lastIndex));
  }

  return nodes;
}

export function ReleaseNotesView({ markdown, onOpenLink }: ReleaseNotesViewProps) {
  const blocks = parseReleaseNotes(markdown);

  if (blocks.length === 0) {
    return (
      <p className="notes-empty">
        此版本暂无详细更新日志，可前往 GitHub Release 页面查看完整变更。
      </p>
    );
  }

  return (
    <div className="rn-content">
      {blocks.map((block, idx) => {
        const key = `rn-${idx}`;
        switch (block.type) {
          case "heading": {
            const headingClass =
              block.level <= 2
                ? "rn-h2"
                : block.level === 3
                  ? "rn-h3"
                  : "rn-h4";
            return (
              <div key={key} className={headingClass} role="heading" aria-level={Math.min(6, Math.max(2, block.level + 1))}>
                {renderInline(block.text, onOpenLink, key)}
              </div>
            );
          }

          case "paragraph":
            return (
              <p
                key={key}
                className={block.isFullChangelog ? "rn-p rn-changelog-row" : "rn-p"}
              >
                {renderInline(block.text, onOpenLink, key)}
              </p>
            );

          case "ul":
            return (
              <ul key={key} className="rn-ul">
                {block.items.map((item, itemIdx) => (
                  <li key={`${key}-li-${itemIdx}`} className="rn-li">
                    <span className="rn-bullet-dot" aria-hidden="true" />
                    <span className="rn-li-body">
                      {renderInline(item, onOpenLink, `${key}-li-${itemIdx}`)}
                    </span>
                  </li>
                ))}
              </ul>
            );

          case "ol":
            return (
              <ol key={key} className="rn-ol">
                {block.items.map((item, itemIdx) => (
                  <li key={`${key}-oi-${itemIdx}`} className="rn-ol-li">
                    <span className="rn-ol-num" aria-hidden="true">
                      {itemIdx + 1}
                    </span>
                    <span className="rn-li-body">
                      {renderInline(item, onOpenLink, `${key}-oi-${itemIdx}`)}
                    </span>
                  </li>
                ))}
              </ol>
            );

          case "blockquote":
            return (
              <blockquote key={key} className="rn-blockquote">
                {block.lines.map((line, lineIdx) => (
                  <p key={`${key}-q-${lineIdx}`} className="rn-blockquote-p">
                    {renderInline(line, onOpenLink, `${key}-q-${lineIdx}`)}
                  </p>
                ))}
              </blockquote>
            );

          case "table":
            return (
              <div key={key} className="rn-table-wrap">
                <table className="rn-table">
                  <thead>
                    <tr>
                      {block.headers.map((header, colIdx) => (
                        <th
                          key={`${key}-th-${colIdx}`}
                          style={{ textAlign: block.alignments[colIdx] ?? "left" }}
                        >
                          {renderInline(header, onOpenLink, `${key}-th-${colIdx}`)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {block.rows.map((row, rowIdx) => (
                      <tr key={`${key}-tr-${rowIdx}`}>
                        {row.map((cell, colIdx) => (
                          <td
                            key={`${key}-td-${rowIdx}-${colIdx}`}
                            style={{ textAlign: block.alignments[colIdx] ?? "left" }}
                          >
                            {renderInline(
                              cell,
                              onOpenLink,
                              `${key}-td-${rowIdx}-${colIdx}`
                            )}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            );

          case "code":
            return (
              <pre key={key} className="rn-code-block">
                <code>{block.content}</code>
              </pre>
            );

          case "hr":
            return <hr key={key} className="rn-hr" />;
        }
      })}
    </div>
  );
}
