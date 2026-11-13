"use client";

import { useState } from "react";
import Link from "@/app/components/ui/AppLink";
import {
  PERSONNEL_CATEGORY_LABELS,
  type PersonnelIndexEntry,
} from "@/app/lib/personnel-types";
import { attachDocumentToStaff, isPersonnelDropFile } from "@/app/lib/personnel-upload-client";

type Props = {
  person: PersonnelIndexEntry;
  canDrop: boolean;
  onUploaded?: () => void;
};

export default function PersonnelStaffCard({ person, canDrop, onUploaded }: Props) {
  const [dragOver, setDragOver] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);

  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setDragOver(false);
    if (!canDrop || uploading) return;

    const file = e.dataTransfer.files?.[0];
    if (!file) return;
    if (!isPersonnelDropFile(file)) {
      alert("Format non supporté. Utilisez Excel (.xlsx, .xls), PDF ou Word.");
      return;
    }

    const visible = confirm(
      `Ajouter « ${file.name} » au dossier de ${person.displayName} ?\n\nOK = visible par la personne\nAnnuler = réservé établissement (RH seulement)`,
    );
    const visibility = visible ? ("personnel" as const) : ("establishment" as const);

    setUploading(true);
    try {
      await attachDocumentToStaff(person.id, file, { visibility });
      setFlash("Document ajouté");
      onUploaded?.();
      window.setTimeout(() => setFlash(null), 2500);
    } catch (err) {
      alert(err instanceof Error ? err.message : "Erreur lors du dépôt.");
    } finally {
      setUploading(false);
    }
  };

  return (
    <div
      onDragEnter={(e) => {
        if (!canDrop) return;
        e.preventDefault();
        e.stopPropagation();
        setDragOver(true);
      }}
      onDragOver={(e) => {
        if (!canDrop) return;
        e.preventDefault();
        e.stopPropagation();
        setDragOver(true);
      }}
      onDragLeave={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setDragOver(false);
      }}
      onDrop={handleDrop}
      className={[
        "relative rounded-xl border p-4 transition-all",
        canDrop && dragOver
          ? "border-indigo-500 bg-indigo-50 shadow-lg ring-2 ring-indigo-300 scale-[1.02]"
          : "border-slate-100 bg-white hover:border-indigo-200 hover:shadow-md",
        uploading ? "opacity-70 pointer-events-none" : "",
      ].join(" ")}
    >
      <Link href={`/rh/${person.id}`} className="block group">
        <p className="font-black text-slate-800 group-hover:text-indigo-700">{person.displayName}</p>
        <p className="text-xs text-slate-500 mt-1">{PERSONNEL_CATEGORY_LABELS[person.category]}</p>
        {person.onboardingStatus && ["en_cours", "signatures", "brouillon"].includes(person.onboardingStatus) && (
          <span className="inline-block mt-2 text-[10px] font-bold bg-amber-100 text-amber-800 px-2 py-0.5 rounded-full">
            Onboarding
          </span>
        )}
      </Link>

      {canDrop && (
        <p className="text-[10px] text-slate-400 mt-3 leading-snug">
          {dragOver ? "Déposez le fichier ici" : "Glisser Excel ou document"}
        </p>
      )}

      {uploading && (
        <div className="absolute inset-0 flex items-center justify-center bg-white/80 rounded-xl">
          <p className="text-xs font-bold text-indigo-600">Envoi…</p>
        </div>
      )}

      {flash && (
        <div className="absolute top-2 right-2 text-[10px] font-bold bg-emerald-100 text-emerald-800 px-2 py-0.5 rounded-full">
          {flash}
        </div>
      )}
    </div>
  );
}
