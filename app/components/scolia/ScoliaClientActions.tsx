"use client";

import { useCallback, useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useEleveDossierModalOptional } from "@/app/components/shell/EleveDossierModalProvider";
import type { BrainClientAction } from "@/app/lib/brain-ai/types";

/**
 * Exécute les clientActions renvoyées par Brain AI (ouvrir page / dossier).
 */
export function useScoliaClientActions() {
  const router = useRouter();
  const dossierModal = useEleveDossierModalOptional();
  const [urlModal, setUrlModal] = useState<{ href: string; title: string } | null>(null);

  const runActions = useCallback(
    (actions: BrainClientAction[] | undefined | null) => {
      if (!actions?.length) return;
      for (const action of actions) {
        if (action.type === "open_eleve_dossier") {
          if (dossierModal) {
            dossierModal.open(action.eleveId, { subView: action.subView || "dossier" });
          } else {
            const href =
              action.subView === "inscription"
                ? `/eleves/dossier/${action.eleveId}/inscription`
                : `/eleves/dossier/${action.eleveId}`;
            router.push(href);
          }
          continue;
        }
        if (action.type === "open_url_modal") {
          setUrlModal({ href: action.href, title: action.title || "Aperçu" });
          continue;
        }
        if (action.type === "open_route") {
          // Modules « lourds » : navigation pleine page ; le reste peut rester en push.
          router.push(action.href);
        }
      }
    },
    [dossierModal, router],
  );

  const closeUrlModal = useCallback(() => setUrlModal(null), []);

  return { runActions, urlModal, closeUrlModal };
}

export function ScoliaUrlModal({
  href,
  title,
  onClose,
}: {
  href: string;
  title: string;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="fixed inset-0 z-[125] flex items-stretch justify-center p-0 sm:items-center sm:p-4"
      role="dialog"
      aria-modal="true"
      aria-label={title}
    >
      <button type="button" className="absolute inset-0 bg-black/45" aria-label="Fermer" onClick={onClose} />
      <div className="relative z-[1] flex h-[100dvh] w-full flex-col overflow-hidden bg-white shadow-2xl sm:h-[min(92dvh,920px)] sm:max-w-6xl sm:rounded-[1.75rem]">
        <div className="flex shrink-0 items-center justify-between gap-3 border-b border-black/8 px-4 py-3">
          <p className="truncate text-sm font-semibold text-slate-800">{title}</p>
          <div className="flex items-center gap-2">
            <a
              href={href}
              className="rounded-full border border-slate-200 px-3 py-1.5 text-xs font-medium text-slate-700 hover:bg-slate-50"
            >
              Plein écran
            </a>
            <button
              type="button"
              onClick={onClose}
              className="inline-flex h-9 w-9 items-center justify-center rounded-full bg-slate-900 text-white"
              aria-label="Fermer"
            >
              ×
            </button>
          </div>
        </div>
        <iframe title={title} src={href} className="min-h-0 flex-1 w-full border-0 bg-white" />
      </div>
    </div>
  );
}
