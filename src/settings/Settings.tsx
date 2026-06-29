import { useEffect, useState } from "react";

import { resolvePalette, THEME_LABELS } from "../lib/theme";
import { buildAccelerator, describeShortcut } from "./shortcut";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { formatBytes } from "../lib/format";
import { type Lang, type TKey, useT } from "../i18n";
import { useSettingsState } from "./useSettingsState";

const LANGS: { value: Lang; label: string }[] = [
  { value: "en", label: "English" },
  { value: "ja", label: "日本語" },
];

type ConfirmAction = "clearText" | "clearFiles" | "clearImage" | "reset";

function DataAction({
  active,
  confirmText,
  label,
  confirmLabel,
  cancelLabel,
  onRequest,
  onConfirm,
  onCancel,
}: {
  active: boolean;
  confirmText: string;
  label: string;
  confirmLabel: string;
  cancelLabel: string;
  onRequest: () => void;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  if (active) {
    return (
      <div className="confirm-row">
        <span>{confirmText}</span>
        <button type="button" onClick={onConfirm} className="danger-btn">
          {confirmLabel}
        </button>
        <button type="button" onClick={onCancel}>
          {cancelLabel}
        </button>
      </div>
    );
  }

  return (
    <button type="button" onClick={onRequest} className="data-action-btn">
      {label}
    </button>
  );
}

export default function Settings() {
  const t = useT();
  const [recording, setRecording] = useState(false);
  const [confirm, setConfirm] = useState<ConfirmAction | null>(null);
  const {
    autoStart,
    bodyRef,
    changeAutoStart,
    changeImageDir,
    changeLang,
    changeMaxLines,
    changePinned,
    changeShowIcons,
    changeTheme,
    clearKind,
    imageBytes,
    imageDir,
    imageLimit,
    lang,
    maxLines,
    pinned,
    resetSettings,
    saveLimit,
    saveShortcut,
    setImageLimit,
    shortcut,
    showIcons,
    theme,
    updateCheck,
    changeUpdateCheck,
    version,
  } = useSettingsState();

  async function confirmClearKind(kind: "text" | "image" | "files") {
    setConfirm(null);
    await clearKind(kind);
  }

  async function confirmResetSettings() {
    setConfirm(null);
    await resetSettings();
  }

  // 記録モード中はキー入力を捕まえてショートカットを決める。
  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.code === "Escape") {
        setRecording(false);
        return;
      }
      const accel = buildAccelerator(e);
      if (!accel) return; // 修飾キーのみ等は確定しない
      setRecording(false);
      saveShortcut(accel);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [recording, saveShortcut]);

  // Esc でウィンドウを閉じる。記録中は記録取消、確認中は確認取消を優先。
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || recording) return;
      if (confirm) setConfirm(null);
      else getCurrentWindow().hide();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [recording, confirm]);

  return (
    <main className="container settings-window">
      {/* 枠なしウィンドウ。上部は透明なドラッグ帯、閉じるボタンは右上に固定する。 */}
      <div className="settings-dragbar" data-tauri-drag-region />
      <button
        type="button"
        className="settings-close"
        title={t("close")}
        onClick={() => getCurrentWindow().hide()}
      >
        ✕
      </button>

      {/* ドラッグ帯は固定し、本文だけをスクロールさせる。 */}
      <div className="settings-body" ref={bodyRef}>
        <section className="section">
          <h3 className="section-title">{t("appearance")}</h3>
          <div className="options">
            <label>
              <input
                type="checkbox"
                checked={showIcons}
                onChange={(e) => changeShowIcons(e.currentTarget.checked)}
              />
              {t("show_row_icons")}
            </label>
            <label>
              <input
                type="checkbox"
                checked={pinned}
                onChange={(e) => changePinned(e.currentTarget.checked)}
              />
              {t("pin_window")}
            </label>
            <label>
              {t("max_lines")}
              <select
                className="align-right-control"
                value={maxLines}
                onChange={(e) => changeMaxLines(Number(e.currentTarget.value))}
              >
                {[1, 3, 5, 10, 30, 100, 300, 0].map((n) => (
                  <option key={n} value={n}>
                    {n === 0 ? t("all_lines") : n}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </section>

        <section className="section">
          <h3 className="section-title">{t("trigger")}</h3>
          <div className="options">
            <label>
              <input
                type="radio"
                name="trigger"
                checked={shortcut === "double:Shift"}
                onChange={() => saveShortcut("double:Shift")}
              />
              {t("trigger_double_shift")}
            </label>
            <label>
              <input
                type="radio"
                name="trigger"
                checked={shortcut === "double:Control"}
                onChange={() => saveShortcut("double:Control")}
              />
              {t("trigger_double_ctrl")}
            </label>
            <label>
              <input
                type="radio"
                name="trigger"
                checked={!shortcut.startsWith("double:")}
                onChange={() => setRecording(true)}
              />
              {t("trigger_combo")}
              <button
                type="button"
                className={`combo-btn${recording ? " recording" : ""}`}
                onClick={() => setRecording((v) => !v)}
              >
                {recording
                  ? t("recording")
                  : shortcut.startsWith("double:")
                    ? t("unset")
                    : describeShortcut(shortcut)}
              </button>
            </label>
          </div>
        </section>

        <section className="section">
          <h3 className="section-title">{t("startup")}</h3>
          <div className="options">
            <label>
              <input
                type="checkbox"
                checked={autoStart}
                onChange={(e) => changeAutoStart(e.currentTarget.checked)}
              />
              {t("launch_at_startup")}
            </label>
          </div>
        </section>

        <section className="section">
          <h3 className="section-title">{t("theme")}</h3>
          <div className="theme-grid">
            {THEME_LABELS.map((opt) => {
              const p = resolvePalette(opt.value);
              return (
                <button
                  type="button"
                  key={opt.value}
                  className={`theme-card${theme === opt.value ? " selected" : ""}`}
                  onClick={() => changeTheme(opt.value)}
                  aria-pressed={theme === opt.value}
                >
                  <span className="theme-swatch" style={{ background: p.bg, color: p.fg }}>
                    Aa
                    <span className="theme-swatch-dot" style={{ background: p.accent }} />
                  </span>
                  <span className="theme-card-label">{t(`theme_${opt.value}` as TKey)}</span>
                </button>
              );
            })}
          </div>
        </section>

        <section className="section">
          <h3 className="section-title">{t("language")}</h3>
          <div className="options">
            {LANGS.map((opt) => (
              <label key={opt.value}>
                <input
                  type="radio"
                  name="lang"
                  checked={lang === opt.value}
                  onChange={() => changeLang(opt.value)}
                />
                {opt.label}
              </label>
            ))}
          </div>
        </section>

        <section className="section">
          <h3 className="section-title">{t("updates")}</h3>
          <div className="options">
            <label>
              <input
                type="checkbox"
                checked={updateCheck}
                onChange={(e) => changeUpdateCheck(e.currentTarget.checked)}
              />
              {t("check_updates")}
            </label>
          </div>
          <p className="muted limit-hint">{t("update_hint")}</p>
        </section>

        <section className="section">
          <h3 className="section-title">{t("image_dir")}</h3>
          <p className="path-box">{imageDir || t("default_paren")}</p>
          <div className="field">
            <span className="muted">{t("capacity")}</span>
            <span className="mono-value">{formatBytes(imageBytes)}</span>
          </div>
          <div className="field">
            <span className="muted">{t("limit")}</span>
            <span className="limit-control">
              <input
                className="limit-input"
                type="number"
                min={1}
                value={imageLimit}
                onChange={(e) => setImageLimit(Number(e.currentTarget.value))}
                onBlur={saveLimit}
              />
              <span className="muted">MB</span>
            </span>
          </div>
          <p className="muted limit-hint">{t("limit_hint")}</p>
          <button type="button" onClick={changeImageDir}>
            {t("change_folder")}
          </button>
        </section>

        <section className="section">
          <h3 className="section-title">{t("data")}</h3>
          <div className="options">
            <DataAction
              active={confirm === "clearText"}
              confirmText={t("confirm_clear_text")}
              label={t("clear_text")}
              confirmLabel={t("do_clear")}
              cancelLabel={t("cancel")}
              onRequest={() => setConfirm("clearText")}
              onConfirm={() => confirmClearKind("text")}
              onCancel={() => setConfirm(null)}
            />
            <DataAction
              active={confirm === "clearFiles"}
              confirmText={t("confirm_clear_files")}
              label={t("clear_files")}
              confirmLabel={t("do_clear")}
              cancelLabel={t("cancel")}
              onRequest={() => setConfirm("clearFiles")}
              onConfirm={() => confirmClearKind("files")}
              onCancel={() => setConfirm(null)}
            />
            <DataAction
              active={confirm === "clearImage"}
              confirmText={t("confirm_clear_image")}
              label={t("clear_image")}
              confirmLabel={t("do_clear")}
              cancelLabel={t("cancel")}
              onRequest={() => setConfirm("clearImage")}
              onConfirm={() => confirmClearKind("image")}
              onCancel={() => setConfirm(null)}
            />
            <DataAction
              active={confirm === "reset"}
              confirmText={t("confirm_reset")}
              label={t("reset")}
              confirmLabel={t("do_reset")}
              cancelLabel={t("cancel")}
              onRequest={() => setConfirm("reset")}
              onConfirm={confirmResetSettings}
              onCancel={() => setConfirm(null)}
            />
          </div>
        </section>

        <p className="app-version">kopipe{version && ` v${version}`}</p>
      </div>
    </main>
  );
}
