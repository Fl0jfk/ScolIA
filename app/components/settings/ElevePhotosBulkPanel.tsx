"use client";

import { useCallback, useMemo, useRef, useState, type DragEvent } from "react";
import { SettingsSection } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

const IMAGE_RE = /\.(jpe?g|png|webp|gif)$/i;
/** Lots raisonnables pour éviter les timeouts / limites body Next.js (~1500 photos). */
const MAX_FILES_PER_CHUNK = 40;
const MAX_BYTES_PER_CHUNK = 10 * 1024 * 1024;

type BulkResponse = {
  matched?: number;
  unmatched?: string[];
  updated?: number;
  message?: string;
  error?: string;
};

type ProgressState = {
  total: number;
  done: number;
  matched: number;
  updated: number;
  unmatched: string[];
  errors: string[];
};

function isImageFile(file: File): boolean {
  return IMAGE_RE.test(file.name) || file.type.startsWith("image/");
}

function chunkFiles(files: File[]): File[][] {
  const chunks: File[][] = [];
  let current: File[] = [];
  let currentBytes = 0;

  for (const file of files) {
    const size = file.size || 0;
    const wouldExceedCount = current.length >= MAX_FILES_PER_CHUNK;
    const wouldExceedBytes =
      current.length > 0 && currentBytes + size > MAX_BYTES_PER_CHUNK;
    if (wouldExceedCount || wouldExceedBytes) {
      chunks.push(current);
      current = [];
      currentBytes = 0;
    }
    current.push(file);
    currentBytes += size;
  }
  if (current.length) chunks.push(current);
  return chunks;
}

async function uploadChunk(files: File[]): Promise<BulkResponse> {
  const fd = new FormData();
  files.forEach((f, i) => fd.append("files", f, f.name || `photo-${i}.jpg`));
  const res = await fetch("/api/eleves/photos/bulk", { method: "POST", body: fd });
  const data = (await res.json().catch(() => ({}))) as BulkResponse;
  if (!res.ok) {
    throw new Error(data.error || `Import impossible (HTTP ${res.status}).`);
  }
  return data;
}

export default function ElevePhotosBulkPanel() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressState | null>(null);

  const percent = useMemo(() => {
    if (!progress || progress.total === 0) return 0;
    return Math.min(100, Math.round((progress.done / progress.total) * 100));
  }, [progress]);

  const processFiles = useCallback(async (raw: File[] | FileList | null) => {
    if (!raw) return;
    const files = Array.from(raw).filter(isImageFile);
    if (!files.length) {
      setError("Aucune image valide (jpg, jpeg, png, webp, gif).");
      return;
    }

    setBusy(true);
    setError(null);
    setMessage(null);
    const next: ProgressState = {
      total: files.length,
      done: 0,
      matched: 0,
      updated: 0,
      unmatched: [],
      errors: [],
    };
    setProgress({ ...next });

    const chunks = chunkFiles(files);
    try {
      for (const chunk of chunks) {
        try {
          const data = await uploadChunk(chunk);
          next.matched += data.matched || 0;
          next.updated += data.updated || 0;
          if (Array.isArray(data.unmatched)) {
            next.unmatched.push(...data.unmatched);
          }
        } catch (e: unknown) {
          const msg = e instanceof Error ? e.message : "Erreur sur un lot";
          next.errors.push(msg);
          next.unmatched.push(...chunk.map((f) => f.name));
        }
        next.done += chunk.length;
        setProgress({ ...next, unmatched: [...next.unmatched], errors: [...next.errors] });
      }

      const unmatchedNote =
        next.unmatched.length > 0
          ? ` ${next.unmatched.length} fichier(s) non associé(s).`
          : "";
      const errNote =
        next.errors.length > 0 ? ` ${next.errors.length} lot(s) en erreur.` : "";
      setMessage(
        `${next.updated} photo(s) associée(s) sur ${next.total} fichier(s).${unmatchedNote}${errNote}`,
      );
      if (next.errors.length && next.updated === 0) {
        setError(next.errors[0] || "Import impossible.");
      }
    } finally {
      setBusy(false);
      if (inputRef.current) inputRef.current.value = "";
    }
  }, []);

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragging(false);
    if (busy) return;
    void processFiles(e.dataTransfer.files);
  };

  return (
    <SettingsSection
      title="Photos des élèves"
      icon="🖼️"
      description={
        <>
          Déposez jusqu’à 1500 photos (ou plus) d’un coup. Les fichiers doivent être nommés{" "}
          <strong className="font-semibold text-slate-800">NOM Prenom.jpg</strong> (espaces,{" "}
          <code className="rounded bg-white/70 px-1">_</code> ou <code className="rounded bg-white/70 px-1">-</code>{" "}
          acceptés). Association automatique sur le référentiel élèves (tous les élèves, pas seulement les
          internes).
        </>
      }
    >
      <input
        ref={inputRef}
        type="file"
        accept="image/jpeg,image/png,image/webp,image/gif,.jpg,.jpeg,.png,.webp,.gif"
        multiple
        className="hidden"
        disabled={busy}
        onChange={(e) => void processFiles(e.target.files)}
      />

      <div
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            if (!busy) inputRef.current?.click();
          }
        }}
        onClick={() => {
          if (!busy) inputRef.current?.click();
        }}
        onDragEnter={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={(e) => {
          e.preventDefault();
          if (e.currentTarget === e.target) setDragging(false);
        }}
        onDrop={onDrop}
        className={`flex min-h-[220px] cursor-pointer flex-col items-center justify-center gap-3 rounded-[1.25rem] border-2 border-dashed px-6 py-10 text-center transition ${
          dragging
            ? "border-[color:var(--dash-mid)] bg-[color:var(--dash-soft)]/60"
            : "border-slate-300/80 bg-white/55 hover:border-[color:var(--dash-mid)]/60 hover:bg-white/75"
        } ${busy ? "pointer-events-none opacity-70" : ""}`}
      >
        <span className="text-3xl" aria-hidden>
          {busy ? "⏳" : "📁"}
        </span>
        <p className={`text-sm font-semibold ${dash.ink}`}>
          {busy ? "Import en cours…" : "Glissez-déposez les photos ici"}
        </p>
        <p className={`max-w-md text-xs ${dash.textMid}`}>
          ou cliquez pour sélectionner un dossier / plusieurs fichiers. L’envoi se fait par lots automatiques
          (environ {MAX_FILES_PER_CHUNK} photos) pour rester stable jusqu’à 1500+ images.
        </p>
        {!busy ? (
          <span className="mt-1 inline-flex rounded-full bg-[color:var(--dash-primary)] px-4 py-2 text-xs font-bold text-white shadow-sm">
            Choisir les photos
          </span>
        ) : null}
      </div>

      {progress ? (
        <div className="space-y-2 rounded-2xl border border-white/70 bg-white/70 p-4">
          <div className="flex items-center justify-between text-xs font-semibold text-slate-700">
            <span>
              Progression : {progress.done} / {progress.total}
            </span>
            <span>{percent}%</span>
          </div>
          <div className="h-2.5 overflow-hidden rounded-full bg-slate-200/80">
            <div
              className="h-full rounded-full bg-[color:var(--dash-mid)] transition-[width] duration-300"
              style={{ width: `${percent}%` }}
            />
          </div>
          <p className={`text-xs ${dash.textMid}`}>
            Associées : {progress.updated} · Reconnues : {progress.matched} · Non reconnues :{" "}
            {progress.unmatched.length}
          </p>
        </div>
      ) : null}

      {message ? (
        <p className="rounded-2xl border border-emerald-200/80 bg-emerald-50/90 px-4 py-3 text-sm text-emerald-900">
          {message}
        </p>
      ) : null}
      {error ? (
        <p className="rounded-2xl border border-rose-200/80 bg-rose-50/90 px-4 py-3 text-sm text-rose-900">
          {error}
        </p>
      ) : null}

      {progress && progress.unmatched.length > 0 ? (
        <details className="rounded-2xl border border-amber-200/80 bg-amber-50/70 p-4">
          <summary className="cursor-pointer text-sm font-semibold text-amber-950">
            Fichiers non associés ({progress.unmatched.length})
          </summary>
          <ul className="mt-3 max-h-56 space-y-1 overflow-y-auto text-xs text-amber-950/90">
            {progress.unmatched.slice(0, 200).map((name) => (
              <li key={name} className="truncate font-mono">
                {name}
              </li>
            ))}
            {progress.unmatched.length > 200 ? (
              <li className="italic">… et {progress.unmatched.length - 200} autre(s)</li>
            ) : null}
          </ul>
          <p className={`mt-2 text-xs ${dash.textMid}`}>
            Vérifiez l’orthographe du nom/prénom (identique au référentiel) et le format{" "}
            <span className="font-semibold">NOM Prenom.ext</span>.
          </p>
        </details>
      ) : null}
    </SettingsSection>
  );
}
