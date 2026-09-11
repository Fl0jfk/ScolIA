"use client";

import { useEffect, useId, useRef, useState } from "react";

function IconCamera({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z" />
      <circle cx="12" cy="13" r="4" />
    </svg>
  );
}

function IconPaperclip({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48" />
    </svg>
  );
}

export type CompleteRequestTarget = {
  id: string;
  subject: string;
  requester: { fullName: string; email: string };
};

const MAX_FILES = 12;

function appendFiles(prev: File[], incoming: File[]): File[] {
  return [...prev, ...incoming].slice(0, MAX_FILES);
}

export default function CompleteRequestModal({
  target,
  busy,
  error,
  onClose,
  onCompleteWithoutMessage,
  onCompleteWithMessage,
}: {
  target: CompleteRequestTarget | null;
  busy?: boolean;
  error?: string | null;
  onClose: () => void;
  onCompleteWithoutMessage: (requestId: string) => void;
  onCompleteWithMessage: (requestId: string, message: string, files: File[]) => void;
}) {
  const [wantMessage, setWantMessage] = useState(true);
  const [message, setMessage] = useState("");
  const [files, setFiles] = useState<File[]>([]);
  const [previewUrls, setPreviewUrls] = useState<string[]>([]);
  const cameraInputRef = useRef<HTMLInputElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const cameraInputId = useId();
  const fileInputId = useId();

  useEffect(() => {
    if (!target) return;
    setWantMessage(true);
    setMessage("");
    setFiles([]);
  }, [target?.id]);

  useEffect(() => {
    const urls = files.map((f) => (f.type.startsWith("image/") ? URL.createObjectURL(f) : ""));
    setPreviewUrls(urls);
    return () => {
      for (const u of urls) {
        if (u) URL.revokeObjectURL(u);
      }
    };
  }, [files]);

  if (!target) return null;

  const canSend = wantMessage && (message.trim().length > 0 || files.length > 0);

  const submit = () => {
    if (wantMessage && canSend) {
      onCompleteWithMessage(target.id, message.trim(), files);
      return;
    }
    onCompleteWithoutMessage(target.id);
  };

  const onPickFiles = (list: FileList | null) => {
    if (!list?.length) return;
    setFiles((prev) => appendFiles(prev, Array.from(list)));
  };

  return (
    <div className="fixed inset-0 z-[9999] flex items-start justify-center p-4 sm:p-8 overflow-y-auto isolate">
      <button
        type="button"
        className="absolute inset-0 bg-slate-900/50 backdrop-blur-[2px]"
        aria-label="Fermer"
        disabled={busy}
        onClick={onClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="complete-request-title"
        className="relative w-full max-w-md rounded-2xl bg-white shadow-2xl border border-slate-200 p-5 sm:p-6 mt-24 sm:mt-32 mb-8"
      >
        <div className="flex items-start justify-between gap-3 mb-4">
          <div>
            <p className="text-[10px] font-black uppercase tracking-widest text-emerald-700">Clôture</p>
            <h2 id="complete-request-title" className="text-xl font-black text-slate-900 mt-0.5">
              Terminer la demande
            </h2>
            <p className="text-xs text-slate-500 mt-1 line-clamp-2">{target.subject}</p>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="shrink-0 rounded-xl px-2.5 py-1.5 text-slate-400 hover:bg-slate-100 hover:text-slate-800 font-bold text-lg leading-none disabled:opacity-50"
            aria-label="Fermer"
          >
            ×
          </button>
        </div>

        <p className="text-sm text-slate-700 leading-relaxed">
          Voulez-vous envoyer un message
          {files.length > 0 || wantMessage ? " (avec photo ou pièce jointe) " : " "}
          à <span className="font-semibold">{target.requester.fullName}</span> (
          <span className="break-all">{target.requester.email}</span>) ?
        </p>

        <div className="mt-4 space-y-2">
          <label className="flex items-start gap-2.5 rounded-xl border border-emerald-200 bg-emerald-50/70 px-3 py-2.5 cursor-pointer">
            <input
              type="radio"
              name="complete-notify"
              checked={wantMessage}
              onChange={() => setWantMessage(true)}
              disabled={busy}
              className="mt-0.5"
            />
            <span className="min-w-0">
              <span className="block text-sm font-bold text-emerald-950">Oui, envoyer un message</span>
              <span className="block text-[11px] text-emerald-900/70 mt-0.5">
                Idéal pour la maintenance : photo du travail fait + court texte au demandeur.
              </span>
            </span>
          </label>
          <label className="flex items-start gap-2.5 rounded-xl border border-slate-200 bg-slate-50 px-3 py-2.5 cursor-pointer">
            <input
              type="radio"
              name="complete-notify"
              checked={!wantMessage}
              onChange={() => setWantMessage(false)}
              disabled={busy}
              className="mt-0.5"
            />
            <span className="min-w-0">
              <span className="block text-sm font-bold text-slate-800">Non, clôturer sans message</span>
              <span className="block text-[11px] text-slate-500 mt-0.5">
                La demande passe en terminée sans texte ni fichier personnalisé.
              </span>
            </span>
          </label>
        </div>

        {wantMessage ? (
          <div className="mt-4 space-y-3 rounded-xl border border-sky-200 bg-sky-50/60 p-3">
            <label className="block text-[10px] font-black uppercase tracking-wide text-sky-900" htmlFor="complete-msg">
              Message au demandeur
            </label>
            <textarea
              id="complete-msg"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={3}
              disabled={busy}
              placeholder="Ex. : Intervention terminée, voici la photo…"
              className="w-full rounded-lg border border-sky-200 bg-white p-2 text-sm text-slate-800 disabled:opacity-60"
            />

            <div>
              <p className="text-[10px] font-bold text-sky-800 mb-2">Preuve / pièce jointe (optionnel)</p>
              <div className="grid grid-cols-2 gap-2">
                <input
                  ref={cameraInputRef}
                  id={cameraInputId}
                  type="file"
                  accept="image/*"
                  capture="environment"
                  disabled={busy || files.length >= MAX_FILES}
                  className="sr-only"
                  onChange={(e) => {
                    onPickFiles(e.target.files);
                    e.target.value = "";
                  }}
                />
                <input
                  ref={fileInputRef}
                  id={fileInputId}
                  type="file"
                  multiple
                  accept="image/*,.pdf,.doc,.docx,.xls,.xlsx,application/pdf"
                  disabled={busy || files.length >= MAX_FILES}
                  className="sr-only"
                  onChange={(e) => {
                    onPickFiles(e.target.files);
                    e.target.value = "";
                  }}
                />
                <button
                  type="button"
                  disabled={busy || files.length >= MAX_FILES}
                  onClick={() => cameraInputRef.current?.click()}
                  className="inline-flex items-center justify-center gap-2 rounded-xl border border-sky-300 bg-white px-3 py-3 text-sm font-bold text-sky-950 hover:bg-sky-100 disabled:opacity-50"
                >
                  <IconCamera className="h-4 w-4 shrink-0" />
                  Prendre une photo
                </button>
                <button
                  type="button"
                  disabled={busy || files.length >= MAX_FILES}
                  onClick={() => fileInputRef.current?.click()}
                  className="inline-flex items-center justify-center gap-2 rounded-xl border border-sky-300 bg-white px-3 py-3 text-sm font-bold text-sky-950 hover:bg-sky-100 disabled:opacity-50"
                >
                  <IconPaperclip className="h-4 w-4 shrink-0" />
                  Joindre un fichier
                </button>
              </div>
              <p className="mt-1.5 text-[10px] text-sky-800/80">
                Sur téléphone, « Prendre une photo » ouvre l’appareil photo (ex. preuve d’intervention).
              </p>
            </div>

            {files.length > 0 ? (
              <ul className="space-y-1.5">
                {files.map((f, i) => (
                  <li
                    key={`${f.name}-${f.size}-${i}`}
                    className="flex items-center gap-2 text-[11px] text-slate-600 bg-white rounded-md px-2 py-1.5 border border-sky-100"
                  >
                    {previewUrls[i] ? (
                      // eslint-disable-next-line @next/next/no-img-element -- blob preview local
                      <img
                        src={previewUrls[i]}
                        alt=""
                        className="h-10 w-10 shrink-0 rounded object-cover border border-sky-100"
                      />
                    ) : (
                      <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded bg-sky-100 text-sky-800">
                        <IconPaperclip className="h-4 w-4" />
                      </span>
                    )}
                    <span className="min-w-0 flex-1 truncate font-medium">{f.name}</span>
                    <button
                      type="button"
                      disabled={busy}
                      className="shrink-0 text-red-700 font-bold disabled:opacity-50 px-1"
                      onClick={() => setFiles((prev) => prev.filter((_, j) => j !== i))}
                      aria-label={`Retirer ${f.name}`}
                    >
                      ×
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}

        {error ? (
          <p
            role="alert"
            className="mt-4 rounded-xl border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800"
          >
            {error}
          </p>
        ) : null}

        <div className="mt-5 flex flex-col-reverse sm:flex-row sm:justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            className="rounded-xl border border-slate-200 px-4 py-2.5 text-sm font-semibold text-slate-700 hover:bg-slate-50 disabled:opacity-50"
          >
            Annuler
          </button>
          <button
            type="button"
            onClick={submit}
            disabled={busy || (wantMessage && !canSend)}
            className="rounded-xl bg-emerald-600 px-4 py-2.5 text-sm font-bold text-white hover:bg-emerald-700 disabled:opacity-50"
          >
            {busy
              ? "Envoi…"
              : wantMessage
                ? "Clôturer et envoyer"
                : "Clôturer sans message"}
          </button>
        </div>
      </div>
    </div>
  );
}
