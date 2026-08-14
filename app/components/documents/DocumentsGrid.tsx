"use client";

import type { DragEvent, ReactNode } from "react";
import ModuleEmptyState from "@/app/components/module-chrome/ModuleEmptyState";

export default function DocumentsGrid({
  rootLabel,
  pathSegments,
  error,
  isSharePicker,
  dragActive,
  uploading,
  loading,
  hasItems,
  onNavigateSegment,
  onDrop,
  onDragActiveChange,
  children,
}: {
  rootLabel: string;
  pathSegments: string[];
  error: string | null;
  isSharePicker: boolean;
  dragActive: boolean;
  uploading: boolean;
  loading: boolean;
  hasItems: boolean;
  onNavigateSegment: (index: number) => void;
  onDrop: (e: DragEvent<HTMLDivElement>) => void;
  onDragActiveChange: (active: boolean) => void;
  children: ReactNode;
}) {
  return (
    <section className="flex-1 min-w-0 flex flex-col min-h-[480px] max-h-[min(720px,calc(100vh-10rem))]">
      <div data-tour="documents-breadcrumb" className="flex items-center gap-1 text-sm text-gray-500 mb-3 flex-wrap">
        <button type="button" onClick={() => onNavigateSegment(-1)} className="hover:text-blue-600 font-medium">
          {rootLabel}
        </button>
        {pathSegments.map((seg, i) => (
          <span key={`${seg}-${i}`} className="flex items-center gap-1">
            <span>/</span>
            <button type="button" onClick={() => onNavigateSegment(i)} className="hover:text-blue-600 font-medium">
              {seg}
            </button>
          </span>
        ))}
      </div>

      {error && (
        <div className="mb-3 rounded-xl border border-red-200 bg-red-50 px-4 py-2 text-sm text-red-700">
          {error}
        </div>
      )}

      {isSharePicker ? (
        <ModuleEmptyState>
          <p className="text-4xl mb-3">👥</p>
          <p className="font-medium">Sélectionnez un dossier partagé dans la barre latérale.</p>
        </ModuleEmptyState>
      ) : (
        <div
          data-tour="documents-dropzone"
          className={[
            "relative flex flex-col flex-1 min-h-0 rounded-3xl border-2 border-dashed transition-colors",
            dragActive ? "border-blue-500 bg-blue-50/50" : "border-gray-200 bg-gray-50/40",
            uploading ? "opacity-70 pointer-events-none" : "",
          ].join(" ")}
          onDragEnter={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (!uploading) onDragActiveChange(true);
          }}
          onDragOver={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (!uploading) onDragActiveChange(true);
          }}
          onDragLeave={(e) => {
            e.preventDefault();
            e.stopPropagation();
            if (e.currentTarget === e.target) onDragActiveChange(false);
          }}
          onDrop={onDrop}
        >
          {loading && (
            <div className="absolute top-4 right-4">
              <div className="h-5 w-5 border-2 border-blue-600 border-t-transparent animate-spin rounded-full" />
            </div>
          )}

          {uploading && (
            <div className="absolute inset-0 flex items-center justify-center bg-white/60 rounded-3xl z-10">
              <p className="text-sm font-semibold text-blue-700">Envoi en cours…</p>
            </div>
          )}

          {dragActive && !uploading && (
            <div className="absolute inset-0 flex items-center justify-center pointer-events-none z-10">
              <p className="text-blue-700 font-bold text-lg bg-white/90 px-6 py-3 rounded-2xl shadow">
                Déposez vos fichiers ou dossiers ici
              </p>
            </div>
          )}

          <div className="flex-1 min-h-0 overflow-y-auto p-4">
            {hasItems ? (
              <div
                data-tour="documents-grid"
                className="grid grid-cols-3 sm:grid-cols-4 md:grid-cols-5 lg:grid-cols-6 gap-2 sm:gap-3 items-start"
                onClick={(e) => e.stopPropagation()}
              >
                {children}
              </div>
            ) : (
              !loading && (
                <div className="flex flex-col items-center justify-center min-h-[240px] h-full text-gray-400 pointer-events-none">
                  <span className="text-4xl mb-3">📂</span>
                  <p className="font-medium text-gray-600 text-sm">Ce dossier est vide</p>
                  <p className="text-xs mt-2 italic text-center px-4">
                    Glissez-déposez des fichiers ou dossiers ici
                  </p>
                </div>
              )
            )}
          </div>
        </div>
      )}
    </section>
  );
}
