"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import { SettingsSection } from "@/app/components/settings/SettingsChrome";
import { dash } from "@/app/lib/dashboard-brand";

const IMAGE_RE = /\.(jpe?g|png|webp|gif)$/i;
/** Lots d’upload (réception seule) — le matching se fait ensuite en batch serveur. */
const MAX_FILES_PER_CHUNK = 40;
const MAX_BYTES_PER_CHUNK = 10 * 1024 * 1024;

type UploadPhase = "idle" | "uploading" | "processing" | "done" | "error";

type JobStatusResponse = {
  jobId?: string;
  status?: string;
  percent?: number;
  label?: string;
  total?: number;
  matched?: number;
  updated?: number;
  unmatched?: string[];
  errors?: string[];
  error?: string;
  message?: string;
};

type ProgressState = {
  phase: UploadPhase;
  totalFiles: number;
  uploadedFiles: number;
  jobId: string | null;
  percent: number;
  label: string;
  updated: number;
  matched: number;
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

function newClientJobId(): string {
  const rand =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID().replace(/-/g, "").slice(0, 12)
      : Math.random().toString(36).slice(2, 14);
  return `photos_${Date.now()}_${rand}`;
}

async function uploadChunk(jobId: string, files: File[]): Promise<{ jobId: string; received: number }> {
  const fd = new FormData();
  fd.append("jobId", jobId);
  files.forEach((f, i) => fd.append("files", f, f.name || `photo-${i}.jpg`));
  const res = await fetch("/api/eleves/photos/bulk/upload", { method: "POST", body: fd });
  const data = (await res.json().catch(() => ({}))) as {
    jobId?: string;
    received?: number;
    error?: string;
  };
  if (!res.ok) {
    throw new Error(data.error || `Envoi impossible (HTTP ${res.status}).`);
  }
  return { jobId: data.jobId || jobId, received: data.received || files.length };
}

async function startJob(jobId: string): Promise<JobStatusResponse> {
  const res = await fetch("/api/eleves/photos/bulk/start", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jobId }),
  });
  const data = (await res.json().catch(() => ({}))) as JobStatusResponse;
  if (!res.ok) {
    throw new Error(data.error || `Démarrage impossible (HTTP ${res.status}).`);
  }
  return data;
}

async function fetchJob(jobId: string): Promise<JobStatusResponse> {
  const res = await fetch(`/api/eleves/photos/bulk/job?jobId=${encodeURIComponent(jobId)}`, {
    cache: "no-store",
  });
  const data = (await res.json().catch(() => ({}))) as JobStatusResponse;
  if (!res.ok) {
    throw new Error(data.error || `Suivi impossible (HTTP ${res.status}).`);
  }
  return data;
}

/** Coup de pouce si l’utilisateur reste sur la page (reprise d’un segment). */
async function kickProcess(jobId: string): Promise<void> {
  await fetch("/api/eleves/photos/bulk/process", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jobId }),
  }).catch(() => undefined);
}

export default function ElevePhotosBulkPanel() {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragging, setDragging] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<ProgressState | null>(null);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const uploadPercent = useMemo(() => {
    if (!progress || progress.totalFiles === 0) return 0;
    if (progress.phase === "uploading") {
      return Math.min(100, Math.round((progress.uploadedFiles / progress.totalFiles) * 100));
    }
    return progress.percent;
  }, [progress]);

  const stopPoll = useCallback(() => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  }, []);

  const startPoll = useCallback(
    (jobId: string) => {
      stopPoll();
      pollRef.current = setInterval(() => {
        void (async () => {
          try {
            const job = await fetchJob(jobId);
            setProgress((prev) =>
              prev
                ? {
                    ...prev,
                    phase:
                      job.status === "completed"
                        ? "done"
                        : job.status === "failed"
                          ? "error"
                          : "processing",
                    percent: job.percent ?? prev.percent,
                    label: job.label || prev.label,
                    updated: job.updated ?? prev.updated,
                    matched: job.matched ?? prev.matched,
                    unmatched: Array.isArray(job.unmatched) ? job.unmatched : prev.unmatched,
                    errors: Array.isArray(job.errors) ? job.errors : prev.errors,
                  }
                : prev,
            );
            if (job.status === "processing" || job.status === "queued") {
              void kickProcess(jobId);
            }
            if (job.status === "completed" || job.status === "failed") {
              stopPoll();
              setBusy(false);
              if (job.status === "completed") {
                setMessage(
                  job.label ||
                    `${job.updated ?? 0} photo(s) enregistrée(s) (anciennes remplacées).`,
                );
              } else {
                setError(job.error || job.label || "Traitement en échec.");
              }
            }
          } catch {
            /* ignore transient poll errors */
          }
        })();
      }, 2500);
    },
    [stopPoll],
  );

  useEffect(() => () => stopPoll(), [stopPoll]);

  const processFiles = useCallback(
    async (raw: File[] | FileList | null) => {
      if (!raw) return;
      const files = Array.from(raw).filter(isImageFile);
      if (!files.length) {
        setError("Aucune image valide (jpg, jpeg, png, webp, gif).");
        return;
      }

      stopPoll();
      setBusy(true);
      setError(null);
      setMessage(null);

      const jobId = newClientJobId();
      const next: ProgressState = {
        phase: "uploading",
        totalFiles: files.length,
        uploadedFiles: 0,
        jobId,
        percent: 0,
        label: "Envoi des fichiers vers le serveur…",
        updated: 0,
        matched: 0,
        unmatched: [],
        errors: [],
      };
      setProgress({ ...next });

      const chunks = chunkFiles(files);
      try {
        for (const chunk of chunks) {
          const result = await uploadChunk(jobId, chunk);
          next.jobId = result.jobId;
          next.uploadedFiles += chunk.length;
          next.label = `Envoi ${next.uploadedFiles}/${next.totalFiles}…`;
          setProgress({ ...next });
        }

        const started = await startJob(next.jobId!);
        next.phase = "processing";
        next.percent = 0;
        next.label =
          started.message ||
          "Upload terminé — association en cours sur le serveur. Vous pouvez quitter cette page.";
        setProgress({ ...next });
        setMessage(next.label);
        setBusy(false);
        if (inputRef.current) inputRef.current.value = "";
        startPoll(next.jobId!);
      } catch (e: unknown) {
        const msg = e instanceof Error ? e.message : "Import impossible.";
        setError(msg);
        setProgress((prev) =>
          prev ? { ...prev, phase: "error", label: msg, errors: [...prev.errors, msg] } : prev,
        );
        setBusy(false);
        if (inputRef.current) inputRef.current.value = "";
      }
    },
    [startPoll, stopPoll],
  );

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragging(false);
    if (busy || progress?.phase === "uploading") return;
    void processFiles(e.dataTransfer.files);
  };

  const uploading = progress?.phase === "uploading" || busy;

  return (
    <SettingsSection
      title="Photos des élèves"
      icon="🖼️"
      description={
        <>
          Déposez jusqu’à 1500 photos (ou plus). Les fichiers doivent être nommés{" "}
          <strong className="font-semibold text-slate-800">NOM Prenom.jpg</strong> (espaces,{" "}
          <code className="rounded bg-white/70 px-1">_</code> ou{" "}
          <code className="rounded bg-white/70 px-1">-</code> acceptés). Une fois l’envoi terminé, le
          serveur associe les photos en arrière-plan et <strong>remplace</strong> les photos déjà
          présentes (nouvelle année scolaire). Vous pouvez quitter la page après l’upload.
        </>
      }
    >
      <input
        ref={inputRef}
        type="file"
        accept="image/jpeg,image/png,image/webp,image/gif,.jpg,.jpeg,.png,.webp,.gif"
        multiple
        className="hidden"
        disabled={uploading}
        onChange={(e) => void processFiles(e.target.files)}
      />

      <div
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            if (!uploading) inputRef.current?.click();
          }
        }}
        onClick={() => {
          if (!uploading) inputRef.current?.click();
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
        } ${uploading ? "pointer-events-none opacity-70" : ""}`}
      >
        <span className="text-3xl" aria-hidden>
          {uploading ? "⏳" : progress?.phase === "processing" ? "⚙️" : "📁"}
        </span>
        <p className={`text-sm font-semibold ${dash.ink}`}>
          {progress?.phase === "uploading"
            ? "Envoi vers le serveur…"
            : progress?.phase === "processing"
              ? "Traitement serveur en cours (vous pouvez quitter)"
              : "Glissez-déposez les photos ici"}
        </p>
        <p className={`max-w-md text-xs ${dash.textMid}`}>
          L’envoi se fait par lots (~{MAX_FILES_PER_CHUNK} fichiers). Ensuite le matching et
          l’écrasement des anciennes photos tournent en batch sur le serveur.
        </p>
        {!uploading ? (
          <span className="mt-1 inline-flex rounded-full bg-[color:var(--dash-primary)] px-4 py-2 text-xs font-bold text-white shadow-sm">
            Choisir les photos
          </span>
        ) : null}
      </div>

      {progress ? (
        <div className="space-y-2 rounded-2xl border border-white/70 bg-white/70 p-4">
          <div className="flex items-center justify-between text-xs font-semibold text-slate-700">
            <span>
              {progress.phase === "uploading"
                ? `Envoi : ${progress.uploadedFiles} / ${progress.totalFiles}`
                : progress.label}
            </span>
            <span>{uploadPercent}%</span>
          </div>
          <div className="h-2.5 overflow-hidden rounded-full bg-slate-200/80">
            <div
              className="h-full rounded-full bg-[color:var(--dash-mid)] transition-[width] duration-300"
              style={{ width: `${uploadPercent}%` }}
            />
          </div>
          {progress.phase !== "uploading" ? (
            <p className={`text-xs ${dash.textMid}`}>
              Associées : {progress.updated} · Reconnues : {progress.matched} · Non reconnues :{" "}
              {progress.unmatched.length}
              {progress.jobId ? (
                <span className="ml-2 font-mono text-[10px] text-slate-400">
                  job {progress.jobId}
                </span>
              ) : null}
            </p>
          ) : null}
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
