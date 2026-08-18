"use client";

import { useState } from "react";
import {
  ocrExtractedSummary,
  ocrFailureCategory,
  ocrFailureHint,
  ocrSuggestedEleves,
  tempOneDriveDisplayPath,
  type OcrSuggestedEleve,
  type ProcessResult,
} from "@/app/lib/ocr-page-model";

export default function OcrResultsList({
  ocrResults,
  ocrResultsSessionId,
  openingOneDrivePath,
  onOpenOneDrivePath,
  accessToken,
  onManualFiled,
}: {
  ocrResults: ProcessResult[];
  ocrResultsSessionId: number;
  openingOneDrivePath: string | null;
  onOpenOneDrivePath: (path: string) => void;
  accessToken?: string | null;
  onManualFiled?: (fileName: string, candidate: OcrSuggestedEleve, finalFileName: string) => void;
}) {
  const failedResults = ocrResults.filter((r) => !r.success);
  const [filingKey, setFilingKey] = useState<string | null>(null);
  const [fileError, setFileError] = useState<Record<string, string>>({});

  async function fileToCandidate(result: ProcessResult, candidate: OcrSuggestedEleve) {
    if (!accessToken) {
      setFileError((prev) => ({ ...prev, [result.fileName]: "Reconnectez OneDrive pour ranger." }));
      return;
    }
    if (!result.tempOneDrivePath || !candidate.folderPath) {
      setFileError((prev) => ({ ...prev, [result.fileName]: "Chemin OneDrive manquant." }));
      return;
    }
    const key = `${result.fileName}::${candidate.folderName}`;
    setFilingKey(key);
    setFileError((prev) => ({ ...prev, [result.fileName]: "" }));
    try {
      const baseName = String(result.result?.fileName || result.fileName).replace(/\.pdf$/i, "");
      const res = await fetch("/api/agentIAOCR/move-file", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          accessToken,
          sourcePath: result.tempOneDrivePath,
          targetFolderPath: candidate.folderPath,
          newFileName: `${baseName}.pdf`,
        }),
      });
      const data = (await res.json().catch(() => ({}))) as { error?: string; finalFileName?: string };
      if (!res.ok) {
        throw new Error(data.error || `Déplacement impossible (${res.status})`);
      }
      onManualFiled?.(result.fileName, candidate, data.finalFileName || `${baseName}.pdf`);
    } catch (e) {
      setFileError((prev) => ({
        ...prev,
        [result.fileName]: e instanceof Error ? e.message : String(e),
      }));
    } finally {
      setFilingKey(null);
    }
  }

  if (ocrResults.length === 0 && failedResults.length === 0) return null;

  return (
    <>
      {failedResults.length > 0 ? (
        <div className="mb-8 p-6 bg-amber-50 border-2 border-amber-300 rounded-3xl shadow-lg">
          <h3 className="text-lg font-black text-amber-950 mb-2 flex items-center gap-2">
            <span>📁</span>
            {failedResults.length} document
            {failedResults.length > 1 ? "s" : ""} à traiter
          </h3>
          <div className="text-sm text-amber-950 mb-4 leading-relaxed space-y-3">
            <p>
              Ces fichiers n&apos;ont <strong>pas pu être rangés automatiquement</strong>. S&apos;il y a des
              suggestions, un clic suffit. Sinon ils restent dans le dossier <strong>Temp</strong> de votre OneDrive.
            </p>
          </div>
          <ul className="space-y-3">
            {failedResults.map((r, index) => {
              const suggestions = ocrSuggestedEleves(r);
              const extracted = ocrExtractedSummary(r);
              return (
                <li
                  key={`${ocrResultsSessionId}-fail-${r.fileName}-${index}`}
                  className="p-4 bg-white rounded-xl border border-amber-200"
                >
                  <p className="font-bold text-slate-900">{r.fileName}</p>
                  <p className="text-sm text-slate-600 mt-1">{ocrFailureHint(r)}</p>
                  {extracted ? (
                    <p className="text-xs text-slate-500 mt-1">Lu dans le document : {extracted}</p>
                  ) : null}
                  {suggestions.length > 0 ? (
                    <div className="mt-3 space-y-2">
                      <p className="text-xs font-bold uppercase tracking-wide text-amber-900">Élèves possibles</p>
                      {suggestions.map((c) => {
                        const key = `${r.fileName}::${c.folderName}`;
                        return (
                          <div
                            key={c.folderName}
                            className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-slate-200 bg-slate-50 px-3 py-2"
                          >
                            <div>
                              <p className="text-sm font-bold text-slate-900">
                                {c.nom} {c.prenom}
                                {c.classe ? ` · ${c.classe}` : ""}
                              </p>
                              <p className="text-[11px] text-slate-500">
                                {c.folderName}
                                {c.matchedBy ? ` · ${c.matchedBy}` : ""}
                              </p>
                            </div>
                            <button
                              type="button"
                              disabled={!accessToken || filingKey === key || !c.folderPath}
                              onClick={() => void fileToCandidate(r, c)}
                              className="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-bold text-white hover:bg-indigo-700 disabled:opacity-50"
                            >
                              {filingKey === key ? "Rangement…" : "Ranger ici"}
                            </button>
                          </div>
                        );
                      })}
                    </div>
                  ) : null}
                  {fileError[r.fileName] ? (
                    <p className="text-xs text-red-600 mt-2">{fileError[r.fileName]}</p>
                  ) : null}
                  {r.tempOneDrivePath ? (
                    <p className="text-sm text-slate-700 mt-2">
                      Emplacement OneDrive :{" "}
                      <span className="font-semibold">Temp / {tempOneDriveDisplayPath(r.tempOneDrivePath)}</span>
                    </p>
                  ) : null}
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}

      {ocrResults.length > 0 ? (
        <div>
          <h3 className="text-xl font-black text-gray-900 mb-4">Journal d&apos;analyse</h3>
          <div className="grid grid-cols-1 gap-3">
            {[...ocrResults]
              .sort((a, b) => (a.success === b.success ? 0 : a.success ? 1 : -1))
              .map((result, index) => {
                const sourcePath = String(result.result?.oneDriveItemPath || result.tempOneDrivePath || "");
                const category = ocrFailureCategory(result);
                return (
                  <div
                    key={`${ocrResultsSessionId}-${result.fileName}-${index}`}
                    className={`p-4 rounded-2xl border ${
                      result.success ? "bg-white border-gray-100" : "bg-red-50 border-red-100 ring-2 ring-red-400/20"
                    }`}
                  >
                    <div className="flex items-center gap-2 flex-wrap">
                      <p className={`font-bold ${result.success ? "text-gray-800" : "text-red-700"}`}>
                        {result.fileName}
                      </p>
                      {!result.success ? (
                        <span
                          className={`text-[10px] px-2 py-0.5 rounded-full font-black uppercase text-white ${
                            category.technical ? "bg-orange-600" : "bg-red-600"
                          }`}
                        >
                          {category.label}
                        </span>
                      ) : null}
                    </div>
                    {result.success ? (
                      <p className="text-xs text-gray-500 mt-1">Classé : {result.result?.fileName || "—"}</p>
                    ) : (
                      <>
                        <p className="text-sm text-red-600 mt-1 font-medium">{ocrFailureHint(result)}</p>
                        {result.error ? (
                          <p className="text-[11px] text-slate-500 mt-1 font-mono break-words">
                            Détail : {result.error}
                          </p>
                        ) : null}
                        {result.tempOneDrivePath ? (
                          <p className="text-xs text-slate-600 mt-2 bg-slate-50 p-2 rounded-lg">
                            Fichier dans OneDrive → dossier <strong>Temp</strong> (
                            {tempOneDriveDisplayPath(result.tempOneDrivePath)})
                          </p>
                        ) : null}
                      </>
                    )}
                    {sourcePath ? (
                      <button
                        type="button"
                        onClick={() => onOpenOneDrivePath(sourcePath)}
                        disabled={openingOneDrivePath === sourcePath}
                        className="mt-2 inline-flex items-center rounded-lg border border-indigo-200 bg-indigo-50 px-2.5 py-1 text-xs font-bold text-indigo-800 hover:bg-indigo-100 disabled:opacity-60"
                      >
                        {openingOneDrivePath === sourcePath
                          ? "Ouverture…"
                          : "Ouvrir le document source dans OneDrive"}
                      </button>
                    ) : null}
                  </div>
                );
              })}
          </div>
        </div>
      ) : null}
    </>
  );
}
