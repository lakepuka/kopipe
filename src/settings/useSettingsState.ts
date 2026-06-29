import { useCallback, useEffect, useRef, useState } from "react";

import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { DEFAULT_LANG, type Lang, loadLang } from "../i18n";
import { DEFAULT_THEME, applyTheme, type Theme, loadTheme } from "../lib/theme";
import {
  getSettings,
  loadDisplay,
  parseUpdateCheck,
  saveSetting,
  SETTING_KEYS,
} from "../lib/prefs";
import {
  appVersion,
  clearClips,
  imageDirPath,
  imageStorageBytes,
  resetAppSettings,
  setImageLimit as setImageLimitCommand,
  setPinned as setPinnedCommand,
  setShortcut as setShortcutCommand,
} from "../services/api";
import { DEFAULT_SHORTCUT, loadShortcut } from "./shortcut";

export function useSettingsState() {
  const [theme, setTheme] = useState<Theme>(DEFAULT_THEME);
  const [lang, setLang] = useState<Lang>(DEFAULT_LANG);
  const [shortcut, setShortcut] = useState<string>(DEFAULT_SHORTCUT);
  const [imageBytes, setImageBytes] = useState(0);
  const [imageDir, setImageDir] = useState("");
  const [imageLimit, setImageLimit] = useState(500);
  const [version, setVersion] = useState("");
  const [showIcons, setShowIcons] = useState(true);
  const [maxLines, setMaxLines] = useState(1);
  const [pinned, setPinned] = useState(false);
  const [autoStart, setAutoStart] = useState(false);
  const [updateCheck, setUpdateCheck] = useState(true);
  const bodyRef = useRef<HTMLDivElement>(null);

  const refreshImageSize = useCallback(() => {
    imageStorageBytes()
      .then(setImageBytes)
      .catch(() => {});
  }, []);

  const refreshImageDir = useCallback(() => {
    imageDirPath()
      .then(setImageDir)
      .catch(() => {});
  }, []);

  const refreshImageLimit = useCallback(() => {
    getSettings().then((s) => {
      const n = parseInt(s[SETTING_KEYS.imageLimitMb] ?? "", 10);
      setImageLimit(Number.isFinite(n) && n > 0 ? n : 500);
    });
  }, []);

  const saveShortcut = useCallback(async (accel: string) => {
    try {
      await setShortcutCommand(accel);
      setShortcut(accel);
    } catch (e) {
      console.error(e);
    }
  }, []);

  async function saveLimit() {
    const mb = Math.max(1, Math.floor(imageLimit) || 500);
    setImageLimit(mb);
    try {
      await setImageLimitCommand(mb);
      refreshImageSize();
    } catch (e) {
      console.error(e);
    }
  }

  useEffect(() => {
    loadTheme().then((th) => {
      setTheme(th);
      applyTheme(th);
    });
    loadLang().then(setLang);
    loadShortcut().then(setShortcut);
    loadDisplay().then((d) => {
      setShowIcons(d.showIcons);
      setMaxLines(d.maxLines);
      setPinned(d.pinned);
    });
    getSettings().then((s) => setUpdateCheck(parseUpdateCheck(s[SETTING_KEYS.updateCheck])));
    isEnabled()
      .then(setAutoStart)
      .catch(() => {});
    refreshImageSize();
    refreshImageDir();
    refreshImageLimit();
    appVersion()
      .then(setVersion)
      .catch(() => {});

    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) {
        loadLang().then(setLang);
        refreshImageSize();
        refreshImageDir();
        refreshImageLimit();
      } else if (bodyRef.current) {
        bodyRef.current.scrollTop = 0;
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [refreshImageDir, refreshImageLimit, refreshImageSize]);

  async function clearKind(kind: "text" | "image" | "files") {
    try {
      await clearClips(kind);
      if (kind === "image") refreshImageSize();
    } catch (e) {
      console.error(e);
    }
  }

  async function changeImageDir() {
    try {
      const dir = await open({ directory: true, title: "画像の保存先フォルダ" });
      if (typeof dir === "string") {
        await saveSetting(SETTING_KEYS.imageDir, dir);
        refreshImageDir();
        refreshImageSize();
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function resetSettings() {
    try {
      await resetAppSettings();
      setTheme(DEFAULT_THEME);
      applyTheme(DEFAULT_THEME);
      setShortcut(DEFAULT_SHORTCUT);
      setLang(DEFAULT_LANG);
      setUpdateCheck(true);
      refreshImageDir();
    } catch (e) {
      console.error(e);
    }
  }

  function changeTheme(value: Theme) {
    setTheme(value);
    applyTheme(value);
    saveSetting(SETTING_KEYS.theme, value);
  }

  function changeLang(value: Lang) {
    setLang(value);
    saveSetting(SETTING_KEYS.lang, value);
  }

  function changeShowIcons(value: boolean) {
    setShowIcons(value);
    saveSetting(SETTING_KEYS.showRowIcons, value ? "true" : "false");
  }

  function changeMaxLines(value: number) {
    setMaxLines(value);
    saveSetting(SETTING_KEYS.maxLines, String(value));
  }

  async function changeAutoStart(value: boolean) {
    setAutoStart(value);
    try {
      if (value) await enable();
      else await disable();
    } catch (e) {
      console.error(e);
      isEnabled()
        .then(setAutoStart)
        .catch(() => {});
    }
  }

  async function changePinned(value: boolean) {
    setPinned(value);
    try {
      await setPinnedCommand(value);
    } catch (e) {
      console.error(e);
    }
  }

  function changeUpdateCheck(value: boolean) {
    setUpdateCheck(value);
    saveSetting(SETTING_KEYS.updateCheck, value ? "true" : "false");
  }

  return {
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
  };
}
