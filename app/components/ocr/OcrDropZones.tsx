"use client";

import type { RefObject } from "react";
import { ocrDropZoneClass } from "@/app/lib/ocr-page-model";
import OcrProcessingSpinner from "./OcrProcessingSpinner";

export default function OcrDropZones({
  dropsAvailable,
  dropDisabled,
  ocrProcessing,
  checkingOneDrive,
  isDraggingClass,
  inputRef,
  onDraggingChange,
  onFiles,
}: {
  dropsAvailable: boolean;
  dropDisabled: boolean;
  ocrProcessing: boolean;
  checkingOneDrive: boolean;
  isDraggingClass: boolean;
  inputRef: RefObject<HTMLInputElement | null>;
  onDraggingChange: (dragging: boolean) => void;
  onFiles: (files: FileList) => void;
}) {
  if (ocrProcessing) return null;

  return (
    <>
      <div className="mb-4">
        <p className="text-[10px] font-black uppercase tracking-wider text-slate-500 mb-1">
          Étape 2 — Dépôt des PDF
        </p>
        <p className="text-sm text-slate-600">
          {dropsAvailable
            ? "Déposez vos PDF : l'outil détecte automatiquement s'il s'agit d'un document par élève ou d'un export de classe entière à découper."
            : "La zone ci-dessous reste désactivée tant que OneDrive n'est pas connecté."}
        </p>
      </div>

      <div className="mb-8">
        <div
          id="ocr-drop-standard"
          data-tour="drop-standard"
          onDragOver={
            dropDisabled
              ? undefined
              : (e) => {
                  e.preventDefault();
                  onDraggingChange(true);
                }
          }
          onDragLeave={() => onDraggingChange(false)}
          onDrop={
            dropDisabled
              ? undefined
              : (e) => {
                  e.preventDefault();
                  onDraggingChange(false);
                  if (e.dataTransfer.files?.length) {
                    onFiles(e.dataTransfer.files);
                  }
                }
          }
          onClick={() => !dropDisabled && inputRef.current?.click()}
          className={ocrDropZoneClass(isDraggingClass, "blue", {
            dropsAvailable,
            dropDisabled,
            ocrProcessing,
          })}
        >
          <div className="mb-3 min-h-[4rem] flex items-center justify-center">
            {ocrProcessing ? <OcrProcessingSpinner size="text-6xl" /> : <span className="text-5xl">📄</span>}
          </div>
          <h3 className="text-lg font-bold text-gray-800 mb-2">
            {!dropsAvailable
              ? "Connexion OneDrive requise"
              : checkingOneDrive
                ? "Vérification OneDrive…"
                : ocrProcessing
                  ? "Analyse en cours…"
                  : "Déposez vos PDF — détection automatique"}
          </h3>
          <p className="text-sm text-gray-500">
            {!dropsAvailable
              ? "Connectez-vous à l'étape 1 pour débloquer le dépôt de fichiers."
              : checkingOneDrive
                ? "Connexion Microsoft vérifiée avant tout traitement."
                : ocrProcessing
                  ? "Traitement en cours — patientez."
                  : "Un document par élève OU un export de classe entière : l'outil reconnaît, découpe si besoin et range automatiquement. Glissez-déposez (plusieurs fichiers possibles) ou cliquez."}
          </p>
          <input
            ref={inputRef}
            type="file"
            className="hidden"
            multiple
            accept="application/pdf,.pdf"
            onChange={(e) => {
              if (e.target.files) {
                onFiles(e.target.files);
                e.target.value = "";
              }
            }}
          />
        </div>
      </div>
    </>
  );
}
