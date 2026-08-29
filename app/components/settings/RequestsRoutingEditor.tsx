"use client";

import type { RequestsRoutingConfig } from "@/app/lib/app-config-schemas";
import type { DirectoryMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

type Props = {
  config: RequestsRoutingConfig;
  onChange: (next: RequestsRoutingConfig) => void;
  members: DirectoryMemberOption[];
  membersLoading: boolean;
  /** options = portail parents + règle direction */
  mode?: "full" | "files" | "options";
};

export default function RequestsRoutingEditor({
  config,
  onChange,
  mode = "full",
}: Props) {
  if (mode !== "options" && mode !== "full") {
    return null;
  }

  return (
    <div className="space-y-8">
      <section className="space-y-4 rounded-2xl border border-amber-200 bg-amber-50/40 p-6">
        <div>
          <h2 className="text-lg font-bold text-slate-900">Page publique parents</h2>
          <p className="mt-1 text-sm text-slate-600">
            Ouvre une page simple pour que les familles déposent une demande. Confirmation par e-mail
            anti-spam ; reconnaissance automatique si l&apos;e-mail figure dans la liste élèves.
          </p>
        </div>
        <label className="flex items-center gap-3 text-sm font-bold text-slate-800">
          <input
            type="checkbox"
            className="h-4 w-4 rounded border-slate-300"
            checked={config.parentPortal?.enabled === true}
            onChange={(e) =>
              onChange({
                ...config,
                parentPortal: { enabled: e.target.checked },
              })
            }
          />
          Ouvrir la page de demandes parents
        </label>
        {config.parentPortal?.enabled ? (
          <div className="rounded-xl border border-amber-200/80 bg-white px-4 py-3 text-sm">
            <p className="font-bold text-slate-800">Lien à partager</p>
            <p className="mt-1 break-all font-mono text-xs text-amber-900">
              {typeof window !== "undefined"
                ? `${window.location.origin}/demande-parents`
                : "/demande-parents"}
            </p>
            <p className="mt-2 text-xs text-slate-500">
              Formulaire : nom, e-mail, téléphone facultatif, texte, pièce jointe facultative.
            </p>
          </div>
        ) : (
          <p className="text-xs text-slate-500">
            Désactivé par défaut. Activez puis enregistrez pour publier le lien.
          </p>
        )}
      </section>

      <section className="rounded-2xl border border-indigo-200 bg-indigo-50/40 p-6 space-y-4">
        <h2 className="text-lg font-bold text-slate-900">Règle direction</h2>
        <p className="text-sm text-slate-600 leading-relaxed">
          Toute demande identifiée comme « pour la direction » est <strong>déposée d&apos;abord en
          administratif</strong> (secrétariat de l&apos;établissement). La direction ne reçoit la demande
          que via un <strong>transfert manuel</strong> validé par l&apos;administratif — jamais en
          routage automatique direct.
        </p>
        <div className="rounded-xl border border-indigo-100 bg-white p-4 text-sm text-slate-700 space-y-2">
          <p className="font-bold text-slate-800">Files direction configurées</p>
          {config.directionQueues.length === 0 ? (
            <p className="text-xs text-slate-500">Aucune file direction — se synchronisent avec les sites.</p>
          ) : (
            <ul className="space-y-1.5">
              {config.directionQueues.map((q) => (
                <li key={q.id} className="flex flex-wrap items-center gap-2 text-xs">
                  <span
                    className={`rounded-full px-2 py-0.5 font-bold ${q.active ? "bg-indigo-100 text-indigo-800" : "bg-slate-100 text-slate-500"}`}
                  >
                    {q.active ? "Actif" : "Inactif"}
                  </span>
                  <span className="font-semibold">{q.label}</span>
                  {q.email ? (
                    <span className="text-slate-500">{q.email}</span>
                  ) : (
                    <span className="italic text-slate-400">e-mail directeur non renseigné</span>
                  )}
                </li>
              ))}
            </ul>
          )}
          <p className="text-[11px] text-slate-500 pt-1">
            Les e-mails directeurs se renseignent dans Paramètres → Établissements. Ici, seule la règle
            de passage par l&apos;administratif s&apos;applique automatiquement.
          </p>
        </div>
      </section>
    </div>
  );
}
