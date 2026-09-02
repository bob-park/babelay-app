import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./index.css";
import { useSettings } from "./lib/settings";
import { useSession } from "./lib/session";
import { applyTheme } from "./lib/theme";
import { initI18n, resolveLang } from "./lib/i18n";

// 페이지 컴포넌트는 Task 7~9에서 만든다. 그때까지는 임시 플레이스홀더.
const MainApp = React.lazy(() => import("./pages/MainApp"));
const OverlayWindow = React.lazy(() => import("./pages/OverlayWindow"));
const Onboarding = React.lazy(() => import("./pages/Onboarding"));

function Root() {
  const { settings, load, subscribeBackend } = useSettings();
  const bindSession = useSession((s) => s.bind);
  const [ready, setReady] = useState(false);
  const label = getCurrentWindow().label;

  useEffect(() => {
    const unsubSettings = subscribeBackend();
    const unsubSession = bindSession();
    load().then(() => setReady(true));
    return () => {
      unsubSettings();
      unsubSession();
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
