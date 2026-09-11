import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router";
import { TopBar } from "../components/TopBar";
import Live from "./main/Live";
import History from "./main/History";
import Settings from "./Settings";
import { useUpdate } from "../lib/update";

export default function MainApp() {
  useEffect(() => useUpdate.getState().subscribe(), []);

  return (
    <HashRouter>
      <div className="flex h-full flex-col bg-base-100">
        <TopBar />
        <main className="min-h-0 flex-1 overflow-auto px-6 py-5">
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
