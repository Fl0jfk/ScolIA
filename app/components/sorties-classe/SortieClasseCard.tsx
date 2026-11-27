"use client";

import {
  STATUT_ELEVE_LABELS,
  dateLongue,
  heureLisible,
  type StatutEleve,
  type VueSortie,
} from "@/app/lib/sorties-classe";

const TONS: Record<StatutEleve, string> = {
  autorise: "bg-emerald-100 text-emerald-900",
  refus_parent: "bg-red-100 text-red-900",
  refus_direction: "bg-red-100 text-red-900",
  attente_parent: "bg-amber-100 text-amber-900",
  attente_direction: "bg-blue-100 text-blue-900",
  annulee: "bg-slate-100 text-slate-600",
};

export type SortieActions = {
  busy: boolean;
  direction: (sortie: VueSortie, etablissement: string, decision: "approve" | "refuse") => void;
  manuel: (sortie: VueSortie, studentId: string, nom: string, decision: "approve" | "refuse") => void;
  relancer: (sortie: VueSortie) => void;
  relancerDirection: (sortie: VueSortie, etablissement: string) => void;
  annuler: (sortie: VueSortie) => void;
};

/** Une sortie avec le statut de chaque élève. Sans `actions` : consultation seule (internat). */
export default function SortieClasseCard({
  sortie,
  actions,
  ouverte = false,
}: {
  sortie: VueSortie;
  actions?: SortieActions;
  ouverte?: boolean;
}) {
  const c = sortie.compteurs;
  const modifiable = !sortie.annulee && !sortie.partie;

  return (
    <details open={ouverte} className="rounded-2xl border border-slate-200 bg-white p-4">
      <summary className="cursor-pointer list-none">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div className="min-w-0">
            <p className="font-bold text-slate-900">{sortie.lieu}</p>
            <p className="text-sm text-slate-600">
              {dateLongue(sortie.date)} · {heureLisible(sortie.heureDepart)} →{" "}
              {heureLisible(sortie.heureRetour)}
            </p>
            <p className="text-xs text-slate-500">Organisée par {sortie.creePar}</p>
          </div>
          <div className="flex flex-wrap gap-1.5 text-xs font-bold">
            {sortie.annulee ? (
              <span className="rounded-lg bg-slate-100 px-2 py-1 text-slate-600">Annulée</span>
            ) : (
              <>
                <span className="rounded-lg bg-emerald-100 px-2 py-1 text-emerald-900">
                  {c.autorises}/{c.total} autorisés
                </span>
                {c.refuses > 0 && (
                  <span className="rounded-lg bg-red-100 px-2 py-1 text-red-900">{c.refuses} refus</span>
                )}
                {c.attenteDirection > 0 && (
                  <span className="rounded-lg bg-blue-100 px-2 py-1 text-blue-900">
                    {c.attenteDirection} accord parent, attente direction
                  </span>
                )}
                {c.attenteParent > 0 && (
                  <span className="rounded-lg bg-amber-100 px-2 py-1 text-amber-900">
                    {c.attenteParent} sans réponse parent
                  </span>
                )}
              </>
            )}
          </div>
        </div>
      </summary>

      <div className="mt-4 space-y-4 border-t border-slate-100 pt-4 text-sm">
        <p className="text-slate-700">
          {sortie.motif ? <>{sortie.motif}<br /></> : null}
          Accompagnateur aller : <strong>{sortie.accompagnateurAller}</strong> · retour :{" "}
          <strong>{sortie.accompagnateurRetour}</strong>
        </p>

        {!sortie.annulee && (
          <ul className="space-y-2">
            {sortie.directions.map((d) => (
              <li key={d.etablissement} className="flex flex-wrap items-center gap-2">
                <span className="font-semibold">Direction ({d.etablissement}) :</span>
                <span>
                  {d.status === "approved" ? "validée" : d.status === "refused" ? "refusée" : "en attente"}
                  {d.decidedBy ? ` par ${d.decidedBy}` : ""}
                  {d.note ? ` — « ${d.note} »` : ""}
                </span>
                {d.status === "pending" && (
                  <span className="text-xs text-slate-500">
                    {d.relanceLe
                      ? `(relancée le ${new Date(d.relanceLe).toLocaleString("fr-FR", { timeZone: "Europe/Paris" })})`
                      : d.emailEnvoye
                        ? "(e-mail envoyé)"
                        : "(e-mail non parti)"}
                  </span>
                )}
                {actions && sortie.gerable && modifiable && d.status === "pending" && (
                  <button
                    type="button"
                    disabled={actions.busy}
                    onClick={() => actions.relancerDirection(sortie, d.etablissement)}
                    className="rounded-lg border border-slate-200 px-3 py-1 text-xs font-bold disabled:opacity-50"
                  >
                    {d.emailEnvoye ? "Relancer la direction" : "Envoyer à la direction"}
                  </button>
                )}
                {actions && d.peutDecider && modifiable && (
                  <>
                    <button
                      type="button"
                      disabled={actions.busy}
                      onClick={() => actions.direction(sortie, d.etablissement, "approve")}
                      className="rounded-lg bg-emerald-600 px-3 py-1 text-xs font-bold text-white disabled:opacity-50"
                    >
                      Valider
                    </button>
                    <button
                      type="button"
                      disabled={actions.busy}
                      onClick={() => actions.direction(sortie, d.etablissement, "refuse")}
                      className="rounded-lg border border-red-200 px-3 py-1 text-xs font-bold text-red-700 disabled:opacity-50"
                    >
                      Refuser
                    </button>
                  </>
                )}
              </li>
            ))}
          </ul>
        )}

        <ul className="divide-y divide-slate-100 rounded-xl border border-slate-100">
          {sortie.eleves.map((e) => (
            <li key={e.studentId} className="flex flex-wrap items-center justify-between gap-2 px-3 py-2">
              <div>
                <span className="font-semibold">
                  {e.nom} {e.prenom}
                </span>{" "}
                <span className="text-slate-500">({e.classe})</span>
                <p className="text-xs text-slate-500">
                  {e.respondedAt
                    ? `Réponse le ${new Date(e.respondedAt).toLocaleString("fr-FR", { timeZone: "Europe/Paris" })}${e.saisieManuelle ? " (saisie manuelle)" : ""}`
                    : e.sansEmail
                      ? "Aucun e-mail parent : réponse à recueillir autrement"
                      : e.emailEnvoye
                        ? "E-mail envoyé aux parents"
                        : "E-mail non parti"}
                  {e.note ? ` — « ${e.note} »` : ""}
                </p>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <span className={`rounded-lg px-2 py-1 text-xs font-bold ${TONS[e.statut]}`}>
                  {STATUT_ELEVE_LABELS[e.statut]}
                </span>
                {actions && sortie.gerable && modifiable && e.parentStatus === "pending" && (
                  <>
                    <button
                      type="button"
                      disabled={actions.busy}
                      onClick={() => actions.manuel(sortie, e.studentId, `${e.prenom} ${e.nom}`, "approve")}
                      className="text-xs font-bold text-emerald-700 underline disabled:opacity-50"
                    >
                      Saisir accord
                    </button>
                    <button
                      type="button"
                      disabled={actions.busy}
                      onClick={() => actions.manuel(sortie, e.studentId, `${e.prenom} ${e.nom}`, "refuse")}
                      className="text-xs font-bold text-red-700 underline disabled:opacity-50"
                    >
                      Saisir refus
                    </button>
                  </>
                )}
                {actions && sortie.gerable && modifiable && e.parentStatus === "authorized" && (
                  <button
                    type="button"
                    disabled={actions.busy}
                    onClick={() => actions.manuel(sortie, e.studentId, `${e.prenom} ${e.nom}`, "refuse")}
                    className="text-xs font-bold text-red-700 underline disabled:opacity-50"
                  >
                    Retirer l&apos;accord
                  </button>
                )}
              </div>
            </li>
          ))}
        </ul>

        {actions && sortie.gerable && modifiable && (
          <div className="flex flex-wrap gap-3">
            <button
              type="button"
              disabled={actions.busy}
              onClick={() => actions.relancer(sortie)}
              className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold disabled:opacity-50"
            >
              Relancer les parents sans réponse
            </button>
            <button
              type="button"
              disabled={actions.busy}
              onClick={() => actions.annuler(sortie)}
              className="rounded-xl border border-red-200 px-3 py-2 text-xs font-bold text-red-700 disabled:opacity-50"
            >
              Annuler la sortie
            </button>
          </div>
        )}
      </div>
    </details>
  );
}
