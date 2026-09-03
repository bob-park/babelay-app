import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "./icons";
import { SettingGroup } from "./SettingGroup";
import { api } from "../lib/tauri";
import { useSettings } from "../lib/settings";

export type Perm = "granted" | "denied" | "unknown";

/** 상태 아이콘. 온보딩 완료 단계도 같은 표시를 쓴다. */
export function PermissionIcon({ perm }: { perm: Perm | null }) {
  const { t } = useTranslation();
  if (perm === "granted") return <span className="flex h-6 w-6 items-center justify-center rounded-full bg-primary text-primary-content" aria-label={t("permission.granted")}><Icon name="check" /></span>;
  if (perm === "denied") return <span className="flex h-6 w-6 items-center justify-center rounded-full bg-error text-error-content" aria-label={t("permission.denied")}><Icon name="x" /></span>;
  return <span className="flex h-6 w-6 items-center justify-center rounded-full bg-neutral text-neutral-content" aria-label={t("permission.unknown")}><Icon name="help" /></span>;
}

// 권한 조회는 실제 탭을 만들어 TCC 프롬프트를 띄운다. 마운트당 한 번만.
export function PermissionRow({ onStatus }: { onStatus?: (p: Perm) => void }) {
  const { t } = useTranslation();
  const setError = useSettings((s) => s.setError);
  const [perm, setPerm] = useState<Perm | null>(null);
  const check = () => api.checkAudioPermission().then((p) => { setPerm(p); onStatus?.(p); }).catch(setError);
  useEffect(() => { check(); }, []);

  return (
    <div className="flex flex-col gap-3">
      <SettingGroup>
        <div className="flex items-center justify-between gap-4 px-4 py-3 text-sm">
          <div className="min-w-0">
            <div className="font-semibold">{t("permission.name")}</div>
            <div className="truncate text-xs text-fg-muted">{t("permission.path")}</div>
          </div>
          <PermissionIcon perm={perm} />
        </div>
      </SettingGroup>
      <div className="flex flex-wrap gap-2">
        <button type="button" className={`btn btn-sm ${perm === "denied" ? "btn-neutral" : "btn-primary"}`} onClick={check}>{t("permission.check")}</button>
        <button type="button" className={`btn btn-sm ${perm === "denied" ? "btn-primary" : "btn-outline"}`} onClick={() => api.openPrivacySettings().catch(setError)}>{t("permission.openSettings")}</button>
      </div>
    </div>
  );
}
