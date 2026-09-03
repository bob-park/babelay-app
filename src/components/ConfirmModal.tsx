import { useEffect, useId, useRef } from "react";
import { useTranslation } from "react-i18next";

interface Props { open: boolean; message: string; onConfirm: () => void; onCancel: () => void }

export function ConfirmModal({ open, message, onConfirm, onCancel }: Props) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);
  const id = useId();
  useEffect(() => {
    const d = ref.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);
  return (
    <dialog ref={ref} className="modal" aria-labelledby={id} onClose={onCancel} onClick={(e) => e.stopPropagation()}>
      <div className="modal-box max-w-sm">
        <p id={id} className="text-sm">{message}</p>
        <div className="modal-action">
          <button type="button" className="btn btn-ghost btn-sm" onClick={onCancel}>{t("common.cancel")}</button>
          <button type="button" className="btn btn-error btn-sm" onClick={onConfirm}>{t("common.confirm")}</button>
        </div>
      </div>
    </dialog>
  );
}
