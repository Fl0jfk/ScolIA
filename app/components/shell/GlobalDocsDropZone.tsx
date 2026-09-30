"use client";

import { useRef, useState, type DragEvent } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { stageDashboardUpload } from "@/app/lib/dashboard-upload-bridge";
import { useOneDriveOcrGate } from "@/app/hooks/useOneDriveOcrGate";

type Props = {
  onCloseMobile?: () => void;
  /** Accès module agent-ia-ocr */
  ocrAvailable?: boolean;
};

const ACCEPT = ".pdf,application/pdf";

export default function GlobalDocsDropZone({ onCloseMobile, ocrAvailable = true }: Props) {
  const router = useRouter();
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);
  const [hint, setHint] = useState<string | null>(null);
  const gate = useOneDriveOcrGate(ocrAvailable);

  // Pas de module OCR, ou pas nommé sur un flux → rien dans la sidebar.
  if (!ocrAvailable) return null;
  if (!gate.ready) return null;
  if (!gate.assigned) return null;

  const dropLocked = !gate.configured || !gate.connected || gate.checking;

  async function goWithFiles(files: FileList | File[] | null) {
    if (!files || files.length === 0) return;
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
    const ok = stageDashboardUpload("standard", files);
    if (!ok) {
      setHint("PDF uniquement.");
      return;
    }
    setHint(null);
    onCloseMobile?.();
    router.push("/agentIAOCR?upload=1");
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
        {attention > 0 ? (
          <Link
            href="/agentIAOCR?suivi=1"
            onClick={onCloseMobile}
            className="inline-flex items-center gap-1 rounded-full bg-[var(--dash-lime)] px-2 py-0.5 text-[10px] font-black text-[var(--dash-ink)]"
            title="Ouvrir le suivi OCR"
          >
            <span className="h-1.5 w-1.5 rounded-full bg-[var(--dash-ink)]" aria-hidden />
            Suivi {attention > 99 ? "99+" : attention}
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
          <span className="text-[10px] text-neutral-500">OCR → dossiers élèves / personnel</span>
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
      {gate.error && gate.connected === false ? (
        <p className="text-[11px] font-medium text-red-600">{gate.error}</p>
      ) : null}
    </div>
  );
}
