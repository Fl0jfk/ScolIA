"use client";

import { useEffect, useState } from "react";
import type { StageConvention } from "@/app/lib/stage-types";
import { isStageSignatureFullyValidated } from "@/app/lib/stage-types";

export type ManualSignaturePayload = {
  note?: string;
  file: File | null;
  presentOnPaperSignatureIds: string[];
};

const MAX_BYTES = 15 * 1024 * 1024;
const ACCEPT = "application/pdf,image/jpeg,image/png,.pdf,.jpg,.jpeg,.png";

/**
 * Validation manuelle d'une signature (papier / hors plateforme) avec dépôt
 * facultatif du document rendu. Ce document devient la convention principale ;
 * on indique quelles autres signatures y figurent déjà (rien n'est ajouté par-dessus).
 */
export default function StageManualSignatureModal({
  convention,
  signatureId,
  busy,
  onCancel,
  onSubmit,
}: {
  convention: StageConvention;
  signatureId: string;
  busy: boolean;
  onCancel: () => void;
  onSubmit: (payload: ManualSignaturePayload) => void;
}) {
  const sig = convention.signatures.find((s) => s.id === signatureId);
  const others = convention.signatures.filter((s) => s.id !== signatureId);
  const [note, setNote] = useState("Signé hors plateforme");
  const [file, setFile] = useState<File | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [fileError, setFileError] = useState<string | null>(null);
  const [present, setPresent] = useState<Set<string>>(new Set());

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape" && !busy) onCancel();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [busy, onCancel]);

  function pickFile(f: File | null | undefined) {
    setFileError(null);
    if (!f) return;
    const okType = /\.(pdf|jpe?g|png)$/i.test(f.name) || /^(application\/pdf|image\/(jpeg|png))$/.test(f.type);
    if (!okType) {
      setFileError("PDF, JPG ou PNG uniquement.");
      return;
    }
    if (f.size > MAX_BYTES) {
      setFileError("Fichier trop lourd (15 Mo maximum).");
      return;
    }
    setFile(f);
  }

  function toggle(id: string) {
    setPresent((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  const allChecked = others.length > 0 && others.every((s) => present.has(s.id));

  if (!sig) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-end justify-center bg-black/40 p-3 sm:items-center sm:p-6">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="stage-manual-sign-title"
        className="max-h-[92vh] w-full max-w-lg overflow-y-auto rounded-2xl border border-stone-200 bg-white shadow-xl"
      >
        <div className="space-y-4 p-5">
          <div>
            <h2 id="stage-manual-sign-title" className="text-base font-bold text-stone-900">
              Valider manuellement — {sig.label}
            </h2>
            <p className="mt-1 text-xs leading-relaxed text-stone-600">
              Signature reçue sur papier ou hors plateforme. Le lien de signature sera invalidé.
            </p>
          </div>

          <label className="block text-sm">
            <span className="font-semibold text-stone-800">Note (facultatif)</span>
            <input
              type="text"
              value={note}
              maxLength={300}
              onChange={(e) => setNote(e.target.value)}
              className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
            />
          </label>

          <div>
            <p className="text-sm font-semibold text-stone-800">Document rendu (facultatif)</p>
            <p className="mt-0.5 text-xs text-stone-600">
              La convention signée qu&apos;on vous a remise. Elle remplacera le document principal et
              partira avec le dossier (dossier élève, OneDrive, e-mails).
            </p>
            <label
              onDragOver={(e) => {
                e.preventDefault();
                setDragOver(true);
              }}
              onDragLeave={() => setDragOver(false)}
              onDrop={(e) => {
                e.preventDefault();
                setDragOver(false);
                pickFile(e.dataTransfer.files?.[0]);
              }}
              className={`mt-2 flex cursor-pointer flex-col items-center justify-center rounded-xl border-2 border-dashed px-4 py-5 text-center text-sm ${
                dragOver ? "border-[#2F6B4A] bg-emerald-50" : "border-stone-300 bg-stone-50"
              }`}
            >
              <input
                type="file"
                accept={ACCEPT}
                className="sr-only"
                onChange={(e) => pickFile(e.target.files?.[0])}
              />
              <span className="font-medium text-stone-700">
                {file ? file.name : "Déposer le fichier ici ou cliquer pour choisir"}
              </span>
              <span className="mt-1 text-xs text-stone-500">PDF, JPG ou PNG — 15 Mo maximum</span>
            </label>
            {file ? (
              <button
                type="button"
                onClick={() => {
                  setFile(null);
                  setPresent(new Set());
                }}
                className="mt-1 text-xs font-semibold text-rose-700 underline"
              >
                Retirer le fichier
              </button>
            ) : null}
            {fileError ? <p className="mt-1 text-xs font-semibold text-rose-700">{fileError}</p> : null}
          </div>

          {file && others.length > 0 ? (
            <div className="rounded-xl border border-stone-200 bg-stone-50 px-3 py-3">
              <div className="flex items-center justify-between gap-2">
                <p className="text-sm font-semibold text-stone-800">
                  Quelles autres signatures sont déjà sur ce document ?
                </p>
                <button
                  type="button"
                  onClick={() =>
                    setPresent(allChecked ? new Set() : new Set(others.map((s) => s.id)))
                  }
                  className="shrink-0 text-xs font-semibold text-[#2F6B4A] underline"
                >
                  {allChecked ? "Tout décocher" : "Toutes présentes"}
                </button>
              </div>
              <ul className="mt-2 space-y-1.5">
                {others.map((s) => {
                  const done = isStageSignatureFullyValidated(s);
                  return (
                    <li key={s.id}>
                      <label className="flex items-start gap-2 text-sm">
                        <input
                          type="checkbox"
                          checked={present.has(s.id)}
                          onChange={() => toggle(s.id)}
                          className="mt-0.5"
                        />
                        <span>
                          {s.label}
                          <span className="ml-1 text-xs text-stone-500">
                            {done ? "(déjà signé dans ScolIA)" : "(en attente)"}
                          </span>
                        </span>
                      </label>
                    </li>
                  );
                })}
              </ul>
              <p className="mt-2 text-xs leading-relaxed text-stone-600">
                Cochées : considérées comme signées sur ce papier, plus de relance. Non cochées : elles
                restent à recueillir dans ScolIA et figureront sur une page « Validation et signatures »
                ajoutée à la fin. Si tout est coché, le document est gardé tel quel.
              </p>
            </div>
          ) : null}

          <div className="flex flex-wrap justify-end gap-2 pt-1">
            <button
              type="button"
              disabled={busy}
              onClick={onCancel}
              className="rounded-lg border border-stone-300 px-3 py-2 text-sm font-semibold text-stone-700 disabled:opacity-50"
            >
              Annuler
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() =>
                onSubmit({
                  note: note.trim() || undefined,
                  file,
                  presentOnPaperSignatureIds: file ? [...present] : [],
                })
              }
              className="rounded-lg bg-[#2F6B4A] px-3 py-2 text-sm font-semibold text-white disabled:opacity-50"
            >
              {busy ? "Enregistrement…" : "Valider la signature"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
