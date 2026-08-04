"use client";

import { useRef, useState } from "react";

const MAX_FILES = 2;

export default function ParentDemandeForm() {
  const fileRef = useRef<HTMLInputElement>(null);
  const [fullName, setFullName] = useState("");
  const [email, setEmail] = useState("");
  const [phone, setPhone] = useState("");
  const [description, setDescription] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [honeypot, setHoneypot] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const canSubmit =
    fullName.trim().length > 1 &&
    /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim()) &&
    description.trim().length >= 15 &&
    !submitting;

  const onPickFiles = (list: FileList | null) => {
    if (!list) return;
    setFiles((prev) => [...prev, ...Array.from(list)].slice(0, MAX_FILES));
  };

  const submit = async () => {
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    setSuccess(null);
    try {
      const fd = new FormData();
      fd.append("fullName", fullName.trim());
      fd.append("email", email.trim());
      fd.append("phone", phone.trim());
      fd.append("description", description.trim());
      fd.append("website", honeypot);
      for (const f of files) fd.append("files", f);

      const res = await fetch("/api/requests/parent-portal", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Envoi impossible.");

      setSuccess(
        data.message ||
          "Un e-mail de confirmation vous a été envoyé. Cliquez sur le lien pour valider votre demande.",
      );
      setDescription("");
      setFiles([]);
      if (fileRef.current) fileRef.current.value = "";
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur inattendue.");
    } finally {
      setSubmitting(false);
    }
  };

  if (success) {
    return (
      <div className="rounded-2xl border border-emerald-200 bg-emerald-50/80 p-6 text-center">
        <p className="text-lg font-black text-emerald-900">Presque terminé</p>
        <p className="mt-2 text-sm leading-relaxed text-emerald-900/80">{success}</p>
        <button
          type="button"
          onClick={() => setSuccess(null)}
          className="mt-5 text-sm font-bold text-emerald-800 underline"
        >
          Envoyer une autre demande
        </button>
      </div>
    );
  }

  return (
    <div className="relative space-y-5 rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
      {/* Honeypot — caché des humains */}
      <div className="pointer-events-none absolute left-0 top-0 -z-10 h-0 w-0 overflow-hidden opacity-0" aria-hidden>
        <label>
          Site web
          <input
            tabIndex={-1}
            autoComplete="off"
            value={honeypot}
            onChange={(e) => setHoneypot(e.target.value)}
          />
        </label>
      </div>

      <div>
        <label htmlFor="parent-name" className="mb-1.5 block text-sm font-bold text-slate-700">
          Nom et prénom
        </label>
        <input
          id="parent-name"
          type="text"
          className="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
          value={fullName}
          onChange={(e) => setFullName(e.target.value)}
          placeholder="Jean Dupont"
          autoComplete="name"
        />
      </div>

      <div>
        <label htmlFor="parent-email" className="mb-1.5 block text-sm font-bold text-slate-700">
          E-mail
        </label>
        <input
          id="parent-email"
          type="email"
          className="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="votre@email.fr"
          autoComplete="email"
        />
        <p className="mt-1 text-xs text-slate-400">
          Un lien de confirmation sera envoyé à cette adresse (anti-spam).
        </p>
      </div>

      <div>
        <label htmlFor="parent-phone" className="mb-1.5 block text-sm font-bold text-slate-700">
          Téléphone <span className="font-normal text-slate-400">(facultatif)</span>
        </label>
        <input
          id="parent-phone"
          type="tel"
          className="w-full rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
          value={phone}
          onChange={(e) => setPhone(e.target.value)}
          placeholder="06 …"
          autoComplete="tel"
        />
      </div>

      <div>
        <label htmlFor="parent-desc" className="mb-1.5 block text-sm font-bold text-slate-700">
          Votre demande
        </label>
        <textarea
          id="parent-desc"
          rows={6}
          className="min-h-[140px] w-full resize-y rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          placeholder="Décrivez votre demande pour l’établissement…"
        />
        <p className="mt-1 text-xs text-slate-400">Minimum 15 caractères.</p>
      </div>

      <div>
        <label className="mb-1.5 block text-sm font-bold text-slate-700">
          Pièce jointe <span className="font-normal text-slate-400">(facultatif)</span>
        </label>
        <p className="mb-2 text-xs text-slate-500">
          Photo ou PDF — {MAX_FILES} fichiers max.
        </p>
        <input
          ref={fileRef}
          type="file"
          multiple
          accept="image/*,application/pdf,.pdf"
          className="block w-full text-sm text-slate-600 file:mr-3 file:rounded-lg file:border-0 file:bg-amber-50 file:px-3 file:py-2 file:font-semibold file:text-amber-900"
          onChange={(e) => onPickFiles(e.target.files)}
        />
        {files.length > 0 ? (
          <ul className="mt-2 space-y-1">
            {files.map((f, i) => (
              <li
                key={`${f.name}-${i}`}
                className="flex items-center justify-between gap-2 rounded-lg bg-slate-50 px-3 py-1.5 text-sm"
              >
                <span className="truncate">{f.name}</span>
                <button
                  type="button"
                  className="shrink-0 text-xs font-bold text-rose-600"
                  onClick={() => setFiles((prev) => prev.filter((_, j) => j !== i))}
                >
                  Retirer
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </div>

      {error ? (
        <p className="rounded-xl bg-rose-50 px-3 py-2 text-sm font-medium text-rose-800">{error}</p>
      ) : null}

      <button
        type="button"
        disabled={!canSubmit}
        onClick={() => void submit()}
        className="w-full rounded-xl bg-slate-900 py-3 text-sm font-black text-white transition hover:bg-black disabled:opacity-50"
      >
        {submitting ? "Envoi…" : "Envoyer ma demande"}
      </button>
    </div>
  );
}
