import { useT } from "../i18n";

// テキストクリップの表示。最大 maxLines 行までで省略(…)。全文は詳細メニューの「表示」から。
// 1 行のときは改行を薄い ↵ に置換して 1 行に収める。2 行以上のときは実際の改行で
// 折り返し、CSS の line-clamp で行数を制限する。
export function ClipText({
  content,
  onPaste,
  maxLines = 1,
}: {
  content: string;
  onPaste: () => void;
  maxLines?: number;
}) {
  const t = useT();
  const buttonStyle = {
    border: "none",
    padding: 0,
    background: "none",
    color: "inherit",
    font: "inherit",
    textAlign: "left" as const,
    width: "100%",
  };

  // 0 以下は「全行」（行数制限なし）。実際の改行で全文を折り返して表示する。
  if (maxLines <= 0) {
    return (
      <button
        type="button"
        tabIndex={-1}
        onClick={onPaste}
        title={t("tip_paste")}
        style={{
          ...buttonStyle,
          cursor: "pointer",
          whiteSpace: "pre-wrap",
          wordBreak: "break-word",
        }}
      >
        {content}
      </button>
    );
  }

  if (maxLines === 1) {
    let offset = 0;
    return (
      <button
        type="button"
        tabIndex={-1}
        onClick={onPaste}
        title={t("tip_paste")}
        style={{
          ...buttonStyle,
          cursor: "pointer",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
      >
        {content.split(/\r?\n/).map((line) => {
          const key = `${offset}:${line}`;
          const showLineBreak = offset > 0;
          offset += line.length + 1;
          return (
            <span key={key}>
              {showLineBreak && <span style={{ opacity: 0.4 }}>↵ </span>}
              {line}
            </span>
          );
        })}
      </button>
    );
  }

  return (
    <button
      type="button"
      tabIndex={-1}
      onClick={onPaste}
      title={t("tip_paste")}
      style={{
        ...buttonStyle,
        cursor: "pointer",
        display: "-webkit-box",
        WebkitBoxOrient: "vertical",
        WebkitLineClamp: maxLines,
        overflow: "hidden",
        whiteSpace: "pre-wrap",
        wordBreak: "break-word",
      }}
    >
      {content}
    </button>
  );
}
