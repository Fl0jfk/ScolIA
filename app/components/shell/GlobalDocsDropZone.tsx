"use client";

import { useEffect, useRef, useState, type DragEvent } from "react";
import Link from "next/link";
import { useOneDriveOcrGate } from "@/app/hooks/useOneDriveOcrGate";
import {
  pollSidebarOcrJobStatus,
  startSidebarOcrDeposit,
  type SidebarOcrDepositProgress,
} from "@/app/lib/ocr-sidebar-deposit";
import { BATCH_JOB_LAST_RESULTS_KEY, BATCH_JOB_STORAGE_KEY } from "@/app/lib/ocr-page-model";

type Props = {
  onCloseMobile?: () => void;
  /** Accès module agent-ia-ocr */
  ocrAvailable?: boolean;
};

const ACCEPT = ".pdf,application/pdf";

export default function GlobalDocsDropZone({ onCloseMobile, ocrAvailable = true }: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const abortRef = useRef<AbortController | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const [progress, setProgress] = useState<SidebarOcrDepositProgress | null>(null);
  const gate = useOneDriveOcrGate(ocrAvailable);

  useEffect(() => {
    return () => {
      abortRef.current?.abort();
    };
  }, []);

  // Poll rapide tant qu’un lot tourne (sans quitter la page).
  useEffect(() => {
    if (!progress || progress.phase !== "processing" || !progress.jobId) return;
    const jobId = progress.jobId;
    let cancelled = false;
    const tick = async () => {
      try {
        const st = await pollSidebarOcrJobStatus(jobId);
        if (cancelled) return;
        if (st.done) {
          try {
            localStorage.removeItem(BATCH_JOB_STORAGE_KEY);
            localStorage.setItem(BATCH_JOB_LAST_RESULTS_KEY, jobId);
          } catch {
            /* ignore */
          }
          setProgress({
            phase: "done",
            percent: 100,
            label:
              st.failed > 0
                ? `Terminé — ${st.completed} ok, ${st.failed} en échec`
                : `Terminé — ${st.completed || st.total} document${(st.completed || st.total) > 1 ? "s" : ""}`,
            jobId,
            completed: st.completed,
            failed: st.failed,
          });
          void gate.refreshSuivi();
          window.setTimeout(() => {
            setProgress((prev) => (prev?.phase === "done" ? null : prev));
          }, 8000);
          return;
        }
        setProgress((prev) =>
          prev?.jobId === jobId
            ? {
                ...prev,
                phase: "processing",
                percent: Math.max(prev.percent, Math.min(99, st.percent || prev.percent)),
                label: st.label || prev.label,
                completed: st.completed,
                failed: st.failed,
              }
            : prev,
        );
        void gate.refreshSuivi();
      } catch {
        /* ignore transient */
      }
      if (!cancelled) {
        window.setTimeout(() => void tick(), 2500);
      }
    };
    void tick();
    return () => {
      cancelled = true;
    };
  }, [progress?.phase, progress?.jobId, gate.refreshSuivi]);

  // Pas de module OCR, ou pas nommé sur un flux → rien dans la sidebar.
  if (!ocrAvailable) return null;
  if (!gate.ready) return null;
  if (!gate.assigned) return null;

  const busy =
    progress?.phase === "uploading" ||
    progress?.phase === "starting" ||
    progress?.phase === "processing";
  const dropLocked = !gate.configured || !gate.connected || gate.checking || busy;

  async function goWithFiles(files: FileList | File[] | null) {
    if (!files || files.length === 0) return;
    if (busy) {
      setHint("Un dépôt est déjà en cours — patientez ou ouvrez le suivi.");
      return;
    }
    if (!gate.configured) {
      setHint("OneDrive n’est pas activé pour cet établissement.");
      return;
    }
    if (!gate.connected) {
      setHint("Connectez OneDrive avant de déposer.");
      return;
    }
    const stillOk = await gate.ensureConnected();
    if (!stillOk) {
      setHint("Session OneDrive expirée — reconnectez-vous.");
      return;
    }

    const pdfs = Array.from(files).filter(
      (f) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf"),
    );
    if (pdfs.length === 0) {
      setHint("PDF uniquement.");
      return;
    }

    setHint(null);
    onCloseMobile?.();
    abortRef.current?.abort();
    abortRef.current = new AbortController();
    const signal = abortRef.current.signal;

    try {
      const { jobId, serverSelfRelays } = await startSidebarOcrDeposit({
        files: pdfs,
        getAccessToken: gate.getAccessToken,
        signal,
        onProgress: setProgress,
      });
      setProgress((prev) =>
        prev
          ? { ...prev, jobId, serverSelfRelays, phase: "processing" }
          : {
              phase: "processing",
              percent: 42,
              label: `OCR en cours (${pdfs.length} PDF)`,
              jobId,
              serverSelfRelays,
            },
      );
      void gate.refreshSuivi();
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return;
      setProgress({
        phase: "error",
        percent: 0,
        label: e instanceof Error ? e.message : "Échec du dépôt",
      });
      setHint(e instanceof Error ? e.message : "Échec du dépôt");
    }
  }

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragOver(false);
    if (dropLocked) return;
    void goWithFiles(e.dataTransfer.files);
  }

  const attention =
    gate.suivi.activeCount + gate.suivi.failedRecent + (gate.suivi.needsToken ? 1 : 0);

  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between gap-2">
        <p className="text-[10px] font-semibold uppercase tracking-[0.16em] text-[var(--dash-mid)]">
          Dépôt intelligent
        </p>
        {attention > 0 || busy ? (
          <Link
            href="/agentIAOCR?suivi=1"
            onClick={onCloseMobile}
            className="inline-flex items-center gap-1 rounded-full bg-[var(--dash-lime)] px-2 py-0.5 text-[10px] font-black text-[var(--dash-ink)]"
            title="Ouvrir le suivi OCR"
          >
            <span
              className={`h-1.5 w-1.5 rounded-full bg-[var(--dash-ink)] ${
                busy ? "animate-pulse" : ""
              }`}
              aria-hidden
            />
            {busy ? "En cours" : `Suivi ${attention > 99 ? "99+" : attention}`}
          </Link>
        ) : (
          <Link
            href="/agentIAOCR"
            onClick={onCloseMobile}
            className="text-[10px] font-semibold text-neutral-500 underline-offset-2 hover:text-[var(--dash-ink)] hover:underline"
          >
            Suivi
          </Link>
        )}
      </div>

      {gate.checking && !gate.connected ? (
        <p className="rounded-xl bg-white/70 px-2 py-2 text-[11px] text-neutral-500">
          Vérification OneDrive…
        </p>
      ) : !gate.configured ? (
        <p className="rounded-xl border border-amber-200 bg-amber-50 px-2 py-2 text-[11px] font-medium text-amber-900">
          OneDrive non activé (Paramètres → Intégrations).
        </p>
      ) : !gate.connected ? (
        <div className="space-y-1.5 rounded-2xl border border-dashed border-black/15 bg-white/80 px-3 py-3 text-center">
          <p className="text-[11px] font-semibold text-[var(--dash-ink)]">
            Connectez OneDrive pour déposer
          </p>
          <p className="text-[10px] text-neutral-500">
            L’OCR range ensuite dans les dossiers élèves / personnel.
          </p>
          <button
            type="button"
            onClick={() => void gate.connect()}
            className="mt-1 w-full rounded-xl bg-[var(--dash-ink)] px-3 py-2 text-[11px] font-bold text-white hover:brightness-110"
          >
            Connecter OneDrive
          </button>
        </div>
      ) : progress && progress.phase !== "error" ? (
        <div
          className={`rounded-2xl border px-3 py-3 ${
            progress.phase === "done"
              ? "border-emerald-300 bg-emerald-50"
              : "border-[var(--dash-ink)]/20 bg-white/90"
          }`}
          role="status"
          aria-live="polite"
        >
          <div className="flex items-start gap-2">
            {progress.phase === "done" ? (
              <span className="text-base leading-none" aria-hidden>
                ✓
              </span>
            ) : (
              <span
                className="mt-0.5 inline-block h-3.5 w-3.5 shrink-0 animate-spin rounded-full border-2 border-[var(--dash-ink)] border-t-transparent"
                aria-hidden
              />
            )}
            <div className="min-w-0 flex-1">
              <p className="text-[11px] font-bold leading-snug text-[var(--dash-ink)]">
                {progress.phase === "uploading"
                  ? "Envoi en cours"
                  : progress.phase === "starting"
                    ? "Lancement OCR"
                    : progress.phase === "done"
                      ? "Dépôt terminé"
                      : "OCR en cours"}
              </p>
              <p
                className="mt-0.5 line-clamp-2 text-[10px] leading-snug text-neutral-600"
                title={progress.label}
              >
                {progress.label}
              </p>
              {progress.phase !== "done" ? (
                <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-black/8">
                  <div
                    className="h-full rounded-full bg-[var(--dash-ink)] transition-[width] duration-500"
                    style={{ width: `${Math.max(4, Math.min(100, progress.percent))}%` }}
                  />
                </div>
              ) : null}
              <p className="mt-1.5 text-[10px] text-neutral-500">
                {progress.phase === "done"
                  ? "Vous pouvez ouvrir le suivi pour le détail."
                  : "Restez sur cette page — pas besoin d’ouvrir l’OCR."}
              </p>
              <Link
                href="/agentIAOCR?suivi=1"
                onClick={onCloseMobile}
                className="mt-1 inline-block text-[10px] font-semibold text-[var(--dash-ink)] underline-offset-2 hover:underline"
              >
                Voir le détail →
              </Link>
            </div>
          </div>
        </div>
      ) : (
        <div
          role="button"
          tabIndex={0}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              inputRef.current?.click();
            }
          }}
          onClick={() => inputRef.current?.click()}
          onDragEnter={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragOver(true);
          }}
          onDragLeave={(e) => {
            e.preventDefault();
            setDragOver(false);
          }}
          onDrop={onDrop}
          className={`flex cursor-pointer flex-col items-center gap-1.5 rounded-2xl border border-dashed px-3 py-3 text-center transition ${
            dragOver
              ? "border-[var(--dash-ink)] bg-[color:var(--dash-lime)]/70"
              : "border-black/12 bg-white/80 hover:border-black/25 hover:bg-white"
          }`}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            strokeWidth={1.75}
            stroke="currentColor"
            className="h-4 w-4 text-[var(--dash-ink)]"
            aria-hidden
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5"
            />
          </svg>
          <span className="text-[11px] font-semibold leading-snug text-[var(--dash-ink)]">
            Déposer un PDF
          </span>
          <span className="text-[10px] text-neutral-500">
            OCR ici — sans quitter la page
          </span>
          <input
            ref={inputRef}
            type="file"
            multiple
            accept={ACCEPT}
            className="hidden"
            onChange={(e) => {
              void goWithFiles(e.target.files);
              e.target.value = "";
            }}
          />
        </div>
      )}

      {hint ? <p className="text-[11px] font-medium text-amber-800">{hint}</p> : null}
      {progress?.phase === "error" ? (
        <button
          type="button"
          onClick={() => {
            setProgress(null);
            setHint(null);
          }}
          className="text-[10px] font-semibold text-[var(--dash-ink)] underline-offset-2 hover:underline"
        >
          Réessayer un dépôt
        </button>
      ) : null}
      {gate.error && gate.connected === false ? (
        <p className="text-[11px] font-medium text-red-600">{gate.error}</p>
      ) : null}
    </div>
  );
}
