"use client";

import { useEffect, useState } from "react";

export type CompleteRequestTarget = {
  id: string;
  subject: string;
  requester: { fullName: string; email: string };
};

export default function CompleteRequestModal({
  target,
  busy,
  onClose,
  onCompleteWithoutMessage,
  onCompleteWithMessage,
}: {
  target: CompleteRequestTarget | null;
  busy?: boolean;
  onClose: () => void;
  onCompleteWithoutMessage: (requestId: string) => void;
  onCompleteWithMessage: (requestId: string, message: string, files: File[]) => void;
}) {
  const [wantMessage, setWantMessage] = useState(true);
  const [message, setMessage] = useState("");
  const [files, setFiles] = useState<File[]>([]);

  useEffect(() => {
    if (!target) return;
    setWantMessage(true);
    setMessage("");
    setFiles([]);
  }, [target?.id]);

  if (!target) return null;

  const canSend = wantMessage && (message.trim().length > 0 || files.length > 0);

  const submit = () => {
    if (wantMessage && canSend) {
      onCompleteWithMessage(target.id, message.trim(), files);
      return;
    }
    onCompleteWithoutMessage(target.id);
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
          {files.length > 0 || wantMessage ? " (avec pièce jointe éventuelle) " : " "}
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
                Le demandeur reçoit un e-mail de clôture avec votre texte et les fichiers joints.
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
          <div className="mt-4 space-y-2 rounded-xl border border-sky-200 bg-sky-50/60 p-3">
            <label className="block text-[10px] font-black uppercase tracking-wide text-sky-900" htmlFor="complete-msg">
              Message au demandeur
            </label>
            <textarea
              id="complete-msg"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              rows={3}
              disabled={busy}
              placeholder="Ex. : Voici le document demandé, n’hésitez pas à nous recontacter…"
              className="w-full rounded-lg border border-sky-200 bg-white p-2 text-sm text-slate-800 disabled:opacity-60"
            />
            <label className="block text-[10px] font-bold text-sky-800">Pièce(s) jointe(s) (optionnel)</label>
            <input
              type="file"
              multiple
              accept="image/*,.pdf,.doc,.docx,.xls,.xlsx,application/pdf"
              disabled={busy}
              className="w-full text-[11px] text-slate-600 file:mr-2 file:rounded file:border-0 file:bg-sky-200 file:px-2 file:py-1"
              onChange={(e) => {
                const list = e.target.files ? Array.from(e.target.files) : [];
                setFiles((prev) => [...prev, ...list].slice(0, 12));
                e.target.value = "";
              }}
            />
            {files.length > 0 ? (
              <ul className="space-y-1">
                {files.map((f, i) => (
                  <li
                    key={`${f.name}-${i}`}
                    className="flex justify-between gap-2 text-[11px] text-slate-600 bg-white rounded-md px-2 py-1 border border-sky-100"
                  >
                    <span className="truncate">{f.name}</span>
                    <button
                      type="button"
                      disabled={busy}
                      className="shrink-0 text-red-700 font-bold disabled:opacity-50"
                      onClick={() => setFiles((prev) => prev.filter((_, j) => j !== i))}
                    >
                      ×
                    </button>
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
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
