"use client";

import { useCallback, useRef, useState, type DragEvent } from "react";

const ACCEPT = ".pdf,.doc,.docx,.xls,.xlsx,.png,.jpg,.jpeg,.webp,.gif,.heic";

type Props = {
  title: string;
  description?: string;
  establishmentId: string;
  itemId: string;
  variant?: "default" | "internat";
};

export default function RentreeSubmissionCard({
  title,
  description,
  establishmentId,
  itemId,
  variant = "default",
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const honeypotRef = useRef<HTMLInputElement>(null);
  const [file, setFile] = useState<File | null>(null);
  const [email, setEmail] = useState("");
  const [studentName, setStudentName] = useState("");
  const [dragOver, setDragOver] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const internat = variant === "internat";
  const cardClass = internat
    ? "rounded-3xl border border-white/15 bg-white p-5 shadow-lg"
    : "rounded-3xl border border-slate-200/70 bg-white p-5 shadow-sm";
  const ctaClass = internat
    ? "bg-amber-600 hover:bg-amber-700"
    : "bg-indigo-600 hover:bg-indigo-700";

  const pickFile = useCallback((next: File | null) => {
    setError(null);
    setDone(null);
    setFile(next);
  }, []);

  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const next = e.dataTransfer.files?.[0];
    if (next) pickFile(next);
  };

  async function submit() {
    setError(null);
    if (!file) {
      setError("Déposez un fichier.");
      return;
    }
    if (!email.trim()) {
      setError("Indiquez votre e-mail pour confirmer l’envoi.");
      return;
    }
    setBusy(true);
    try {
      const form = new FormData();
      form.set("establishmentId", establishmentId);
      form.set("itemId", itemId);
      form.set("email", email.trim());
      if (studentName.trim()) form.set("studentName", studentName.trim());
      form.set("file", file);
      form.set("website", honeypotRef.current?.value || "");
      const res = await fetch("/api/rentree/submissions/create", {
        method: "POST",
        body: form,
      });
      const json = (await res.json().catch(() => ({}))) as { error?: string; message?: string };
      if (!res.ok) {
        setError(json.error || "Envoi impossible.");
        return;
      }
      setDone(
        json.message ||
          "Un e-mail vient de vous être envoyé. Cliquez sur le lien pour transmettre le document.",
      );
      setFile(null);
    } catch {
      setError("Envoi impossible. Réessayez.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className={cardClass}>
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <p className="font-black text-slate-900 text-lg leading-snug">{title}</p>
          {description ? (
            <p className="text-sm text-slate-500 mt-1 leading-relaxed">{description}</p>
          ) : (
            <p className="text-sm text-slate-500 mt-1 leading-relaxed">
              Déposez le document, indiquez votre e-mail, puis confirmez via le lien reçu.
            </p>
          )}
        </div>
        <span className="flex items-center gap-2 shrink-0">
          <span className="text-lg" aria-hidden>
            📤
          </span>
          <span className="text-[10px] font-black uppercase tracking-widest px-2.5 py-1 rounded-full border border-slate-200 text-slate-600 bg-slate-50">
            Dépôt
          </span>
        </span>
      </div>

      {done ? (
        <p className="mt-4 rounded-2xl bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800 leading-relaxed">
          {done}
        </p>
      ) : (
        <div className="mt-4 space-y-3">
          <button
            type="button"
            onDragOver={(e) => {
              e.preventDefault();
              setDragOver(true);
            }}
            onDragLeave={() => setDragOver(false)}
            onDrop={onDrop}
            onClick={() => inputRef.current?.click()}
            className={`w-full rounded-2xl border-2 border-dashed px-4 py-6 text-center transition ${
              dragOver
                ? "border-indigo-400 bg-indigo-50"
                : "border-slate-200 bg-slate-50 hover:border-slate-300"
            }`}
          >
            <p className="text-sm font-bold text-slate-700">
              {file ? file.name : "Glissez le fichier ici, ou cliquez pour choisir"}
            </p>
            <p className="text-[11px] text-slate-400 mt-1">PDF, image, Word ou Excel — 12 Mo max.</p>
          </button>
          <input
            ref={inputRef}
            type="file"
            accept={ACCEPT}
            className="hidden"
            onChange={(e) => {
              pickFile(e.target.files?.[0] ?? null);
              e.target.value = "";
            }}
          />
          <label className="absolute -left-[9999px] h-0 w-0 overflow-hidden" aria-hidden>
            Site web
            <input ref={honeypotRef} type="text" name="website" tabIndex={-1} autoComplete="off" defaultValue="" />
          </label>
          <label className="block">
            <span className="text-[10px] font-black uppercase tracking-widest text-slate-400">
              Nom de l&apos;élève (recommandé)
            </span>
            <input
              type="text"
              value={studentName}
              onChange={(e) => setStudentName(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              placeholder="Prénom et nom"
              autoComplete="name"
            />
          </label>
          <label className="block">
            <span className="text-[10px] font-black uppercase tracking-widest text-slate-400">
              Votre e-mail
            </span>
            <input
              type="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              placeholder="ex. parent@email.fr"
              autoComplete="email"
            />
          </label>
          {error ? <p className="text-sm font-medium text-rose-600">{error}</p> : null}
          <button
            type="button"
            disabled={busy}
            onClick={() => void submit()}
            className={`w-full rounded-full px-4 py-2.5 text-sm font-black text-white transition disabled:opacity-60 ${ctaClass}`}
          >
            {busy ? "Envoi…" : "Envoyer et recevoir le lien de confirmation"}
          </button>
        </div>
      )}
    </div>
  );
}
