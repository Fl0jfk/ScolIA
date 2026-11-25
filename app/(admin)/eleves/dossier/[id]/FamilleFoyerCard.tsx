"use client";

import Link from "@/app/components/ui/AppLink";
import ResponsableEditor from "@/app/(admin)/eleves/dossiers/foyers/ResponsableEditor";
import {
  formatFamilleTitre,
  formatFoyerAdresse,
  formatFoyerPayeurDetail,
  formatPersonneNom,
  initialesDe,
} from "@/app/lib/foyer-display";

export type FamilleFoyer = {
  id: string;
  label: string;
  adresse: string | null;
  codePostal: string | null;
  ville: string | null;
  payeurEstFoyer: boolean;
  relation: string;
  enfants?: Array<{ id: string; nom: string; prenom: string; classe: string | null; photoUrl?: string | null }>;
  responsables: Array<{
    id: string;
    nom: string;
    prenom: string;
    email: string | null;
    telephone: string | null;
    autoriteParentale: boolean;
    contactUrgence: boolean;
    payeur: boolean;
    rang: number;
    compteFamille?: boolean;
    peutRecuperer?: boolean;
    lien?: string | null;
  }>;
};

const TONES = {
  slate: "bg-slate-100 text-slate-700",
  indigo: "bg-indigo-50 text-indigo-700",
  amber: "bg-amber-50 text-amber-800",
  emerald: "bg-emerald-50 text-emerald-800",
} as const;

function Pastille({ children, tone = "slate" }: { children: string; tone?: keyof typeof TONES }) {
  return <span className={`rounded-full px-2 py-0.5 text-[11px] font-semibold ${TONES[tone]}`}>{children}</span>;
}

function MailIcon() {
  return (
    <svg aria-hidden viewBox="0 0 20 20" className="h-3.5 w-3.5 shrink-0" fill="none" stroke="currentColor" strokeWidth="1.6">
      <rect x="2.5" y="4.5" width="15" height="11" rx="2" />
      <path d="m3 6 7 5 7-5" />
    </svg>
  );
}

function PhoneIcon() {
  return (
    <svg aria-hidden viewBox="0 0 20 20" className="h-3.5 w-3.5 shrink-0" fill="none" stroke="currentColor" strokeWidth="1.6">
      <path d="M5 3h3l1.5 4-2 1.2a9 9 0 0 0 4.3 4.3L13 10.5l4 1.5v3a2 2 0 0 1-2 2A12 12 0 0 1 3 5a2 2 0 0 1 2-2Z" />
    </svg>
  );
}

/** Miniature ronde d'un enfant : photo (URL pré-signée courte) ou initiales. */
function Miniature({ prenom, nom, photoUrl }: { prenom: string; nom: string; photoUrl?: string | null }) {
  const alt = `Photo de ${formatPersonneNom({ prenom, nom })}`;
  return photoUrl ? (
    // eslint-disable-next-line @next/next/no-img-element
    <img
      src={photoUrl}
      alt={alt}
      loading="lazy"
      decoding="async"
      referrerPolicy="no-referrer"
      className="h-9 w-9 shrink-0 rounded-full bg-slate-100 object-cover"
    />
  ) : (
    <span
      role="img"
      aria-label={alt}
      className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-indigo-100 text-xs font-bold text-indigo-700"
    >
      {initialesDe({ prenom, nom })}
    </span>
  );
}

export default function FamilleFoyerCard({
  foyer: f,
  eleve,
  canEdit,
  showFinances,
  onOpenFinances,
  editRespId,
  onEditResp,
  onRespSaved,
  onAddResp,
}: {
  foyer: FamilleFoyer;
  eleve: { id: string; prenom: string; nom: string };
  canEdit: boolean;
  showFinances: boolean;
  onOpenFinances: () => void;
  editRespId: string | null;
  onEditResp: (id: string | null) => void;
  onRespSaved: () => void;
  onAddResp: () => void;
}) {
  const titre = formatFamilleTitre(f);
  const adresse = formatFoyerAdresse(f);
  const sousTitre = adresse || (f.label.trim() && f.label.trim() !== titre ? f.label.trim() : "");
  const eleveLabel = formatPersonneNom(eleve);

  return (
    <article aria-label={titre} className="rounded-3xl border border-slate-200 bg-white p-4 shadow-sm sm:p-6">
      <header className="mb-3 flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <h3 className="text-base font-bold text-slate-900">{titre}</h3>
          {sousTitre ? <p className="text-xs text-slate-500">{sousTitre}</p> : null}
          {f.relation !== "principal" ? (
            <span className="mt-1 inline-block rounded-full bg-slate-100 px-2 py-0.5 text-[10px] font-bold uppercase text-slate-600">
              {f.relation}
            </span>
          ) : null}
        </div>
        {showFinances ? (
          <button
            type="button"
            onClick={onOpenFinances}
            className="rounded-xl border border-indigo-200 bg-indigo-50 px-3 py-1.5 text-xs font-bold text-indigo-700 hover:bg-indigo-100"
          >
            IBAN · facturation →
          </button>
        ) : null}
      </header>
      <p className="mb-3 text-xs text-slate-600">{formatFoyerPayeurDetail(f)}</p>

      <ul className="space-y-2" aria-label={`Responsables — ${titre}`}>
        {f.responsables.map((r) => (
          <li key={r.id}>
            {editRespId === r.id ? (
              <ResponsableEditor
                resp={r}
                eleveId={eleve.id}
                eleveLabel={eleveLabel}
                onCancel={() => onEditResp(null)}
                onSaved={onRespSaved}
              />
            ) : (
              <div className="flex gap-3 rounded-2xl bg-slate-50 p-3">
                <span
                  aria-hidden
                  className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-indigo-600 text-sm font-bold text-white"
                >
                  {initialesDe(r)}
                </span>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                    <span className="font-semibold text-slate-900">{formatPersonneNom(r)}</span>
                    {r.lien ? <Pastille tone="indigo">{r.lien}</Pastille> : null}
                  </div>
                  <div className="mt-1 flex flex-wrap gap-1">
                    {r.autoriteParentale ? <Pastille>Autorité parentale</Pastille> : null}
                    {!f.payeurEstFoyer && r.payeur ? <Pastille>Payeur</Pastille> : null}
                    {r.contactUrgence ? <Pastille tone="amber">Contact d'urgence</Pastille> : null}
                    {r.peutRecuperer ? <Pastille tone="emerald">Peut récupérer l'enfant</Pastille> : null}
                    {r.compteFamille ? <Pastille tone="emerald">Compte famille</Pastille> : null}
                  </div>
                  <div className="mt-2 flex flex-col gap-1 text-xs text-slate-700 sm:flex-row sm:flex-wrap sm:gap-x-4">
                    {r.email ? (
                      <a
                        href={`mailto:${r.email}`}
                        className="inline-flex min-w-0 items-center gap-1.5 break-all text-indigo-700 hover:underline"
                      >
                        <MailIcon />
                        {r.email}
                      </a>
                    ) : (
                      <span className="text-slate-400">E-mail non renseigné</span>
                    )}
                    {r.telephone ? (
                      <a
                        href={`tel:${r.telephone.replace(/[^\d+]/g, "")}`}
                        className="inline-flex items-center gap-1.5 text-indigo-700 hover:underline"
                      >
                        <PhoneIcon />
                        {r.telephone}
                      </a>
                    ) : (
                      <span className="text-slate-400">Téléphone non renseigné</span>
                    )}
                  </div>
                </div>
                {canEdit ? (
                  <button
                    type="button"
                    aria-label={`Modifier ${formatPersonneNom(r)}`}
                    className="h-8 shrink-0 rounded-xl border border-slate-200 bg-white px-3 text-xs font-bold text-indigo-600 hover:bg-indigo-50"
                    onClick={() => onEditResp(r.id)}
                  >
                    Modifier
                  </button>
                ) : null}
              </div>
            )}
          </li>
        ))}
      </ul>
      {canEdit && f.responsables.length < 4 ? (
        <button type="button" className="mt-3 text-xs font-bold text-indigo-600" onClick={onAddResp}>
          + Ajouter un responsable
        </button>
      ) : null}

      {f.enfants && f.enfants.length > 0 ? (
        <div className="mt-4">
          <p className="mb-2 text-xs font-semibold text-slate-500">Enfants du foyer ({f.enfants.length})</p>
          <ul className="flex flex-wrap gap-2">
            {f.enfants.map((k) => {
              const courant = k.id === eleve.id;
              return (
                <li key={k.id}>
                  <Link
                    href={`/eleves/dossier/${k.id}`}
                    aria-current={courant ? "page" : undefined}
                    className={`flex items-center gap-2 rounded-full py-1 pl-1 pr-3 text-xs font-semibold ${
                      courant
                        ? "bg-indigo-600 text-white ring-2 ring-indigo-200"
                        : "bg-indigo-50 text-indigo-800 hover:bg-indigo-100"
                    }`}
                  >
                    <Miniature prenom={k.prenom} nom={k.nom} photoUrl={k.photoUrl} />
                    <span>
                      {k.prenom} {k.nom.toUpperCase()}
                      {k.classe ? ` · ${k.classe}` : ""}
                    </span>
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ) : null}
    </article>
  );
}
