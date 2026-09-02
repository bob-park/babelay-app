import { useTranslation } from "react-i18next";
import { useSettings } from "../lib/settings";

// 설정 IPC 실패는 조용히 삼키지 않는다.
export function ErrorBar() {
  const { t } = useTranslation();
  const error = useSettings((s) => s.error);
  const clearError = useSettings((s) => s.clearError);
  if (!error) return null;
  return (
    <div role="alert" className="mb-4 flex items-start justify-between gap-3 rounded-md bg-danger px-3 py-2 text-sm text-white">
      <span className="min-w-0 break-words">{error}</span>
      <button type="button" onClick={clearError} aria-label={t("common.dismiss")} className="shrink-0 font-bold">
        ✕
      </button>
    </div>
  );
}
