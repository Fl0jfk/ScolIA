"use client";

import { useId, useState } from "react";

type TripDocumentsDropZoneProps = {
  onFiles: (files: File[]) => void | Promise<void>;
  uploading?: boolean;
  disabled?: boolean;
  multiple?: boolean;
  title?: string;
  hint?: string;
  className?: string;
};

/** Zone glisser-déposer + clic pour joindre des documents séjour. */
export function TripDocumentsDropZone({
  onFiles,
  uploading = false,
  disabled = false,
  multiple = true,
  title = "Glisser-déposer des documents ici",
  hint = "ou cliquer pour parcourir — plusieurs fichiers acceptés",
  className = "",
}: TripDocumentsDropZoneProps) {
  const [dragOver, setDragOver] = useState(false);
  const inputId = useId();
  const blocked = disabled || uploading;

  const takeFiles = (list: FileList | null | undefined) => {
    if (!list?.length || blocked) return;
    const files = Array.from(list);
    void onFiles(multiple ? files : files.slice(0, 1));
  };

  return (
    <label
      htmlFor={inputId}
      onDragEnter={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (!blocked) setDragOver(true);
      }}
      onDragOver={(e) => {
        e.preventDefault();
        e.stopPropagation();
        if (!blocked) setDragOver(true);
      }}
      onDragLeave={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
      }}
      onDrop={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
        takeFiles(e.dataTransfer.files);
      }}
      className={[
        "block rounded-2xl border-2 border-dashed px-4 py-8 text-center transition-colors",
        blocked ? "cursor-wait opacity-70" : "cursor-pointer",
        dragOver
          ? "border-indigo-500 bg-indigo-50/70"
          : "border-slate-200 bg-slate-50/40 hover:border-indigo-300 hover:bg-indigo-50/40",
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      <input
        id={inputId}
        type="file"
        multiple={multiple}
        className="hidden"
        disabled={blocked}
        onChange={(e) => {
          takeFiles(e.target.files);
          e.target.value = "";
        }}
      />
      <p className="text-sm font-bold text-slate-800">
        {uploading ? "Envoi en cours…" : dragOver ? "Déposez les fichiers ici" : title}
      </p>
      {!uploading && <p className="mt-1.5 text-xs text-slate-500">{hint}</p>}
    </label>
  );
}
