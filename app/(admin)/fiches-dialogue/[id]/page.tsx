"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useState } from "react";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

type Etape = {
  id: string;
  ordre: number;
  kind: string;
  label: string;
  description: string | null;
  optionnelle: boolean;
  gelee: boolean;
  conseilDate: string | null;
  opensAt: string | null;
  closesAt: string | null;
};

type Fiche = {
  id: string;
  eleveNom: string;
  elevePrenom: string;
  classeActuelle: string;
  statut: string;
  etapeCouranteId: string | null;
  parentEmailCount: number;
  lastSentAt: string | null;
  parentAccordStatut: string | null;
};

type Campagne = {
  id: string;
  label: string;
  anneeLabel: string;
  calendrierMode: string;
  statut: string;
  delaiFamilleJours: number;
  starterMode?: string;
  niveauActuel?: string | null;
  templateKey?: string | null;
  contactPpLabel?: string | null;
  classesCibles: string[];
  catalogue: {
    destinations: Array<{ id: string; label: string }>;
    options: Array<{ id: string; label: string }>;
  };
  appelConfig: {
    enabled: boolean;
    dateLimite?: string;
    procedureHtml?: string;
    documentsLabels?: string[];
    contactPpLabel?: string;
  };
};

type Archive = {
  id: string;
  ficheId: string;
  kind: string;
  title: string;
  createdAt: string;
  storage: "s3" | "db" | "none";
};

type DossierSection = {
  title: string;
  lines: string[];
  checks?: Array<{ label: string; checked: boolean }>;
};

type DossierReponse = {
  id: string;
  etapeId: string;
  etapeLabel: string;
  etapeKind: string;
  auteurRole: string;
  auteurLabel: string | null;
  submittedAt: string;
  payload: {
    values?: Record<string, string | string[] | boolean | null>;
    destinationProposee?: string;
    [key: string]: unknown;
  };
  sections: DossierSection[];
};

type FicheDossier = {
  fiche: {
    id: string;
    eleveNom: string;
    elevePrenom: string;
    classeActuelle: string;
    statut: string;
    parentEmails: string[];
    lastSentAt: string | null;
    lastReminderAt: string | null;
    reminderCount: number;
    parentAccord: {
      statutAutre?: string;
      deposantEmail?: string;
      autreEmail?: string;
      conflictMotif?: string;
      resolution?: string;
      resolutionNote?: string;
      conflictNotifiedAt?: string;
      taciteNotifiedAt?: string;
    } | null;
    appel: AppelPayload | null;
  };
  campagne: { label: string; anneeLabel: string };
  etapes: Array<{ id: string; ordre: number; kind: string; label: string; closesAt: string | null; gelee: boolean }>;
  reponses: DossierReponse[];
  signatures: Array<{
    id: string;
    etapeId: string;
    role: string;
    signerName: string;
    signerEmail: string | null;
    method: string;
    signedAt: string;
  }>;
  archives: Archive[];
  tokens: Array<{
    id: string;
    etapeId: string | null;
    email: string | null;
    purpose: string;
    expiresAt: string | null;
    usedAt: string | null;
    revokedAt: string | null;
    createdAt: string;
  }>;
};

type AppelDocument = {
  label: string;
  reference?: string;
  statut?: "attendu" | "recu";
  receivedAt?: string;
  notes?: string;
};

type AppelPayload = {
  commissionLabel?: string;
  saisineDate?: string;
  commissionDate?: string;
  receivedAt?: string;
  decision?: "maintien" | "reformation";
  motif?: string;
  destinationProposee?: string;
  informations?: string;
  documents?: AppelDocument[];
  decidedAt?: string;
  closedAt?: string;
  notifiedAt?: string;
  updatedAt?: string;
  updatedBy?: string;
};

function isConseilKind(kind: string | undefined): boolean {
  return kind === "conseil" || kind === "decision_finale_conseil";
}

function isFamilleKind(kind: string | undefined): boolean {
  return kind === "saisie_famille" || kind === "choix_definitifs" || kind === "acceptation_famille";
}

const STATUT_LABELS: Record<string, string> = {
  a_envoyer: "À envoyer",
  en_attente_famille: "En attente famille",
  saisie_recue: "Saisie reçue",
  en_attente_accord_parent2: "Accord 2e parent",
  en_conflit: "Conflit parents",
  en_conseil: "En conseil",
  en_attente_direction: "Attente direction",
  decision_envoyee: "Décision envoyée",
  en_attente_acceptation: "Acceptation en attente",
  acceptee: "Acceptée",
  refusee: "Refusée",
  en_appel: "En appel",
  cloturee: "Clôturée",
  figee: "Figée",
};

function formatDossierDate(value: string | null | undefined): string {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
}

function DossierPanel({
  dossier,
  busy,
  onClose,
}: {
  dossier: FicheDossier;
  busy: boolean;
  onClose: () => void;
}) {
  const accord = dossier.fiche.parentAccord;
  return (
    <ModuleCard bodyClassName="space-y-4 p-5">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h2 className={`text-lg font-semibold ${dash.ink}`}>
            Dossier famille — {dossier.fiche.elevePrenom} {dossier.fiche.eleveNom}
          </h2>
          <p className={`text-sm ${dash.textMid}`}>
            {dossier.fiche.classeActuelle || "Classe non renseignée"} ·{" "}
            {dossier.campagne.label} · {STATUT_LABELS[dossier.fiche.statut] ?? dossier.fiche.statut}
          </p>
        </div>
        <button
          type="button"
          className="rounded-lg border border-slate-200 px-3 py-1.5 text-sm font-medium text-slate-700 hover:bg-slate-50"
          onClick={onClose}
        >
          Fermer
        </button>
      </div>

      {busy && <p className="text-sm text-slate-500">Chargement du dossier…</p>}

      <div className="grid gap-3 text-sm sm:grid-cols-2">
        <div className="rounded-xl bg-slate-50 p-3">
          <p className="text-xs font-semibold uppercase text-slate-500">Responsables</p>
          <p className="mt-1 text-slate-800">
            {dossier.fiche.parentEmails.length
              ? dossier.fiche.parentEmails.join(", ")
              : "Aucun e-mail connu"}
          </p>
        </div>
        <div className="rounded-xl bg-slate-50 p-3">
          <p className="text-xs font-semibold uppercase text-slate-500">Dernier envoi réussi</p>
          <p className="mt-1 text-slate-800">
            {formatDossierDate(dossier.fiche.lastSentAt)} · {dossier.fiche.reminderCount} relance(s)
          </p>
        </div>
      </div>

      {accord && (
        <div className="rounded-xl border border-slate-200 p-3 text-sm">
          <p className="text-xs font-semibold uppercase text-slate-500">Accord parental</p>
          <p className="mt-1 text-slate-800">
            {accord.statutAutre === "confirme"
              ? "Les deux responsables ont confirmé."
              : accord.statutAutre === "tacite"
                ? "Accord tacite : le délai de confirmation est écoulé."
                : accord.statutAutre === "contredit"
                  ? "Désaccord signalé : arbitrage direction requis."
                  : "Confirmation du second responsable en attente."}
          </p>
          {accord.deposantEmail && (
            <p className="mt-1 text-slate-600">Déposant : {accord.deposantEmail}</p>
          )}
          {accord.autreEmail && (
            <p className="text-slate-600">Second responsable : {accord.autreEmail}</p>
          )}
          {accord.conflictMotif && (
            <p className="mt-1 text-amber-900">Motif du désaccord : {accord.conflictMotif}</p>
          )}
          {accord.resolution && (
            <p className="mt-1 text-slate-700">
              Résolution : {accord.resolution}
              {accord.resolutionNote ? ` — ${accord.resolutionNote}` : ""}
            </p>
          )}
          {accord.conflictNotifiedAt && (
            <p className="mt-1 text-xs text-slate-500">
              Alerte désaccord envoyée : {formatDossierDate(accord.conflictNotifiedAt)}
            </p>
          )}
          {accord.taciteNotifiedAt && (
            <p className="mt-1 text-xs text-slate-500">
              Information accord tacite envoyée : {formatDossierDate(accord.taciteNotifiedAt)}
            </p>
          )}
        </div>
      )}

      <div className="space-y-3">
        <h3 className={`text-sm font-semibold ${dash.ink}`}>Réponses et avis</h3>
        {dossier.reponses.length === 0 ? (
          <p className="rounded-xl bg-slate-50 p-3 text-sm text-slate-600">
            Aucune réponse déposée pour le moment.
          </p>
        ) : (
          dossier.reponses.map((reponse) => (
            <article key={reponse.id} className="rounded-xl border border-slate-200 p-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-sm font-semibold text-slate-800">
                  {reponse.etapeLabel} —{" "}
                  {reponse.auteurRole === "conseil"
                    ? "Conseil de classe"
                    : reponse.auteurLabel || "Famille"}
                </p>
                <span className="text-xs text-slate-500">
                  {formatDossierDate(reponse.submittedAt)}
                </span>
              </div>
              <div className="mt-2 space-y-2">
                {reponse.sections.map((section, index) => (
                  <div key={`${section.title}-${index}`}>
                    <p className="text-xs font-semibold uppercase tracking-wide text-slate-500">
                      {section.title}
                    </p>
                    {section.lines.length > 0 && (
                      <ul className="mt-1 space-y-0.5 text-sm text-slate-700">
                        {section.lines.map((line, lineIndex) => (
                          <li key={lineIndex}>{line}</li>
                        ))}
                      </ul>
                    )}
                    {section.checks && section.checks.length > 0 && (
                      <ul className="mt-1 space-y-0.5 text-sm text-slate-700">
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
            </article>
          ))
        )}
      </div>

      {dossier.signatures.length > 0 && (
        <div className="rounded-xl bg-slate-50 p-3 text-sm">
          <p className="text-xs font-semibold uppercase text-slate-500">Signatures</p>
          <ul className="mt-1 space-y-1 text-slate-700">
            {dossier.signatures.map((s) => (
              <li key={s.id}>
                {s.signerName} ({s.role === "direction" ? "direction" : s.role}) —{" "}
                {formatDossierDate(s.signedAt)}
              </li>
            ))}
          </ul>
        </div>
      )}

      {dossier.archives.length > 0 && (
        <div className="rounded-xl bg-slate-50 p-3 text-sm">
          <p className="text-xs font-semibold uppercase text-slate-500">Documents</p>
          <ul className="mt-1 space-y-1">
            {dossier.archives.map((a) => (
              <li key={a.id}>
                <a
                  className="text-slate-800 underline decoration-slate-300 underline-offset-2 hover:decoration-slate-600"
                  href={`/api/fiches-dialogue/archives/${a.id}`}
                  target="_blank"
                  rel="noreferrer"
                >
                  {a.title}
                </a>
                <span className="ml-2 text-xs text-slate-500">
                  {a.storage === "db" ? "repli BDD" : a.storage === "s3" ? "S3" : "indisponible"}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      <details className="rounded-xl border border-slate-200 p-3 text-sm">
        <summary className="cursor-pointer font-medium text-slate-700">
          Historique des liens ({dossier.tokens.length})
        </summary>
        <ul className="mt-2 space-y-1 text-xs text-slate-600">
          {dossier.tokens.map((t) => (
            <li key={t.id}>
              {t.purpose} · {t.email || "sans adresse"} · créé le {formatDossierDate(t.createdAt)}
              {t.revokedAt ? " · révoqué" : t.usedAt ? " · utilisé" : " · actif"}
              {t.expiresAt ? ` · expire le ${formatDossierDate(t.expiresAt)}` : ""}
            </li>
          ))}
        </ul>
      </details>
    </ModuleCard>
  );
}

export default function FichesDialogueCampagnePage() {
  const params = useParams();
  const id = String(params.id || "");
  const [campagne, setCampagne] = useState<Campagne | null>(null);
  const [etapes, setEtapes] = useState<Etape[]>([]);
  const [fiches, setFiches] = useState<Fiche[]>([]);
  const [stats, setStats] = useState<{ total: number; byStatut: Record<string, number> } | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filtreClasse, setFiltreClasse] = useState("");
  const [conseilFicheId, setConseilFicheId] = useState<string | null>(null);
  const [conseilEtapeId, setConseilEtapeId] = useState<string | null>(null);
  const [conseilAvenant, setConseilAvenant] = useState(false);
  const [conflitFicheId, setConflitFicheId] = useState<string | null>(null);
  const [conflitResolution, setConflitResolution] = useState<
    "maintien_famille" | "avis_conseil"
  >("maintien_famille");
  const [conflitNote, setConflitNote] = useState("");
  const [archives, setArchives] = useState<Archive[]>([]);
  const [dossier, setDossier] = useState<FicheDossier | null>(null);
  const [dossierBusy, setDossierBusy] = useState(false);
  const [conseilForm, setConseilForm] = useState({
    avis: "favorable" as "favorable" | "reserve" | "defavorable" | "autre",
    destinationProposee: "",
    motif: "",
    commentaire: "",
    ppName: "",
    directionName: "",
  });
  const [appelForm, setAppelForm] = useState({
    dateLimite: "",
    procedureHtml: "",
    documentsText: "",
    contactPpLabel: "",
  });
  const [appelDirty, setAppelDirty] = useState(false);
  const [appelFiche, setAppelFiche] = useState<Fiche | null>(null);
  const [appelDossier, setAppelDossier] = useState<AppelPayload | null>(null);
  const [appelDossierForm, setAppelDossierForm] = useState({
    commissionLabel: "",
    saisineDate: "",
    commissionDate: "",
    receivedAt: "",
    decision: "" as "" | "maintien" | "reformation",
    motif: "",
    destinationProposee: "",
    informations: "",
    documents: [] as AppelDocument[],
  });
  const [appelDossierBusy, setAppelDossierBusy] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, { cache: "no-store" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Erreur");
      setCampagne(json.campagne);
      if (!appelDirty && json.campagne?.appelConfig) {
        const ac = json.campagne.appelConfig as Campagne["appelConfig"];
        setAppelForm({
          dateLimite: ac.dateLimite || "",
          procedureHtml: ac.procedureHtml || "",
          documentsText: (ac.documentsLabels || []).join("\n"),
          contactPpLabel: ac.contactPpLabel || json.campagne.contactPpLabel || "",
        });
      }
      setEtapes(json.etapes || []);
      setFiches(json.fiches || []);
      setStats(json.stats || null);
      setArchives(json.archives || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => {
    if (id) void reload();
  }, [id, reload]);

  const classes = useMemo(() => {
    const set = new Set(fiches.map((f) => f.classeActuelle || "—"));
    return [...set].sort((a, b) => a.localeCompare(b, "fr"));
  }, [fiches]);

  const fichesFiltrees = useMemo(() => {
    if (!filtreClasse) return fiches;
    return fiches.filter((f) => f.classeActuelle === filtreClasse);
  }, [fiches, filtreClasse]);

  const etapeCouranteDominante = useMemo(() => {
    const counts = new Map<string, number>();
    for (const f of fiches) {
      if (!f.etapeCouranteId) continue;
      counts.set(f.etapeCouranteId, (counts.get(f.etapeCouranteId) ?? 0) + 1);
    }
    let best: string | null = null;
    let n = 0;
    for (const [k, v] of counts) {
      if (v > n) {
        best = k;
        n = v;
      }
    }
    return etapes.find((e) => e.id === best) ?? etapes[0] ?? null;
  }, [fiches, etapes]);

  const etapeById = useMemo(() => new Map(etapes.map((e) => [e.id, e])), [etapes]);

  function etapeOfFiche(f: Fiche): Etape | null {
    return f.etapeCouranteId ? etapeById.get(f.etapeCouranteId) ?? null : null;
  }

  /** Étape famille la plus basse encore active — c’est elle qu’on gèle vers le conseil. */
  const etapeFamillePourGel = useMemo(() => {
    const candidates: Etape[] = [];
    for (const f of fiches) {
      const e = f.etapeCouranteId ? etapeById.get(f.etapeCouranteId) : undefined;
      if (e && isFamilleKind(e.kind)) candidates.push(e);
    }
    if (!candidates.length) {
      return etapeCouranteDominante && isFamilleKind(etapeCouranteDominante.kind)
        ? etapeCouranteDominante
        : null;
    }
    return candidates.slice().sort((a, b) => a.ordre - b.ordre)[0] ?? null;
  }, [fiches, etapeById, etapeCouranteDominante]);

  const fichesPretesPourGel = useMemo(() => {
    if (!etapeFamillePourGel) return 0;
    return fiches.filter(
      (f) => f.etapeCouranteId === etapeFamillePourGel.id && f.statut === "saisie_recue",
    ).length;
  }, [fiches, etapeFamillePourGel]);

  const fichesBloqueesPourGel = useMemo(() => {
    if (!etapeFamillePourGel) return [];
    return fiches.filter(
      (f) =>
        f.etapeCouranteId === etapeFamillePourGel.id &&
        f.statut !== "saisie_recue" &&
        f.statut !== "acceptee" &&
        f.statut !== "refusee" &&
        f.statut !== "cloturee" &&
        f.statut !== "figee",
    );
  }, [fiches, etapeFamillePourGel]);

  async function loadDossier(ficheId: string): Promise<FicheDossier | null> {
    setDossierBusy(true);
    try {
      const res = await fetch(`/api/fiches-dialogue/fiches/${ficheId}`, {
        cache: "no-store",
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Dossier introuvable");
      return json as FicheDossier;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      return null;
    } finally {
      setDossierBusy(false);
    }
  }

  async function openDossier(f: Fiche) {
    setMessage(null);
    setError(null);
    const d = await loadDossier(f.id);
    if (d) setDossier(d);
  }

  async function openConseil(f: Fiche, avenant: boolean) {
    const courante = etapeOfFiche(f);
    const etape = avenant
      ? etapes
          .filter(
            (e) => isConseilKind(e.kind) && (!courante || e.ordre <= courante.ordre),
          )
          .slice()
          .sort((a, b) => b.ordre - a.ordre)[0] ?? null
      : courante;
    if (!etape) {
      setError("Aucune étape de conseil disponible pour cette fiche.");
      return;
    }
    setConseilFicheId(f.id);
    setConseilEtapeId(etape.id);
    setConseilAvenant(avenant);

    /**
     * Le PP doit voir la réponse de la famille avant de décider. On charge le
     * dossier et on pré-remplit la destination avec le dernier vœu connu (ou la
     * dernière proposition en cas d’avenant), sans écraser le reste du formulaire.
     */
    const d = await loadDossier(f.id);
    if (d) setDossier(d);
    const reponsesFamille = (d?.reponses ?? []).filter(
      (r) => r.auteurRole === "famille" && r.payload.values,
    );
    const reponsesConseil = (d?.reponses ?? []).filter((r) => r.auteurRole === "conseil");
    const destination = avenant
      ? reponsesConseil[reponsesConseil.length - 1]?.payload.destinationProposee || ""
      : reponsesFamille[reponsesFamille.length - 1]?.payload.values?.destination || "";
    setConseilForm((prev) => ({
      ...prev,
      destinationProposee:
        typeof destination === "string" && destination
          ? destination
          : campagne?.catalogue.destinations[0]?.id || "",
    }));
  }

  async function openAppelDossier(f: Fiche) {
    setAppelFiche(f);
    setAppelDossierBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/fiches/${f.id}/appel`, {
        cache: "no-store",
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Dossier d’appel introuvable");
      const appel = (json.appel ?? null) as AppelPayload | null;
      setAppelDossier(appel);
      setAppelDossierForm({
        commissionLabel: appel?.commissionLabel || "",
        saisineDate: appel?.saisineDate ? appel.saisineDate.slice(0, 10) : "",
        commissionDate: appel?.commissionDate ? appel.commissionDate.slice(0, 10) : "",
        receivedAt: appel?.receivedAt ? appel.receivedAt.slice(0, 10) : "",
        decision: appel?.decision ?? "",
        motif: appel?.motif || "",
        destinationProposee: appel?.destinationProposee || "",
        informations: appel?.informations || "",
        documents: (appel?.documents ?? []).map((d) => ({ ...d })),
      });
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setAppelDossierBusy(false);
    }
  }

  async function saveAppelDossier(close: boolean) {
    if (!appelFiche) return;
    if (close && !appelDossierForm.decision) {
      setError("Choisissez la décision de la commission avant de clôturer.");
      return;
    }
    if (close && !appelDossierForm.motif.trim()) {
      setError("Le motif de la décision est obligatoire pour clôturer.");
      return;
    }
    setAppelDossierBusy(true);
    setError(null);
    setMessage(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/fiches/${appelFiche.id}/appel`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: close ? "close" : "save",
          patch: {
            // Chaîne vide = effacement explicite ; champ absent = on conserve l’existant.
            commissionLabel: appelDossierForm.commissionLabel,
            saisineDate: appelDossierForm.saisineDate,
            commissionDate: appelDossierForm.commissionDate,
            receivedAt: appelDossierForm.receivedAt,
            decision: appelDossierForm.decision || undefined,
            motif: appelDossierForm.motif,
            destinationProposee: appelDossierForm.destinationProposee,
            informations: appelDossierForm.informations,
            documents: appelDossierForm.documents
              .map((d) => ({
                label: d.label.trim(),
                reference: d.reference?.trim() || undefined,
                statut: d.statut || "attendu",
                receivedAt: d.receivedAt || undefined,
                notes: d.notes?.trim() || undefined,
              }))
              .filter((d) => d.label),
          },
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Enregistrement impossible");
      setAppelDossier(json.appel as AppelPayload);
      if (close) {
        setMessage(
          json.sent
            ? "Appel clôturé : décision archivée et parents notifiés."
            : "Appel clôturé : décision archivée, mais aucun e-mail n’a pu être envoyé (SMTP indisponible).",
        );
        await reload();
      } else {
        setMessage("Dossier d’appel enregistré. Vous pourrez le compléter plus tard.");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setAppelDossierBusy(false);
    }
  }

  function addAppelDocument() {
    setAppelDossierForm((prev) => ({
      ...prev,
      documents: [...prev.documents, { label: "", statut: "attendu" }],
    }));
  }

  function updateAppelDocument(index: number, patch: Partial<AppelDocument>) {
    setAppelDossierForm((prev) => ({
      ...prev,
      documents: prev.documents.map((d, i) => (i === index ? { ...d, ...patch } : d)),
    }));
  }

  function removeAppelDocument(index: number) {
    setAppelDossierForm((prev) => ({
      ...prev,
      documents: prev.documents.filter((_, i) => i !== index),
    }));
  }

  async function runAction(action: string, extra?: Record<string, unknown>) {
    setBusy(action);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/campagnes/${id}/actions`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action, ...extra }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Action impossible");
      setMessage(
        action === "generate"
          ? `Fiches créées : ${json.created} (ignorées : ${json.skipped})${
              json.errors?.length ? ` — erreurs : ${json.errors.length}` : ""
            }`
          : action === "remind" || action === "send"
            ? `Envois : ${json.sent}, échecs : ${json.failed}${
                json.errors?.length ? ` (${json.errors.slice(0, 2).join(" ; ")})` : ""
              }`
            : action === "unfreeze_etape"
              ? "Étape dégelée : relancez les familles restantes, puis gelez à nouveau."
              : `Fiches avancées vers le conseil : ${json.advanced} — bloquées : ${
                  json.skipped?.length ?? 0
                }`,
      );
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function patchCampagneStatut(statut: "active" | "cloturee" | "archivee") {
    if (
      (statut === "cloturee" || statut === "archivee") &&
      !window.confirm(
        statut === "cloturee"
          ? "Clôturer la campagne ? Les familles ne pourront plus répondre et les liens en cours seront refusés."
          : "Archiver la campagne ? Elle restera consultable mais ne pourra plus être réactivée.",
      )
    ) {
      return;
    }
    setBusy(`campagne-${statut}`);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ statut }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Changement de statut impossible");
      setMessage(
        statut === "cloturee"
          ? "Campagne clôturée. Les fiches en cours restent consultables."
          : statut === "archivee"
            ? "Campagne archivée."
            : "Campagne réactivée.",
      );
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function saveEtapeWindow(
    etapeId: string,
    patch: { conseilDate?: string; opensAt?: string; closesAt?: string },
  ) {
    setBusy(`etape-${etapeId}`);
    try {
      const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          etapes: [
            {
              id: etapeId,
              conseilDate: patch.conseilDate === undefined ? undefined : patch.conseilDate || null,
              opensAt:
                patch.opensAt === undefined
                  ? undefined
                  : patch.opensAt
                    ? new Date(patch.opensAt).toISOString()
                    : null,
              closesAt:
                patch.closesAt === undefined
                  ? undefined
                  : patch.closesAt
                    ? new Date(patch.closesAt).toISOString()
                    : null,
            },
          ],
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Enregistrement impossible");
      setEtapes(json.etapes || []);
      setMessage("Dates d’étape enregistrées.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function submitConseil(publish: boolean) {
    if (!conseilFicheId || !conseilEtapeId) {
      setError("Étape du conseil introuvable pour cette fiche.");
      return;
    }
    if (conseilAvenant && !conseilForm.motif.trim() && !conseilForm.commentaire.trim()) {
      setError("Un avenant nécessite un motif explicite.");
      return;
    }
    setBusy("conseil");
    setError(null);
    try {
      const signatures = publish
        ? [
            {
              role: "professeur_principal" as const,
              name: conseilForm.ppName.trim() || "Professeur principal",
              method: "saved",
            },
            {
              role: "direction" as const,
              name: conseilForm.directionName.trim() || "Direction",
              method: "saved",
            },
          ]
        : [
            {
              role: "professeur_principal" as const,
              name: conseilForm.ppName.trim() || "Professeur principal",
              method: "saved",
            },
          ];
      const res = await fetch(`/api/fiches-dialogue/fiches/${conseilFicheId}/conseil`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          etapeId: conseilEtapeId,
          payload: {
            avis: conseilForm.avis,
            destinationProposee: conseilForm.destinationProposee || undefined,
            motif: conseilForm.motif || undefined,
            commentaire: conseilForm.commentaire || undefined,
          },
          signatures,
          publish,
          avenant: conseilAvenant,
          useSavedSignature: true,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Décision impossible");
      setConseilFicheId(null);
      setConseilEtapeId(null);
      setConseilAvenant(false);
      setMessage(
        !publish
          ? "Avis PP enregistré — en attente de validation direction."
          : conseilAvenant
            ? "Avenant publié : la famille est informée et la fiche repasse en acceptation."
            : "Décision publiée, PDF archivé et envoyé à la famille.",
      );
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function resolveConflit() {
    if (!conflitFicheId) return;
    setBusy("conflit");
    setError(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/fiches/${conflitFicheId}/conflit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          resolution: conflitResolution,
          note: conflitNote.trim() || undefined,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Résolution impossible");
      setConflitFicheId(null);
      setConflitNote("");
      setMessage("Conflit parental résolu : la fiche peut être avancée vers le conseil.");
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  if (loading || !campagne) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1200px]">
        <p className={`text-center text-sm ${dash.textMid}`}>
          {error || "Chargement…"}
        </p>
      </ModulePageShell>
    );
  }

  return (
    <ModulePageShell maxWidthClass="max-w-[1200px]">
      <ModulePageHeader
        title={campagne.label}
        description={`${campagne.anneeLabel}${campagne.niveauActuel ? ` · ${campagne.niveauActuel}` : ""} · mode ${campagne.calendrierMode} · statut ${campagne.statut}`}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Link href="/fiches-dialogue">
              <ModuleButton variant="secondary">← Campagnes</ModuleButton>
            </Link>
            {campagne.statut === "active" && (
              <ModuleButton
                disabled={!!busy}
                onClick={() => void patchCampagneStatut("cloturee")}
              >
                Clôturer la campagne
              </ModuleButton>
            )}
            {campagne.statut === "cloturee" && (
              <ModuleButton
                disabled={!!busy}
                variant="secondary"
                onClick={() => void patchCampagneStatut("archivee")}
              >
                Archiver
              </ModuleButton>
            )}
          </div>
        }
      />

      {message && (
        <p className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-900">
          {message}
        </p>
      )}
      {error && (
        <p className="rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800">
          {error}
        </p>
      )}

      {dossier && (
        <DossierPanel
          dossier={dossier}
          busy={dossierBusy}
          onClose={() => setDossier(null)}
        />
      )}

      <div className="grid gap-3 sm:grid-cols-4">
        <ModuleCard bodyClassName="p-4">
          <p className={`text-xs uppercase ${dash.textMid}`}>Fiches</p>
          <p className={`text-2xl font-semibold ${dash.ink}`}>{stats?.total ?? 0}</p>
        </ModuleCard>
        {Object.entries(stats?.byStatut ?? {})
          .slice(0, 3)
          .map(([k, v]) => (
            <ModuleCard key={k} bodyClassName="p-4">
              <p className={`text-xs uppercase ${dash.textMid}`}>{STATUT_LABELS[k] || k}</p>
              <p className={`text-2xl font-semibold ${dash.ink}`}>{v}</p>
            </ModuleCard>
          ))}
      </div>

      <ModuleCard bodyClassName="space-y-3 p-5">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Actions campagne</h2>
        <div className="flex flex-wrap gap-2">
          <ModuleButton
            disabled={!!busy}
            onClick={() => void runAction("generate")}
          >
            Générer les fiches élèves
          </ModuleButton>
          <ModuleButton
            disabled={!!busy}
            variant="secondary"
            onClick={() => void runAction("send", { onlyMissing: true })}
          >
            Envoyer aux familles (manquants)
          </ModuleButton>
          <ModuleButton
            disabled={!!busy}
            variant="secondary"
            onClick={() => void runAction("remind")}
          >
            Relancer les non-réponses
          </ModuleButton>
          {etapeFamillePourGel && (
            <ModuleButton
              disabled={!!busy}
              variant="secondary"
              onClick={() => {
                const total = fiches.filter(
                  (f) => f.etapeCouranteId === etapeFamillePourGel.id,
                ).length;
                const blocked = Math.max(0, total - fichesPretesPourGel);
                if (
                  !confirm(
                    `Geler « ${etapeFamillePourGel.label} » et passer au conseil ?\n\n` +
                      `${fichesPretesPourGel} fiche(s) prête(s) seront avancées.\n` +
                      `${blocked} fiche(s) sans réponse / en conflit / en attente resteront à cette étape (gelée) et seront signalées.`,
                  )
                ) {
                  return;
                }
                void runAction("freeze_and_conseil", {
                  fromEtapeId: etapeFamillePourGel.id,
                });
              }}
            >
              Geler & passer au conseil
            </ModuleButton>
          )}
        </div>
        <p className={`text-sm ${dash.textMid}`}>
          Qui commence :{" "}
          {campagne.starterMode === "conseil_dabord" ? "conseil" : "famille"}
          {campagne.contactPpLabel ? ` · Contact PP : ${campagne.contactPpLabel}` : ""} ·
          Classes cibles :{" "}
          {campagne.classesCibles?.length ? campagne.classesCibles.join(", ") : "toutes"}
          {" · Appel obligatoire"}
          {campagne.appelConfig?.dateLimite
            ? ` (limite ${campagne.appelConfig.dateLimite})`
            : " (détails à compléter)"}
        </p>
        {etapeFamillePourGel && fichesBloqueesPourGel.length > 0 && (
          <details className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-950">
            <summary className="cursor-pointer font-medium">
              {fichesBloqueesPourGel.length} fiche(s) à traiter avant le conseil
            </summary>
            <ul className="mt-2 space-y-1">
              {fichesBloqueesPourGel.slice(0, 12).map((f) => (
                <li key={f.id} className="flex flex-wrap items-center justify-between gap-2">
                  <span>
                    {f.elevePrenom} {f.eleveNom} ({f.classeActuelle || "—"}) —{" "}
                    {STATUT_LABELS[f.statut] ?? f.statut}
                  </span>
                  <button
                    type="button"
                    className="rounded-lg border border-amber-300 bg-white px-2 py-1 text-xs font-medium hover:bg-amber-100"
                    onClick={() => void openDossier(f)}
                  >
                    Ouvrir le dossier
                  </button>
                </li>
              ))}
            </ul>
            {fichesBloqueesPourGel.length > 12 && (
              <p className="mt-2 text-xs">
                … et {fichesBloqueesPourGel.length - 12} autre(s) fiche(s).
              </p>
            )}
          </details>
        )}
        <div className="pt-2">
          <ModuleButton
            variant="secondary"
            disabled={!!busy}
            onClick={async () => {
              if (
                !confirm(
                  `Supprimer définitivement « ${campagne.label} » et toutes les fiches associées ?`,
                )
              ) {
                return;
              }
              setBusy("delete");
              setError(null);
              try {
                const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, {
                  method: "DELETE",
                });
                const json = await res.json().catch(() => ({}));
                if (!res.ok) throw new Error(json.error || "Suppression impossible");
                window.location.href = "/fiches-dialogue";
              } catch (e) {
                setError(e instanceof Error ? e.message : "Erreur");
                setBusy(null);
              }
            }}
          >
            Supprimer la campagne
          </ModuleButton>
        </div>
      </ModuleCard>

      <ModuleCard bodyClassName="space-y-3 p-5">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>
          Procédure d’appel (obligatoire — complétable en fin d’année)
        </h2>
        <p className={`text-sm ${dash.textMid}`}>
          Date limite, documents et modalités sont souvent connus en mai–juin. Vous pouvez
          les renseigner ici à tout moment ; l’appel reste toujours actif.
        </p>
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="block text-sm sm:col-span-2">
            Date limite d’appel
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              placeholder="Ex. 15 juin 2026"
              value={appelForm.dateLimite}
              onChange={(e) => {
                setAppelDirty(true);
                setAppelForm({ ...appelForm, dateLimite: e.target.value });
              }}
            />
          </label>
          <label className="block text-sm sm:col-span-2">
            Contact PP affiché aux familles
            <input
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={appelForm.contactPpLabel}
              onChange={(e) => {
                setAppelDirty(true);
                setAppelForm({ ...appelForm, contactPpLabel: e.target.value });
              }}
            />
          </label>
          <label className="block text-sm sm:col-span-2">
            Documents à fournir (un par ligne)
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              rows={3}
              value={appelForm.documentsText}
              onChange={(e) => {
                setAppelDirty(true);
                setAppelForm({ ...appelForm, documentsText: e.target.value });
              }}
            />
          </label>
          <label className="block text-sm sm:col-span-2">
            Texte / procédure
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              rows={4}
              placeholder="Modalités d’appel communiquées par le rectorat / l’établissement…"
              value={appelForm.procedureHtml}
              onChange={(e) => {
                setAppelDirty(true);
                setAppelForm({ ...appelForm, procedureHtml: e.target.value });
              }}
            />
          </label>
        </div>
        <ModuleButton
          disabled={!!busy || !appelDirty}
          onClick={async () => {
            setBusy("appel");
            setError(null);
            try {
              const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, {
                method: "PATCH",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                  contactPpLabel: appelForm.contactPpLabel.trim() || null,
                  appelConfig: {
                    enabled: true,
                    dateLimite: appelForm.dateLimite.trim() || undefined,
                    procedureHtml: appelForm.procedureHtml.trim() || undefined,
                    contactPpLabel: appelForm.contactPpLabel.trim() || undefined,
                    documentsLabels: appelForm.documentsText
                      .split("\n")
                      .map((s) => s.trim())
                      .filter(Boolean),
                  },
                }),
              });
              const json = await res.json();
              if (!res.ok) throw new Error(json.error || "Enregistrement impossible");
              setCampagne(json.campagne);
              setAppelDirty(false);
              setMessage("Procédure d’appel enregistrée.");
            } catch (e) {
              setError(e instanceof Error ? e.message : "Erreur");
            } finally {
              setBusy(null);
            }
          }}
        >
          Enregistrer la procédure d’appel
        </ModuleButton>
      </ModuleCard>

      <ModuleCard bodyClassName="space-y-3 p-5">
        <h2 className={`text-lg font-semibold ${dash.ink}`}>Étapes</h2>
        <ol className="space-y-3">
          {etapes.map((e) => (
            <li
              key={e.id}
              className="flex flex-wrap items-start justify-between gap-3 rounded-lg border border-slate-100 px-3 py-2"
            >
              <div>
                <p className={`font-medium ${dash.ink}`}>
                  {e.ordre}. {e.label}
                  {e.optionnelle ? " (optionnelle)" : ""}
                  {e.gelee ? " — figée" : ""}
                </p>
                <p className={`text-sm ${dash.textMid}`}>
                  {e.kind}
                  {e.description ? ` · ${e.description}` : ""}
                </p>
              </div>
              <div className="flex flex-col gap-2 text-sm">
                {(e.kind === "conseil" || e.kind === "decision_finale_conseil") && (
                  <>
                    <label>
                      Date conseil
                      <input
                        type="date"
                        className="ml-2 rounded border border-slate-200 px-2 py-1"
                        value={e.conseilDate ?? ""}
                        onChange={(ev) =>
                          void saveEtapeWindow(e.id, { conseilDate: ev.target.value })
                        }
                      />
                    </label>
                    <label>
                      Publication résultats
                      <input
                        type="datetime-local"
                        className="ml-2 rounded border border-slate-200 px-2 py-1"
                        value={
                          e.opensAt
                            ? new Date(e.opensAt).toISOString().slice(0, 16)
                            : ""
                        }
                        onChange={(ev) =>
                          void saveEtapeWindow(e.id, { opensAt: ev.target.value })
                        }
                      />
                    </label>
                  </>
                )}
                {(e.kind === "saisie_famille" ||
                  e.kind === "choix_definitifs" ||
                  e.kind === "acceptation_famille") && (
                  <>
                    <label>
                      Ouverture
                      <input
                        type="datetime-local"
                        className="ml-2 rounded border border-slate-200 px-2 py-1"
                        value={
                          e.opensAt
                            ? new Date(e.opensAt).toISOString().slice(0, 16)
                            : ""
                        }
                        onChange={(ev) =>
                          void saveEtapeWindow(e.id, { opensAt: ev.target.value })
                        }
                      />
                    </label>
                    <label>
                      Dernier jour (clôture)
                      <input
                        type="datetime-local"
                        className="ml-2 rounded border border-slate-200 px-2 py-1"
                        value={
                          e.closesAt
                            ? new Date(e.closesAt).toISOString().slice(0, 16)
                            : ""
                        }
                        onChange={(ev) =>
                          void saveEtapeWindow(e.id, { closesAt: ev.target.value })
                        }
                      />
                    </label>
                  </>
                )}
                {e.gelee &&
                  (e.kind === "saisie_famille" ||
                    e.kind === "choix_definitifs" ||
                    e.kind === "acceptation_famille") &&
                  fiches.some(
                    (f) =>
                      f.etapeCouranteId === e.id &&
                      !["en_conseil", "acceptee", "refusee", "cloturee", "figee"].includes(
                        f.statut,
                      ),
                  ) && (
                    <button
                      type="button"
                      className="text-left text-amber-700 underline"
                      disabled={!!busy}
                      onClick={() => {
                        if (
                          !confirm(
                            "Dégeler cette étape pour permettre aux familles restantes de répondre ? Les fiches déjà avancées ne seront pas modifiées.",
                          )
                        ) {
                          return;
                        }
                        void runAction("unfreeze_etape", { fromEtapeId: e.id });
                      }}
                    >
                      Dégeler pour traiter les fiches restantes
                    </button>
                  )}
              </div>
            </li>
          ))}
        </ol>
      </ModuleCard>

      <ModuleCard bodyClassName="space-y-3 p-5">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>Fiches élèves</h2>
          <select
            className="rounded-lg border border-slate-200 px-3 py-2 text-sm"
            value={filtreClasse}
            onChange={(e) => setFiltreClasse(e.target.value)}
          >
            <option value="">Toutes les classes</option>
            {classes.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </div>
        <div className="overflow-x-auto">
          <table className="min-w-full text-left text-sm">
            <thead>
              <tr className={`border-b ${dash.textMid}`}>
                <th className="px-2 py-2">Élève</th>
                <th className="px-2 py-2">Classe</th>
                <th className="px-2 py-2">Statut</th>
                <th className="px-2 py-2">Contacts</th>
                <th className="px-2 py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {fichesFiltrees.map((f) => {
                const etapeFiche = etapeOfFiche(f);
                const peutConseil =
                  isConseilKind(etapeFiche?.kind) &&
                  !["acceptee", "refusee", "en_appel", "cloturee", "figee"].includes(f.statut);
                const peutAvenant =
                  Boolean(etapeFiche) &&
                  ["decision_envoyee", "en_attente_acceptation", "acceptee", "refusee", "en_appel"].includes(
                    f.statut,
                  ) &&
                  etapes.some(
                    (e) => isConseilKind(e.kind) && e.ordre <= (etapeFiche?.ordre ?? 0),
                  );
                return (
                  <tr key={f.id} className="border-b border-slate-100">
                    <td className="px-2 py-2 font-medium">
                      {f.elevePrenom} {f.eleveNom}
                    </td>
                    <td className="px-2 py-2">{f.classeActuelle || "—"}</td>
                    <td className="px-2 py-2">
                      {STATUT_LABELS[f.statut] || f.statut}
                      {f.parentAccordStatut ? ` (${f.parentAccordStatut})` : ""}
                    </td>
                    <td className="px-2 py-2">
                      {f.parentEmailCount > 0
                        ? `${f.parentEmailCount} e-mail(s)`
                        : "Aucun"}
                    </td>
                    <td className="space-x-3 px-2 py-2">
                      <button
                        type="button"
                        className="text-slate-700 underline"
                        onClick={() => void openDossier(f)}
                      >
                        Dossier famille
                      </button>
                      {peutConseil && (
                        <button
                          type="button"
                          className="text-emerald-700 underline"
                          onClick={() => void openConseil(f, false)}
                        >
                          {f.statut === "en_attente_direction"
                            ? "Valider (direction)"
                            : "Saisir avis conseil"}
                        </button>
                      )}
                      {peutAvenant && (
                        <button
                          type="button"
                          className="text-amber-700 underline"
                          onClick={() => {
                            if (
                              !confirm(
                                "Publier un avenant ? La famille sera informée et la fiche sera renvoyée à l’étape suivant la décision rectifiée.",
                              )
                            ) {
                              return;
                            }
                            void openConseil(f, true);
                          }}
                        >
                          Avenant
                        </button>
                      )}
                      {f.statut === "en_conflit" && (
                        <button
                          type="button"
                          className="text-rose-700 underline"
                          onClick={() => {
                            setConflitFicheId(f.id);
                            setConflitResolution("maintien_famille");
                            setConflitNote("");
                          }}
                        >
                          Résoudre le conflit
                        </button>
                      )}
                      {f.statut === "en_appel" && (
                        <button
                          type="button"
                          className="text-indigo-700 underline"
                          onClick={() => void openAppelDossier(f)}
                        >
                          Dossier d’appel
                        </button>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </ModuleCard>

      {conseilFicheId && (
        <ModuleCard bodyClassName="space-y-3 p-5">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>
            {conseilAvenant ? "Avenant — rectification du conseil" : "Avis du conseil"}
          </h2>
          {conseilAvenant && (
            <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
              Mode avenant : la décision précédemment publiée sera rectifiée et la famille
              informée. Un motif est obligatoire.
            </p>
          )}
          <div className="grid gap-3 sm:grid-cols-2">
            <label className="text-sm">
              Avis
              <select
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={conseilForm.avis}
                onChange={(e) =>
                  setConseilForm({
                    ...conseilForm,
                    avis: e.target.value as typeof conseilForm.avis,
                  })
                }
              >
                <option value="favorable">Favorable</option>
                <option value="reserve">Réservé</option>
                <option value="defavorable">Défavorable</option>
                <option value="autre">Autre</option>
              </select>
            </label>
            <label className="text-sm">
              Destination proposée
              <select
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={conseilForm.destinationProposee}
                onChange={(e) =>
                  setConseilForm({ ...conseilForm, destinationProposee: e.target.value })
                }
              >
                <option value="">—</option>
                {campagne.catalogue.destinations.map((d) => (
                  <option key={d.id} value={d.id}>
                    {d.label}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-sm sm:col-span-2">
              Motif
              <textarea
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                rows={2}
                value={conseilForm.motif}
                onChange={(e) => setConseilForm({ ...conseilForm, motif: e.target.value })}
              />
            </label>
            <label className="text-sm">
              Signature PP (nom)
              <input
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={conseilForm.ppName}
                onChange={(e) => setConseilForm({ ...conseilForm, ppName: e.target.value })}
              />
            </label>
            <label className="text-sm">
              Signature direction (nom)
              <input
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={conseilForm.directionName}
                onChange={(e) =>
                  setConseilForm({ ...conseilForm, directionName: e.target.value })
                }
              />
            </label>
          </div>
          <div className="flex flex-wrap gap-2">
            {!conseilAvenant && (
              <ModuleButton
                disabled={!!busy}
                variant="secondary"
                onClick={() => void submitConseil(false)}
              >
                Enregistrer (PP → direction)
              </ModuleButton>
            )}
            <ModuleButton
              disabled={
                !!busy ||
                (conseilAvenant &&
                  !conseilForm.motif.trim() &&
                  !conseilForm.commentaire.trim())
              }
              onClick={() => void submitConseil(true)}
            >
              {conseilAvenant
                ? "Publier l’avenant (direction + PDF + mail)"
                : "Publier (direction + PDF + mail)"}
            </ModuleButton>
            <ModuleButton
              variant="secondary"
              onClick={() => {
                setConseilFicheId(null);
                setConseilEtapeId(null);
                setConseilAvenant(false);
              }}
            >
              Annuler
            </ModuleButton>
          </div>
        </ModuleCard>
      )}

      {conflitFicheId && (
        <ModuleCard bodyClassName="space-y-3 p-5">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>
            Résolution du conflit parental
          </h2>
          <p className={`text-sm ${dash.textMid}`}>
            Le 2e parent a contredit la réponse déposée. Choisissez la position retenue ;
            la fiche redeviendra avançable vers le conseil.
          </p>
          <label className="block text-sm">
            Décision
            <select
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              value={conflitResolution}
              onChange={(e) =>
                setConflitResolution(e.target.value as "maintien_famille" | "avis_conseil")
              }
            >
              <option value="maintien_famille">Maintenir la réponse de la famille</option>
              <option value="avis_conseil">Retenir l’avis du conseil de classe</option>
            </select>
          </label>
          <label className="block text-sm">
            Note de résolution (facultative)
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              rows={2}
              value={conflitNote}
              onChange={(e) => setConflitNote(e.target.value)}
            />
          </label>
          <div className="flex flex-wrap gap-2">
            <ModuleButton disabled={!!busy} onClick={() => void resolveConflit()}>
              Enregistrer la résolution
            </ModuleButton>
            <ModuleButton
              variant="secondary"
              onClick={() => {
                setConflitFicheId(null);
                setConflitNote("");
              }}
            >
              Annuler
            </ModuleButton>
          </div>
        </ModuleCard>
      )}

      {appelFiche && (
        <ModuleCard bodyClassName="space-y-4 p-5">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div>
              <h2 className={`text-lg font-semibold ${dash.ink}`}>
                Appel — {appelFiche.elevePrenom} {appelFiche.eleveNom}
              </h2>
              <p className={`text-sm ${dash.textMid}`}>
                La commission d’appel est extérieure. Le dossier peut être complété au fur et à
                mesure : dates, informations reçues et documents encore attendus.
              </p>
            </div>
            <button
              type="button"
              className="text-sm text-slate-600 underline"
              onClick={() => setAppelFiche(null)}
            >
              Fermer
            </button>
          </div>

          {appelDossier?.closedAt && (
            <p className="rounded-lg border border-emerald-200 bg-emerald-50 p-3 text-sm text-emerald-900">
              Décision enregistrée le{" "}
              {new Date(appelDossier.closedAt).toLocaleString("fr-FR")}
              {appelDossier.notifiedAt
                ? " — parents notifiés."
                : " — notification parent non confirmée (SMTP)."}
              {" "}Le dossier est désormais consultable en lecture seule.
            </p>
          )}

          <fieldset
            disabled={Boolean(appelDossier?.closedAt)}
            className="space-y-4 disabled:opacity-80"
          >
          <div className="grid gap-3 md:grid-cols-2">
            <label className="block text-sm">
              Commission / instance
              <input
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={appelDossierForm.commissionLabel}
                onChange={(e) =>
                  setAppelDossierForm((p) => ({ ...p, commissionLabel: e.target.value }))
                }
                placeholder="Commission d’appel extérieure"
              />
            </label>
            <label className="block text-sm">
              Date de saisine
              <input
                type="date"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={appelDossierForm.saisineDate}
                onChange={(e) =>
                  setAppelDossierForm((p) => ({ ...p, saisineDate: e.target.value }))
                }
              />
            </label>
            <label className="block text-sm">
              Date de la commission
              <input
                type="date"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={appelDossierForm.commissionDate}
                onChange={(e) =>
                  setAppelDossierForm((p) => ({ ...p, commissionDate: e.target.value }))
                }
              />
            </label>
            <label className="block text-sm">
              Décision reçue le
              <input
                type="date"
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={appelDossierForm.receivedAt}
                onChange={(e) =>
                  setAppelDossierForm((p) => ({ ...p, receivedAt: e.target.value }))
                }
              />
            </label>
          </div>

          <label className="block text-sm">
            Informations et observations
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              rows={4}
              value={appelDossierForm.informations}
              onChange={(e) =>
                setAppelDossierForm((p) => ({ ...p, informations: e.target.value }))
              }
              placeholder="Éléments communiqués par la commission, pièces manquantes, points à vérifier…"
            />
          </label>

          <div className="grid gap-3 md:grid-cols-2">
            <label className="block text-sm">
              Décision de la commission
              <select
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={appelDossierForm.decision}
                onChange={(e) =>
                  setAppelDossierForm((p) => ({
                    ...p,
                    decision: e.target.value as "" | "maintien" | "reformation",
                  }))
                }
              >
                <option value="">À compléter</option>
                <option value="maintien">Maintien de la décision</option>
                <option value="reformation">Réformation</option>
              </select>
            </label>
            <label className="block text-sm">
              Destination / suite retenue
              <input
                className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
                value={appelDossierForm.destinationProposee}
                onChange={(e) =>
                  setAppelDossierForm((p) => ({
                    ...p,
                    destinationProposee: e.target.value,
                  }))
                }
                placeholder="Ex. 2nde générale, maintien en 1re…"
              />
            </label>
          </div>

          <label className="block text-sm">
            Motif de la décision
            <textarea
              className="mt-1 w-full rounded-lg border border-slate-200 px-3 py-2"
              rows={3}
              value={appelDossierForm.motif}
              onChange={(e) =>
                setAppelDossierForm((p) => ({ ...p, motif: e.target.value }))
              }
            />
          </label>

          <div className="space-y-2">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <h3 className={`text-sm font-semibold ${dash.ink}`}>Documents du dossier</h3>
                <p className={`text-xs ${dash.textMid}`}>
                  Ajoutez les documents attendus puis marquez-les reçus quand ils arrivent.
                </p>
              </div>
              <ModuleButton variant="secondary" onClick={addAppelDocument}>
                Ajouter un document
              </ModuleButton>
            </div>
            {appelDossierForm.documents.length === 0 ? (
              <p className="text-sm text-slate-500">Aucun document renseigné.</p>
            ) : (
              <div className="space-y-2">
                {appelDossierForm.documents.map((doc, index) => (
                  <div
                    key={`${index}-${doc.label}`}
                    className="grid gap-2 rounded-lg border border-slate-200 p-3 md:grid-cols-12"
                  >
                    <input
                      className="rounded-lg border border-slate-200 px-2 py-1.5 text-sm md:col-span-4"
                      value={doc.label}
                      onChange={(e) => updateAppelDocument(index, { label: e.target.value })}
                      placeholder="Libellé du document"
                    />
                    <input
                      className="rounded-lg border border-slate-200 px-2 py-1.5 text-sm md:col-span-3"
                      value={doc.reference || ""}
                      onChange={(e) =>
                        updateAppelDocument(index, { reference: e.target.value })
                      }
                      placeholder="Référence / lien"
                    />
                    <select
                      className="rounded-lg border border-slate-200 px-2 py-1.5 text-sm md:col-span-2"
                      value={doc.statut || "attendu"}
                      onChange={(e) =>
                        updateAppelDocument(index, {
                          statut: e.target.value as "attendu" | "recu",
                        })
                      }
                    >
                      <option value="attendu">Attendu</option>
                      <option value="recu">Reçu</option>
                    </select>
                    <input
                      type="date"
                      className="rounded-lg border border-slate-200 px-2 py-1.5 text-sm md:col-span-2"
                      value={doc.receivedAt || ""}
                      onChange={(e) =>
                        updateAppelDocument(index, { receivedAt: e.target.value })
                      }
                    />
                    <button
                      type="button"
                      className="text-sm text-rose-700 underline md:col-span-1"
                      onClick={() => removeAppelDocument(index)}
                    >
                      Retirer
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>

          <div className="flex flex-wrap gap-2">
            <ModuleButton
              disabled={appelDossierBusy}
              onClick={() => void saveAppelDossier(false)}
            >
              Enregistrer le dossier
            </ModuleButton>
            <ModuleButton
              variant="secondary"
              disabled={appelDossierBusy}
              onClick={() => void saveAppelDossier(true)}
            >
              Clôturer et notifier
            </ModuleButton>
          </div>
          <p className={`text-xs ${dash.textMid}`}>
            La clôture archive un PDF de la décision et envoie ce document aux parents. Si le SMTP
            est indisponible, la fiche est quand même clôturée et l’échec est signalé.
          </p>
          </fieldset>
        </ModuleCard>
      )}

      {archives.length > 0 && (
        <ModuleCard bodyClassName="space-y-2 p-5">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>Documents archivés</h2>
          <p className={`text-sm ${dash.textMid}`}>
            PDF des fiches et avenants. Le repli base de données garantit le téléchargement
            même si le stockage S3 était indisponible à l’archivage.
          </p>
          <ul className="space-y-1 text-sm">
            {archives.map((a) => {
              const fiche = fiches.find((f) => f.id === a.ficheId);
              return (
                <li key={a.id} className="flex flex-wrap items-center justify-between gap-2">
                  <span>
                    {fiche ? `${fiche.elevePrenom} ${fiche.eleveNom} — ` : ""}
                    {a.title}
                    <span className="ml-2 text-xs uppercase text-slate-500">
                      {a.storage === "db" ? "repli BDD" : a.storage === "s3" ? "S3" : "indisponible"}
                    </span>
                  </span>
                  {a.storage !== "none" && (
                    <a
                      className="text-emerald-700 underline"
                      href={`/api/fiches-dialogue/archives/${a.id}`}
                    >
                      Télécharger
                    </a>
                  )}
                </li>
              );
            })}
          </ul>
        </ModuleCard>
      )}
    </ModulePageShell>
  );
}
