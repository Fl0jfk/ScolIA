"use client";

import {
  createContext,
  Suspense,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import EleveDossierClient from "@/app/(admin)/eleves/dossier/[id]/EleveDossierClient";

type EleveDossierModalApi = {
  open: (eleveId: string, opts?: { subView?: "dossier" | "inscription" }) => void;
  close: () => void;
  isOpen: boolean;
  eleveId: string | null;
};

const EleveDossierModalContext = createContext<EleveDossierModalApi | null>(null);

export function useEleveDossierModal(): EleveDossierModalApi {
  const ctx = useContext(EleveDossierModalContext);
  if (!ctx) {
    throw new Error("useEleveDossierModal doit être utilisé dans EleveDossierModalProvider");
  }
  return ctx;
}

/** Variante souple pour composants hors provider (no-op). */
export function useEleveDossierModalOptional(): EleveDossierModalApi | null {
  return useContext(EleveDossierModalContext);
}

function ModalBody({
  eleveId,
  initialSubView,
  onClose,
  onNavigateEleve,
}: {
  eleveId: string;
  initialSubView: "dossier" | "inscription";
  onClose: () => void;
  onNavigateEleve: (id: string) => void;
}) {
  return (
    <div
      className="fixed inset-0 z-[80] flex items-stretch justify-center p-0 sm:items-center sm:p-4"
      role="dialog"
      aria-modal="true"
      aria-label="Dossier élève"
    >
      <button
        type="button"
        className="absolute inset-0 bg-black/40"
        aria-label="Fermer"
        onClick={onClose}
      />
      <div className="relative z-[1] flex h-[100dvh] w-full flex-col overflow-hidden bg-[var(--dash-surface)] shadow-2xl sm:h-[min(92dvh,920px)] sm:max-w-6xl sm:rounded-[1.75rem] sm:border sm:border-black/8">
        <div className="flex shrink-0 items-center justify-between gap-3 border-b border-black/6 bg-white px-4 py-3 sm:px-5">
          <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
            {initialSubView === "inscription" ? "Documents d’inscription" : "Dossier élève"}
          </p>
          <button
            type="button"
            onClick={onClose}
            className="inline-flex h-9 w-9 items-center justify-center rounded-full bg-[var(--dash-ink)] text-white transition hover:brightness-110"
            aria-label="Fermer le dossier"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={2}
              stroke="currentColor"
              className="h-4 w-4"
              aria-hidden
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18 18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto">
          <Suspense
            fallback={
              <div className="p-8 text-sm text-neutral-500">Chargement du dossier…</div>
            }
          >
            <EleveDossierClient
              key={`${eleveId}:${initialSubView}`}
              mode="modal"
              eleveId={eleveId}
              initialModalSubView={initialSubView}
              onClose={onClose}
              onNavigateEleve={onNavigateEleve}
            />
          </Suspense>
        </div>
      </div>
    </div>
  );
}

export default function EleveDossierModalProvider({ children }: { children: ReactNode }) {
  const [eleveId, setEleveId] = useState<string | null>(null);
  const [subView, setSubView] = useState<"dossier" | "inscription">("dossier");

  const open = useCallback((id: string, opts?: { subView?: "dossier" | "inscription" }) => {
    setSubView(opts?.subView === "inscription" ? "inscription" : "dossier");
    setEleveId(id);
  }, []);

  const close = useCallback(() => {
    setEleveId(null);
    setSubView("dossier");
  }, []);

  useEffect(() => {
    if (!eleveId) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      document.body.style.overflow = prev;
      window.removeEventListener("keydown", onKey);
    };
  }, [eleveId, close]);

  const api = useMemo<EleveDossierModalApi>(
    () => ({
      open,
      close,
      isOpen: Boolean(eleveId),
      eleveId,
    }),
    [open, close, eleveId],
  );

  return (
    <EleveDossierModalContext.Provider value={api}>
      {children}
      {eleveId ? (
        <ModalBody
          eleveId={eleveId}
          initialSubView={subView}
          onClose={close}
          onNavigateEleve={(id) => {
            setSubView("dossier");
            setEleveId(id);
          }}
        />
      ) : null}
    </EleveDossierModalContext.Provider>
  );
}
