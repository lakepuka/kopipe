// PreviewModal — 画像の拡大プレビュー。どこをクリックしても閉じる。
export function PreviewModal({ src, onClose }: { src: string; onClose: () => void }) {
  return (
    <button
      type="button"
      tabIndex={-1}
      onClick={onClose}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 50,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(0,0,0,0.7)",
        border: "none",
        padding: 0,
        cursor: "zoom-out",
      }}
    >
      <img
        src={src}
        alt="preview"
        style={{
          maxWidth: "92vw",
          maxHeight: "92vh",
          objectFit: "contain",
          boxShadow: "0 8px 30px rgba(0,0,0,0.5)",
        }}
      />
    </button>
  );
}
