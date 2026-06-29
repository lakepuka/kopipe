import { useState } from "react";
import { enable } from "@tauri-apps/plugin-autostart";

import { DEFAULT_LANG, type Lang, useT } from "../i18n";
import { saveSetting, SETTING_KEYS } from "../lib/prefs";

const LANGS: { value: Lang; label: string }[] = [
  { value: "en", label: "English" },
  { value: "ja", label: "日本語" },
];

// 初回起動時のみのオンボーディング（すべてフロント側）。
// 言語選択 → ようこそ → 自動起動の推奨、の順に進む。
export function Onboarding({ onFinish }: { onFinish: () => void }) {
  const t = useT();
  const [step, setStep] = useState<0 | 1 | 2>(0);
  const [lang, setLang] = useState<Lang>(DEFAULT_LANG);

  function chooseLang(value: Lang) {
    setLang(value);
    // 保存すると settings-changed が emit され、以降の画面も選んだ言語で表示される。
    saveSetting(SETTING_KEYS.lang, value);
  }

  // 最終ステップ。自動起動を有効化（任意）し、確認済みフラグを保存して終了。
  async function finish(enableAutostart: boolean) {
    if (enableAutostart) {
      try {
        await enable();
      } catch {
        // 自動起動の可否は致命的でないので無視。
      }
    }
    saveSetting(SETTING_KEYS.autostartPrompted, "true");
    onFinish();
  }

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 60,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(0,0,0,0.7)",
        padding: 24,
      }}
    >
      <div className="onboard">
        <div className="onboard-dots">
          {[0, 1, 2].map((i) => (
            <span key={i} className={`onboard-dot${i === step ? " active" : ""}`} />
          ))}
        </div>

        <div className="onboard-body">
          {step === 0 && (
            <>
              <h2 className="onboard-title">{t("onboard_lang_title")}</h2>
              <div className="onboard-langs">
                {LANGS.map((o) => (
                  <label key={o.value}>
                    <input
                      type="radio"
                      name="onboard-lang"
                      checked={lang === o.value}
                      onChange={() => chooseLang(o.value)}
                    />
                    {o.label}
                  </label>
                ))}
              </div>
            </>
          )}

          {step === 1 && (
            <>
              <h2 className="onboard-title">{t("help_title")}</h2>
              <p className="onboard-text">{t("help_body")}</p>
            </>
          )}

          {step === 2 && (
            <>
              <h2 className="onboard-title">{t("autostart_prompt_title")}</h2>
              <p className="onboard-text">{t("autostart_prompt_message")}</p>
            </>
          )}
        </div>

        <div className="onboard-footer">
          {step < 2 ? (
            <button
              type="button"
              className="onboard-btn"
              onClick={() => setStep((s) => (s + 1) as 0 | 1 | 2)}
            >
              {t("next")}
            </button>
          ) : (
            <>
              <button type="button" className="onboard-btn secondary" onClick={() => finish(false)}>
                {t("later")}
              </button>
              <button type="button" className="onboard-btn" onClick={() => finish(true)}>
                {t("enable")}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
