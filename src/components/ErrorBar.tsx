import { useTranslation } from "react-i18next";
import { useSettings } from "../lib/settings";

// 설정 IPC 실패는 조용히 삼키지 않는다.
export function ErrorBar() {
  const { t } = useTranslation();
  const error = useSettings((s) => s.error);
  const clearError = useSettings((s) => s.clearError);
  if (!error) return null;
  return (
    <div role="alert" className="alert alert-error mb-4 text-sm">
      <span className="min-w-0 break-words">{error}</span>
      <button type="button" onClick={clearError} aria-label={t("common.dismiss")} className="btn btn-ghost btn-xs">✕</button>
    </div>
  );
}
