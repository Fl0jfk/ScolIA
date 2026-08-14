"use client";

import {
  ocrFailureCategory,
  ocrFailureHint,
  tempOneDriveDisplayPath,
  type ProcessResult,
} from "@/app/lib/ocr-page-model";

export default function OcrResultsList({
  ocrResults,
  ocrResultsSessionId,
  openingOneDrivePath,
  onOpenOneDrivePath,
}: {
  ocrResults: ProcessResult[];
  ocrResultsSessionId: number;
  openingOneDrivePath: string | null;
  onOpenOneDrivePath: (path: string) => void;
}) {
  const failedResults = ocrResults.filter((r) => !r.success);
  if (ocrResults.length === 0 && failedResults.length === 0) return null;

  return (
    <>
      {failedResults.length > 0 ? (
        <div className="mb-8 p-6 bg-amber-50 border-2 border-amber-300 rounded-3xl shadow-lg">
          <h3 className="text-lg font-black text-amber-950 mb-2 flex items-center gap-2">
            <span>📁</span>
            {failedResults.length} document
            {failedResults.length > 1 ? "s" : ""} à traiter manuellement
          </h3>
          <div className="text-sm text-amber-950 mb-4 leading-relaxed space-y-3">
            <p>
              Ces fichiers n&apos;ont <strong>pas pu être rangés automatiquement</strong> dans le dossier d&apos;un
              élève. Ils se trouvent dans le dossier <strong>Temp</strong>, à la{" "}
              <strong>racine de votre OneDrive</strong> (même niveau que « Documents », « Images », etc.).
            </p>
            <p>
              <strong>Pourquoi ?</strong> Le plus souvent : le nom de l&apos;élève n&apos;a pas été reconnu, le type de
              document est ambigu, ou le texte du PDF est illisible.
            </p>
            <p>
              <strong>Que faire ?</strong>
            </p>
            <ul className="list-disc pl-5 space-y-1">
              <li>
                Ouvrez OneDrive → dossier <strong>Temp</strong> → déplacez le PDF dans le bon dossier élève ;
              </li>
              <li>
                ou, si le document n&apos;a pas été reconnu, vérifiez qu&apos;il est lisible et que l&apos;élève figure
                bien dans la liste, puis redéposez-le.
              </li>
            </ul>
          </div>
          <ul className="space-y-3">
            {failedResults.map((r, index) => (
              <li
                key={`${ocrResultsSessionId}-fail-${r.fileName}-${index}`}
                className="p-4 bg-white rounded-xl border border-amber-200"
              >
                <p className="font-bold text-slate-900">{r.fileName}</p>
                <p className="text-sm text-slate-600 mt-1">{ocrFailureHint(r)}</p>
                {r.tempOneDrivePath ? (
                  <p className="text-sm text-slate-700 mt-2">
                    Emplacement OneDrive :{" "}
                    <span className="font-semibold">Temp / {tempOneDriveDisplayPath(r.tempOneDrivePath)}</span>
                  </p>
                ) : null}
              </li>
            ))}
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
