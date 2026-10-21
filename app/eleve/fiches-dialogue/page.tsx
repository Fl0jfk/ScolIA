import Link from "next/link";
import { redirect } from "next/navigation";
import { requireAppUser } from "@/app/lib/app-session";
import {
  findFdEleveForUser,
  listFdEleveFiches,
  type FdEleveFicheView,
} from "@/app/lib/fiches-dialogue-workflow";
import { hasEleveRole } from "@/app/lib/intranet-role-utils";
import { requireTenantId } from "@/app/lib/tenant-scope";

export const metadata = {
  title: "Mes fiches de dialogue",
};

const STATUT_LABELS: Record<string, string> = {
  a_envoyer: "À venir",
  en_attente_famille: "Réponse de la famille attendue",
  saisie_recue: "Réponse enregistrée",
  en_attente_accord_parent2: "Accord de l’autre responsable attendu",
  en_conflit: "Désaccord familial en cours d’examen",
  en_conseil: "Examen par le conseil de classe",
  en_attente_direction: "Avis en attente de publication",
  decision_envoyee: "Avis du conseil communiqué",
  en_attente_acceptation: "Acceptation de la famille attendue",
  acceptee: "Décision acceptée",
  refusee: "Décision refusée",
  en_appel: "Appel en cours",
  cloturee: "Dossier clôturé",
  figee: "Dossier figé",
};

function formatDate(value: string | null): string {
  if (!value) return "—";
  return new Date(value).toLocaleString("fr-FR", {
    dateStyle: "long",
    timeStyle: "short",
  });
}

function AuteurLabel({ role }: { role: string }) {
  return (
    <span className="rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-700">
      {role === "conseil" ? "Conseil de classe" : "Famille"}
    </span>
  );
}

function FicheCard({ fiche }: { fiche: FdEleveFicheView }) {
  return (
    <article className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h2 className="text-lg font-semibold text-slate-900">{fiche.campagneLabel}</h2>
          <p className="text-sm text-slate-500">
            {fiche.anneeLabel}
            {fiche.etapeLabel ? ` · ${fiche.etapeLabel}` : ""}
          </p>
        </div>
        <span className="rounded-full bg-slate-900 px-3 py-1 text-xs font-semibold text-white">
          {STATUT_LABELS[fiche.statut] ?? fiche.statut}
        </span>
      </div>

      <p className="mt-3 text-xs text-slate-500">
        Dernière mise à jour : {formatDate(fiche.updatedAt)}
        {fiche.closedAt ? ` · Clôturé le ${formatDate(fiche.closedAt)}` : ""}
      </p>

      <div className="mt-4 space-y-4">
        {fiche.reponses.length === 0 ? (
          <p className="rounded-xl bg-slate-50 p-4 text-sm text-slate-600">
            Aucune réponse n’a encore été déposée pour cette campagne.
          </p>
        ) : (
          fiche.reponses.map((reponse) => (
            <section key={reponse.id} className="rounded-xl border border-slate-100 p-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <h3 className="text-sm font-semibold text-slate-800">{reponse.etapeLabel}</h3>
                  <AuteurLabel role={reponse.auteurRole} />
                </div>
                <span className="text-xs text-slate-500">{formatDate(reponse.submittedAt)}</span>
              </div>
              <div className="mt-3 space-y-3">
                {reponse.sections.map((section, index) => (
                  <div key={`${section.title}-${index}`}>
                    <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
                      {section.title}
                    </p>
                    {section.lines.length > 0 && (
                      <ul className="mt-1 space-y-1 text-sm text-slate-700">
                        {section.lines.map((line, lineIndex) => (
                          <li key={lineIndex}>{line}</li>
                        ))}
                      </ul>
                    )}
                    {section.checks && section.checks.length > 0 && (
                      <ul className="mt-1 space-y-1 text-sm text-slate-700">
                        {section.checks.map((check) => (
                          <li key={check.label}>
                            <span className="mr-1 font-semibold">{check.checked ? "✓" : "○"}</span>
                            {check.label}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>
                ))}
              </div>
            </section>
          ))
        )}
      </div>

      {fiche.appel && (
        <section className="mt-4 rounded-xl border border-amber-200 bg-amber-50 p-4">
          <h3 className="text-sm font-semibold text-amber-950">Décision d’appel</h3>
          <p className="mt-1 text-sm text-amber-900">
            {fiche.appel.decision === "reformation"
              ? "La commission a réformé la décision."
              : fiche.appel.decision === "maintien"
                ? "La commission a maintenu la décision."
                : "Décision d’appel en attente."}
          </p>
          {fiche.appel.destinationProposee && (
            <p className="mt-1 text-sm text-amber-900">
              Orientation retenue : {fiche.appel.destinationProposee}
            </p>
          )}
          {fiche.appel.motif && <p className="mt-1 text-sm text-amber-900">{fiche.appel.motif}</p>}
          {fiche.appel.closedAt && (
            <p className="mt-2 text-xs text-amber-800">
              Clôturé le {formatDate(fiche.appel.closedAt)}
              {fiche.appel.notifiedAt ? " · information envoyée à la famille" : ""}
            </p>
          )}
        </section>
      )}
    </article>
  );
}

export default async function EleveFichesDialoguePage() {
  const scope = await requireTenantId();
  if (!scope.ok) redirect("/sign-in");
  const appUser = await requireAppUser();
  if (!appUser.ok) redirect("/sign-in");

  if (!hasEleveRole(appUser.user.roles)) {
    redirect("/dashboard");
  }

  const eleve = await findFdEleveForUser({
    etablissementId: scope.ctx.etablissementId,
    email: appUser.user.email,
    externalUserId: appUser.user.externalUserId,
  });
  const fiches = eleve ? await listFdEleveFiches(scope.ctx.etablissementId, eleve.id) : [];

  return (
    <main className="mx-auto w-full max-w-3xl px-4 py-8 sm:px-6">
      <div className="mb-6 flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
            Espace élève
          </p>
          <h1 className="text-2xl font-semibold text-slate-900">Mes fiches de dialogue</h1>
          {eleve && (
            <p className="mt-1 text-sm text-slate-600">
              {eleve.prenom} {eleve.nom}
              {eleve.classe ? ` · ${eleve.classe}` : ""}
            </p>
          )}
        </div>
        <Link
          href="/dashboard"
          className="rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-medium text-slate-700 shadow-sm hover:bg-slate-50"
        >
          ← Accueil
        </Link>
      </div>

      <p className="mb-5 rounded-xl border border-slate-200 bg-slate-50 px-4 py-3 text-sm text-slate-600">
        Consultation en lecture seule. Les réponses et décisions sont gérées par les responsables
        légaux et l’établissement ; aucune modification n’est possible depuis cet écran.
      </p>

      {!eleve ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-6 text-sm text-slate-700 shadow-sm">
          Aucun dossier élève n’est rattaché à votre compte. Si vous pensez qu’il s’agit d’une
          erreur, contactez la vie scolaire ou le secrétariat.
        </div>
      ) : fiches.length === 0 ? (
        <div className="rounded-2xl border border-slate-200 bg-white p-6 text-sm text-slate-700 shadow-sm">
          Aucune fiche de dialogue n’est disponible pour le moment.
        </div>
      ) : (
        <div className="space-y-4">
          {fiches.map((fiche) => (
            <FicheCard key={fiche.id} fiche={fiche} />
          ))}
        </div>
      )}
    </main>
  );
}
