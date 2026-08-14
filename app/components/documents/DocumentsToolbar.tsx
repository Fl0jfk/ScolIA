"use client";

import type { RefObject } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import type { ShareInfo } from "@/app/lib/documents-page-model";

export default function DocumentsToolbar({
  fileInputRef,
  uploading,
  uploadDisabled,
  canDeleteCurrentFolder,
  deleteFolderLabel,
  activeShare,
  shareId,
  canLeaveShare,
  onNewFolder,
  onPickFiles,
  onNewShare,
  onShowAccess,
  onManageAccess,
  onDeleteFolder,
  onLeaveShare,
  onFilesSelected,
}: {
  fileInputRef: RefObject<HTMLInputElement | null>;
  uploading: boolean;
  uploadDisabled: boolean;
  canDeleteCurrentFolder: boolean;
  deleteFolderLabel: string;
  activeShare: ShareInfo | null;
  shareId: string | null;
  canLeaveShare: boolean;
  onNewFolder: () => void;
  onPickFiles: () => void;
  onNewShare: () => void;
  onShowAccess: () => void;
  onManageAccess: () => void;
  onDeleteFolder: () => void;
  onLeaveShare: () => void;
  onFilesSelected: (files: File[]) => void;
}) {
  return (
    <div data-tour="documents-upload" className="flex flex-wrap gap-2">
      <input
        ref={fileInputRef}
        type="file"
        multiple
        className="hidden"
        onChange={(e) => {
          const list = Array.from(e.target.files ?? []);
          e.target.value = "";
          onFilesSelected(list);
        }}
      />
      <ModuleButton
        variant="secondary"
        onClick={onNewFolder}
        disabled={uploadDisabled}
        className="disabled:cursor-not-allowed"
      >
        + Dossier
      </ModuleButton>
      <ModuleButton
        variant="secondary"
        onClick={onPickFiles}
        disabled={uploadDisabled || uploading}
        className="disabled:cursor-not-allowed"
      >
        + Ajouter un fichier
      </ModuleButton>
      <button
        type="button"
        onClick={onNewShare}
        className="px-4 py-2 rounded-xl border border-blue-200 text-blue-700 text-sm font-semibold hover:bg-blue-50"
      >
        + Dossier partagé
      </button>
      {activeShare && shareId && (
        <button
          type="button"
          onClick={onShowAccess}
          className="px-4 py-2 rounded-xl border border-indigo-200 text-indigo-700 text-sm font-semibold hover:bg-indigo-50"
        >
          Voir qui a accès
        </button>
      )}
      {activeShare?.isOwner && shareId && (
        <button
          type="button"
          onClick={onManageAccess}
          className="px-4 py-2 rounded-xl bg-slate-800 text-white text-sm font-semibold hover:bg-slate-700"
        >
          Modifier les accès
        </button>
      )}
      {canDeleteCurrentFolder && (
        <button
          type="button"
          onClick={onDeleteFolder}
          className="px-4 py-2 rounded-xl border border-red-200 text-red-700 text-sm font-semibold hover:bg-red-50"
        >
          {deleteFolderLabel}
        </button>
      )}
      {canLeaveShare && (
        <button
          type="button"
          onClick={onLeaveShare}
          className="px-4 py-2 rounded-xl border border-amber-200 text-amber-800 text-sm font-semibold hover:bg-amber-50"
        >
          Quitter le dossier
        </button>
      )}
    </div>
  );
}
