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

  const openUrlPreview = useCallback((href: string, title?: string) => {
    setUrlModal({ href, title: title || "Aperçu" });
  }, []);

  return { runActions, urlModal, closeUrlModal, openUrlPreview };
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
  const [frameSrc, setFrameSrc] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  useEffect(() => {
    let cancelled = false;
    let objectUrl: string | null = null;

    async function resolveFrameSrc() {
      setLoading(true);
      setLoadError(null);
      setFrameSrc(null);

      const isEleveDocApi = /^\/api\/eleves\/[^/]+\/documents\/[^/]+\/file\/?$/.test(
        href.split("?")[0] || "",
      );

      // Les PDF dossier élève redirigent vers S3 : l’iframe est bloquée (XFO / CSP).
      // On charge un blob same-origin pour l’aperçu.
      if (isEleveDocApi) {
        try {
          const jsonUrl = href.includes("?") ? `${href}&format=json` : `${href}?format=json`;
          const metaRes = await fetch(jsonUrl, { credentials: "same-origin", cache: "no-store" });
          const meta = (await metaRes.json().catch(() => ({}))) as {
            signedUrl?: string;
            contentType?: string;
            error?: string;
          };
          if (!metaRes.ok || !meta.signedUrl) {
            throw new Error(meta.error || "Document inaccessible.");
          }
          const fileRes = await fetch(meta.signedUrl, { cache: "no-store" });
          if (!fileRes.ok) {
            throw new Error("Impossible de télécharger le fichier.");
          }
          const blob = await fileRes.blob();
          const typed =
            blob.type && blob.type !== "application/octet-stream"
              ? blob
              : new Blob([blob], { type: meta.contentType || "application/pdf" });
          objectUrl = URL.createObjectURL(typed);
          if (cancelled) {
            URL.revokeObjectURL(objectUrl);
            return;
          }
          setFrameSrc(objectUrl);
          setLoading(false);
          return;
        } catch (e: unknown) {
          if (!cancelled) {
            setLoadError(e instanceof Error ? e.message : "Impossible d’ouvrir le document.");
            setLoading(false);
          }
          return;
        }
      }

      if (!cancelled) {
        setFrameSrc(href);
        setLoading(false);
      }
    }

    void resolveFrameSrc();
    return () => {
      cancelled = true;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [href]);

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
              target="_blank"
              rel="noopener noreferrer"
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
        {loading ? (
          <div className="flex min-h-0 flex-1 items-center justify-center text-sm text-neutral-500">
            Chargement du document…
          </div>
        ) : loadError ? (
          <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-6 text-center">
            <p className="text-sm font-medium text-slate-800">{loadError}</p>
            <a
              href={href}
              target="_blank"
              rel="noopener noreferrer"
              className="rounded-full bg-slate-900 px-4 py-2 text-xs font-semibold text-white"
            >
              Ouvrir dans un nouvel onglet
            </a>
          </div>
        ) : frameSrc ? (
          <iframe title={title} src={frameSrc} className="min-h-0 flex-1 w-full border-0 bg-white" />
        ) : null}
      </div>
    </div>
  );
}
