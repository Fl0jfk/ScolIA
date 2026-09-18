"use client";

import { useRef, useState, type DragEvent } from "react";

type Props = {
  disabled?: boolean;
  busy?: boolean;
  progressLabel?: string | null;
  /** Libellé d’aide (identité élève connue, etc.). */
  hint?: string;
  onFiles: (files: File[]) => void;
};

const ACCEPT =
  ".pdf,.png,.jpg,.jpeg,.webp,application/pdf,image/png,image/jpeg,image/webp";

export default function InscriptionDocsDropZone({
  disabled,
  busy,
  progressLabel,
  hint,
  onFiles,
}: Props) {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragOver, setDragOver] = useState(false);
  const locked = Boolean(disabled || busy);

  function takeFiles(list: FileList | null) {
    if (!list?.length || locked) return;
    onFiles(Array.from(list));
  }

  function onDrop(e: DragEvent<HTMLDivElement>) {
    e.preventDefault();
    setDragOver(false);
    takeFiles(e.dataTransfer.files);
  }

  return (
    <div className="space-y-2">
      <div
        role="button"
        tabIndex={0}
        aria-disabled={locked}
        onKeyDown={(e) => {
          if (locked) return;
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            inputRef.current?.click();
          }
        }}
        onDragOver={(e) => {
          e.preventDefault();
          if (!locked) setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={onDrop}
        onClick={() => {
          if (!locked) inputRef.current?.click();
        }}
        className={[
          "rounded-2xl border-2 border-dashed px-4 py-8 text-center transition-colors",
          locked ? "cursor-not-allowed opacity-60 border-slate-200 bg-slate-50" : "cursor-pointer",
          dragOver && !locked
            ? "border-sky-500 bg-sky-50"
            : "border-slate-300 bg-white hover:border-sky-400 hover:bg-sky-50/40",
        ].join(" ")}
      >
        <p className="text-sm font-bold text-slate-800">
          {busy
            ? progressLabel || "Analyse OCR en cours…"
            : "Glissez-déposez les pièces d’inscription"}
        </p>
        <p className="mt-1 text-xs text-slate-500">
          Plusieurs fichiers PDF ou images · l’IA détecte le type (fiche, bulletins, CNI…) · le
          nom de l’élève est ajouté automatiquement
        </p>
        {hint ? <p className="mt-2 text-xs font-medium text-sky-800">{hint}</p> : null}
        <button
          type="button"
          disabled={locked}
          className="mt-4 rounded-xl bg-sky-700 px-4 py-2 text-sm font-bold text-white hover:bg-sky-800 disabled:opacity-50"
          onClick={(e) => {
            e.stopPropagation();
            if (!locked) inputRef.current?.click();
          }}
        >
          {busy ? "Traitement…" : "Choisir des fichiers"}
        </button>
        <input
          ref={inputRef}
          type="file"
          multiple
          accept={ACCEPT}
          className="hidden"
          disabled={locked}
          onChange={(e) => {
            takeFiles(e.target.files);
            e.target.value = "";
          }}
        />
      </div>
    </div>
  );
}
