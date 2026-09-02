import { useTranslation } from "react-i18next";

export default function History() {
  const { t } = useTranslation();
  return <div className="rounded-lg bg-base-2 p-4 text-sm text-fg-muted">{t("history.empty")}</div>;
}
