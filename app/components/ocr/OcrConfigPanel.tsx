"use client";

import type { RefObject } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import type { OcrMefCounts, OcrSyncReport } from "@/app/lib/ocr-page-model";

export default function OcrConfigPanel({
  elevesCount,
  mefCounts,
  mefUploading,
  mefMessage,
  mefInputRef,
  dropsAvailable,
  checkingOneDrive,
  syncingFolders,
  syncReport,
  onMefFile,
  onSyncFolders,
}: {
  elevesCount: number | null;
  mefCounts: OcrMefCounts | null;
  mefUploading: boolean;
  mefMessage: string;
  mefInputRef: RefObject<HTMLInputElement | null>;
  dropsAvailable: boolean;
  checkingOneDrive: boolean;
  syncingFolders: boolean;
  syncReport: OcrSyncReport | null;
  onMefFile: (file: File) => void;
  onSyncFolders: () => void;
}) {
  return (
    <details
      data-tour="eleves-import"
      className="mt-10 rounded-3xl border border-slate-200 bg-white shadow-sm overflow-hidden group"
    >
      <summary className="cursor-pointer list-none px-6 py-4 font-bold text-slate-800 hover:bg-slate-50 flex items-center justify-between gap-2">
        <span>Configuration — liste élèves & dossiers OneDrive</span>
        <span className="text-slate-400 text-sm font-normal group-open:rotate-180 transition-transform">▼</span>
      </summary>
      <div className="px-6 pb-6 pt-2 border-t border-slate-100 space-y-6">
        <p className="text-sm text-slate-600">
          Trois actions dans l&apos;ordre : importer la liste élèves, configurer la table MEF, puis créer les dossiers
          OneDrive manquants.
          {elevesCount != null ? (
            <span className="block mt-1 font-medium text-slate-800">
              {elevesCount} élève(s) actuellement enregistré(s) pour le classement automatique.
            </span>
          ) : null}
        </p>

        <section className="rounded-xl border border-indigo-200 bg-indigo-50/50 p-4 space-y-3">
          <h3 className="text-sm font-bold text-slate-800">1 — Liste élèves (référentiel global)</h3>
          <p className="text-xs text-slate-600 leading-relaxed">
            L&apos;import Excel des élèves se fait désormais dans les{" "}
            <a href="/parametres?tab=referentiel" className="font-bold text-indigo-800 underline">
              Paramètres généraux → Référentiel scolaire
            </a>
            . Le fichier <code className="bg-white px-1 rounded">eleves.json</code> alimente tous les modules (stages,
            certificats, répartition des classes, reconnaissance IA…).
          </p>
          {elevesCount != null ? (
            <p className="text-sm font-medium text-slate-800">{elevesCount} élève(s) actuellement enregistré(s).</p>
          ) : null}
          <a
            href="/parametres?tab=referentiel"
            className="inline-flex rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white hover:bg-indigo-700"
          >
            Mettre à jour la liste élèves →
          </a>
        </section>

        <section data-tour="mef-table" className="rounded-xl border border-slate-200 bg-slate-50/50 p-4 space-y-3">
          <h3 className="text-sm font-bold text-slate-800">2 — Table des formations (MEF)</h3>
          <p className="text-xs text-slate-600 leading-relaxed">
            Associe chaque <strong>code ou libellé MEF</strong> (colonne E de l&apos;Excel : 3EME, 2NDE, Cycle 2…) au
            secteur <strong>Lycée</strong>, <strong>Collège</strong> ou <strong>École</strong>. Cela permet à chaque
            secrétariat de ne traiter que ses élèves et de créer les bons dossiers OneDrive.
          </p>
          <p className="text-xs text-slate-500">
            Configuration habituelle :{" "}
            <a href="/parametres" className="text-indigo-600 font-medium hover:underline">
              Paramètres → Formations MEF
            </a>{" "}
            (une fois par an ou si les formations changent). Le bouton ci-dessous est un raccourci pour importer un
            fichier JSON déjà préparé.
          </p>
          {mefCounts && mefCounts.total > 0 ? (
            <p className="text-xs font-medium text-emerald-800 bg-emerald-50 border border-emerald-100 rounded-lg px-3 py-2">
              Table MEF active : {mefCounts.total} formation(s) — lycée {mefCounts.lycee}, collège {mefCounts.college},
              école {mefCounts.ecole}.
            </p>
          ) : null}
          <div>
            <ModuleButton
              variant="secondary"
              disabled={mefUploading}
              onClick={() => mefInputRef.current?.click()}
              className="px-4 py-2 text-sm"
            >
              {mefUploading ? "Envoi…" : "Importer table MEF (JSON)"}
            </ModuleButton>
          </div>
          {mefMessage ? (
            <p className={`text-sm ${mefMessage.startsWith("Erreur") ? "text-red-600" : "text-slate-700"}`}>
              {mefMessage}
            </p>
          ) : null}
        </section>

        <section data-tour="sync-onedrive" className="rounded-xl border border-slate-200 bg-slate-50/50 p-4 space-y-3">
          <h3 className="text-sm font-bold text-slate-800">3 — Créer les dossiers sur OneDrive</h3>
          <p className="text-xs text-slate-600 leading-relaxed">
            Crée un dossier par élève de <strong>votre secteur</strong> (format « NOM Prenom » — sans tirets, sans
            classe) dans l&apos;arborescence OneDrive connectée en haut de page.
          </p>
          <div className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-xs text-emerald-950 leading-relaxed space-y-1">
            <p className="font-bold">Sans risque pour les dossiers existants</p>
            <p>
              Ce bouton <strong>ne supprime ni ne renomme rien</strong>. Il ajoute uniquement les dossiers manquants
              pour les élèves de la liste actuelle. Les dossiers déjà là sont laissés tels quels.
            </p>
            <p>
              Il est normal d&apos;avoir <strong>plus de dossiers sur OneDrive</strong> que d&apos;élèves dans la liste
              : les anciens élèves partis restent archivés sur OneDrive et ne sont <strong>jamais supprimés</strong>{" "}
              par cette action.
            </p>
          </div>
          <div>
            <ModuleButton
              disabled={!dropsAvailable || syncingFolders || checkingOneDrive}
              onClick={onSyncFolders}
              className="px-4 py-2 text-sm"
            >
              {syncingFolders ? "Synchronisation…" : "Créer les dossiers sur OneDrive"}
            </ModuleButton>
            {!dropsAvailable ? (
              <p className="mt-2 text-xs text-amber-700">
                Connectez OneDrive en haut de page pour activer ce bouton.
              </p>
            ) : null}
          </div>
          {syncReport ? (
            <ModuleCard className="p-4 text-sm text-slate-700 space-y-3">
              <div>
                <p className="font-bold text-slate-900">
                  {syncReport.secteurLabel} — {syncReport.basePath}
                </p>
                <p className="text-slate-600 mt-1">{syncReport.message}</p>
              </div>

              <div className="grid grid-cols-2 sm:grid-cols-4 gap-2 text-xs">
                <div className="rounded-lg bg-emerald-50 border border-emerald-100 px-3 py-2">
                  <p className="font-bold text-emerald-900">{syncReport.created ?? 0}</p>
                  <p className="text-emerald-800">créé(s)</p>
                </div>
                <div className="rounded-lg bg-slate-50 border border-slate-200 px-3 py-2">
                  <p className="font-bold text-slate-800">{syncReport.alreadyThere ?? 0}</p>
                  <p className="text-slate-600">déjà présent(s)</p>
                </div>
                <div className="rounded-lg bg-amber-50 border border-amber-100 px-3 py-2">
                  <p className="font-bold text-amber-900">{syncReport.extraFoldersCount ?? 0}</p>
                  <p className="text-amber-800">archives OneDrive</p>
                </div>
                <div className="rounded-lg bg-slate-50 border border-slate-200 px-3 py-2">
                  <p className="font-bold text-slate-800">{syncReport.jsonForYourSecteur ?? "—"}</p>
                  <p className="text-slate-600">élèves secteur</p>
                </div>
              </div>

              {(syncReport.createdFolders?.length ?? 0) > 0 ? (
                <div>
                  <p className="text-xs font-bold text-slate-800 mb-1">
                    Dossiers créés ({syncReport.createdFolders!.length})
                  </p>
                  <ul className="max-h-48 overflow-y-auto rounded-lg border border-emerald-100 bg-emerald-50/50 text-xs font-mono divide-y divide-emerald-100">
                    {syncReport.createdFolders!.map((name) => (
                      <li key={name} className="px-3 py-1.5 text-emerald-950">
                        {name}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}

              {(syncReport.created ?? 0) === 0 && (syncReport.alreadyThere ?? 0) > 0 ? (
                <p className="text-xs text-slate-600 leading-relaxed">
                  Tous les élèves de la liste avaient déjà leur dossier — rien à ajouter, c&apos;est normal.
                </p>
              ) : null}

              {(syncReport.extraFoldersCount ?? 0) > 0 ? (
                <p className="text-xs text-amber-900 bg-amber-50 border border-amber-100 rounded-lg px-3 py-2 leading-relaxed">
                  <strong>{syncReport.extraFoldersCount} dossier(s)</strong> sur OneDrive ne correspondent plus à un
                  élève de la liste actuelle (anciens élèves, archives…). Ils ont été{" "}
                  <strong>laissés en place</strong> — cette action ne les supprime pas.
                </p>
              ) : null}

              {(syncReport.ambiguousCount ?? 0) > 0 ? (
                <div>
                  <p className="text-xs font-bold text-amber-800 mb-1">
                    Non traités — MEF manquant ou inconnu ({syncReport.ambiguousCount})
                  </p>
                  <ul className="max-h-32 overflow-y-auto text-xs text-amber-900 space-y-0.5">
                    {syncReport.ambiguous?.map((a) => (
                      <li key={a.folderName}>
                        {a.folderName}
                        {a.mef ? ` (${a.mef})` : ""} — {a.reason}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}

              {(syncReport.errors?.length ?? 0) > 0 ? (
                <div>
                  <p className="text-xs font-bold text-red-700 mb-1">Erreurs</p>
                  <ul className="text-xs text-red-600 space-y-0.5">
                    {syncReport.errors!.map((e) => (
                      <li key={e.folderName}>
                        {e.folderName} — {e.error}
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </ModuleCard>
          ) : null}
        </section>

        <input
          ref={mefInputRef}
          type="file"
          accept=".json,application/json"
          className="hidden"
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) onMefFile(f);
          }}
        />
      </div>
    </details>
  );
}
