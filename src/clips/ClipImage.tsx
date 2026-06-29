import { useEffect, useState } from "react";

import { imageDataUrl } from "../services/api";
import { useT } from "../i18n";

// 画像クリップのサムネイル。表示時に data URL を遅延取得する（一覧クエリは軽いまま）。
export function ClipImage({ id, onClick }: { id: number; onClick: () => void }) {
  const t = useT();
  const [src, setSrc] = useState("");

  useEffect(() => {
    let alive = true;
    imageDataUrl(id)
      .then((s) => {
        if (alive) setSrc(s);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [id]);

  if (!src) return <span style={{ opacity: 0.5 }}>{t("image_loading")}</span>;
  return (
    <button
      type="button"
      tabIndex={-1}
      onClick={onClick}
      title={t("tip_paste")}
      style={{
        border: "none",
        padding: 0,
        background: "none",
        cursor: "pointer",
        display: "block",
      }}
    >
      <img
        src={src}
        alt={t("preview")}
        style={{
          maxHeight: 80,
          maxWidth: "100%",
          borderRadius: 4,
          display: "block",
        }}
      />
    </button>
  );
}
