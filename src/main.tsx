import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import { useSettings } from "./lib/settings";
import { useSession } from "./lib/session";
import { useModels } from "./lib/models";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";

// 세션 스토어는 세 창 모두가 붙는다(오버레이도 자막을 그린다).
const MainApp = React.lazy(() => import("./pages/MainApp"));
const OverlayWindow = React.lazy(() => import("./pages/OverlayWindow"));
const Onboarding = React.lazy(() => import("./pages/Onboarding"));

// React 마운트 전에 붙여야 오버레이 창이 불투명하게 번쩍이지 않는다.
const label = getCurrentWindow().label;
if (label === "overlay") document.body.classList.add("overlay");

function Root() {
  const { settings, load, subscribeBackend } = useSettings();
  const bindSession = useSession((s) => s.bind);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    const unsubSettings = subscribeBackend();
    const unsubSession = bindSession();
    const unsubModels = label === "overlay" ? () => {} : useModels.getState().bind();
    load().then(() => {
      setReady(true);
      if (label !== "overlay") useModels.getState().refresh();
    });
    return () => {
      unsubSettings();
      unsubSession();
      unsubModels();
    };
  }, []);

  useEffect(() => {
    if (!settings) return;
    applyTheme(settings.general.theme);
    initI18n(resolveLang(settings.general.ui_language, navigator.language));
  }, [settings?.general.theme, settings?.general.ui_language]);

  if (!ready || !settings) return null;
  const page = label === "overlay" ? <OverlayWindow /> : label === "onboarding" ? <Onboarding /> : <MainApp />;
  return <React.Suspense fallback={null}>{page}</React.Suspense>;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Root />
  </React.StrictMode>,
);
