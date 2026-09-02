import { useState } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router";
import { ErrorBar } from "../components/ErrorBar";
import { Sidebar } from "../components/Sidebar";
import Live from "./main/Live";
import History from "./main/History";
import General from "./settings/General";
import Transcription from "./settings/Transcription";
import Translation from "./settings/Translation";
import Overlay from "./settings/Overlay";

const KEY = "babelay.sidebar";

export default function MainApp() {
  const [collapsed, setCollapsed] = useState(() => {
    try { return localStorage.getItem(KEY) === "collapsed"; } catch { return false; }
  });
  const toggle = () => {
    const next = !collapsed;
    setCollapsed(next);
    try { localStorage.setItem(KEY, next ? "collapsed" : "expanded"); } catch { /* ignore */ }
  };

  return (
    <HashRouter>
      <div className="flex h-full">
        <Sidebar collapsed={collapsed} onToggle={toggle} />
        <main className="flex-1 overflow-auto p-6">
          <ErrorBar />
          <Routes>
            <Route path="/" element={<Navigate to="/live" replace />} />
            <Route path="/live" element={<Live />} />
            <Route path="/history" element={<History />} />
            <Route path="/settings/general" element={<General />} />
            <Route path="/settings/transcription" element={<Transcription />} />
            <Route path="/settings/translation" element={<Translation />} />
            <Route path="/settings/overlay" element={<Overlay />} />
            <Route path="*" element={<Navigate to="/live" replace />} />
          </Routes>
        </main>
      </div>
    </HashRouter>
  );
}
