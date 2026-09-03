import { useState } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router";
import { DownloadToast } from "../components/DownloadToast";
import { ErrorBar } from "../components/ErrorBar";
import { Sidebar } from "../components/Sidebar";
import Live from "./main/Live";
import History from "./main/History";
import Settings from "./Settings";

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
      <div className="flex h-full bg-base-100">
        <DownloadToast />
        <Sidebar collapsed={collapsed} onToggle={toggle} />
        <main className="flex-1 overflow-auto px-6 py-5">
          <ErrorBar />
          <Routes>
            <Route path="/" element={<Navigate to="/live" replace />} />
            <Route path="/live" element={<Live />} />
            <Route path="/history" element={<History />} />
            <Route path="/settings" element={<Navigate to="/settings/general" replace />} />
            <Route path="/settings/transcription" element={<Navigate to="/settings/models" replace />} />
            <Route path="/settings/:tab" element={<Settings />} />
            <Route path="*" element={<Navigate to="/live" replace />} />
          </Routes>
        </main>
      </div>
    </HashRouter>
  );
}
