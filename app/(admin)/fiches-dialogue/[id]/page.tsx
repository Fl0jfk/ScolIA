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
  parentEmails: string[];
  lastSentAt: string | null;
};

type Campagne = {
  id: string;
  label: string;
  anneeLabel: string;
  calendrierMode: string;
  statut: string;
  delaiFamilleJours: number;
  classesCibles: string[];
  catalogue: {
    destinations: Array<{ id: string; label: string }>;
    options: Array<{ id: string; label: string }>;
  };
  appelConfig: { enabled: boolean; dateLimite?: string; procedureHtml?: string };
};

const STATUT_LABELS: Record<string, string> = {
  a_envoyer: "À envoyer",
  en_attente_famille: "En attente famille",
  saisie_recue: "Saisie reçue",
  en_conseil: "En conseil",
  decision_envoyee: "Décision envoyée",
  en_attente_acceptation: "Acceptation en attente",
  acceptee: "Acceptée",
  refusee: "Refusée",
  en_appel: "En appel",
  cloturee: "Clôturée",
  figee: "Figée",
};

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
  const [conseilForm, setConseilForm] = useState({
    avis: "favorable" as "favorable" | "reserve" | "defavorable" | "autre",
    destinationProposee: "",
    motif: "",
    commentaire: "",
    ppName: "",
    directionName: "",
  });

  const reload = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, { cache: "no-store" });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Erreur");
      setCampagne(json.campagne);
      setEtapes(json.etapes || []);
      setFiches(json.fiches || []);
      setStats(json.stats || null);
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
          ? `Fiches créées : ${json.created} (ignorées : ${json.skipped})`
          : action === "remind" || action === "send"
            ? `Envois : ${json.sent}, échecs : ${json.failed}`
            : `Fiches avancées vers le conseil : ${json.advanced}`,
      );
      await reload();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function saveEtapeDates(etapeId: string, conseilDate: string) {
    setBusy(`etape-${etapeId}`);
    try {
      const res = await fetch(`/api/fiches-dialogue/campagnes/${id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          etapes: [{ id: etapeId, conseilDate: conseilDate || null }],
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Enregistrement impossible");
      setEtapes(json.etapes || []);
      setMessage("Date de conseil enregistrée.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(null);
    }
  }

  async function submitConseil() {
    if (!conseilFicheId || !etapeCouranteDominante) return;
    setBusy("conseil");
    setError(null);
    try {
      const res = await fetch(`/api/fiches-dialogue/fiches/${conseilFicheId}/conseil`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          etapeId: etapeCouranteDominante.id,
          payload: {
            avis: conseilForm.avis,
            destinationProposee: conseilForm.destinationProposee || undefined,
            motif: conseilForm.motif || undefined,
            commentaire: conseilForm.commentaire || undefined,
          },
          signatures: [
            {
              role: "professeur_principal",
              name: conseilForm.ppName.trim() || "Professeur principal",
              method: "pad",
            },
            {
              role: "direction",
              name: conseilForm.directionName.trim() || "Direction",
              method: "pad",
            },
          ],
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Décision impossible");
      setConseilFicheId(null);
      setMessage("Décision enregistrée, PDF archivé et envoyé à la famille.");
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
        description={`${campagne.anneeLabel} · mode ${campagne.calendrierMode} · statut ${campagne.statut}`}
        actions={
          <Link href="/fiches-dialogue">
            <ModuleButton variant="secondary">← Campagnes</ModuleButton>
          </Link>
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
          {etapeCouranteDominante &&
            (etapeCouranteDominante.kind === "saisie_famille" ||
              etapeCouranteDominante.kind === "choix_definitifs") && (
              <ModuleButton
                disabled={!!busy}
                variant="secondary"
                onClick={() =>
                  void runAction("freeze_and_conseil", {
                    fromEtapeId: etapeCouranteDominante.id,
                  })
                }
              >
                Geler & passer au conseil
              </ModuleButton>
            )}
        </div>
        <p className={`text-sm ${dash.textMid}`}>
          Délai famille : {campagne.delaiFamilleJours} j · Classes cibles :{" "}
          {campagne.classesCibles?.length ? campagne.classesCibles.join(", ") : "toutes"}
          {campagne.appelConfig?.enabled
            ? ` · Appel activé${campagne.appelConfig.dateLimite ? ` (limite ${campagne.appelConfig.dateLimite})` : ""}`
            : " · Appel désactivé"}
        </p>
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
              {(e.kind === "conseil" || e.kind === "decision_finale_conseil") && (
                <label className="text-sm">
                  Date conseil
                  <input
                    type="date"
                    className="ml-2 rounded border border-slate-200 px-2 py-1"
                    value={e.conseilDate ?? ""}
                    onChange={(ev) => void saveEtapeDates(e.id, ev.target.value)}
                  />
                </label>
              )}
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
                <th className="px-2 py-2">Parents</th>
                <th className="px-2 py-2">Actions</th>
              </tr>
            </thead>
            <tbody>
              {fichesFiltrees.map((f) => (
                <tr key={f.id} className="border-b border-slate-100">
                  <td className="px-2 py-2 font-medium">
                    {f.elevePrenom} {f.eleveNom}
                  </td>
                  <td className="px-2 py-2">{f.classeActuelle || "—"}</td>
                  <td className="px-2 py-2">{STATUT_LABELS[f.statut] || f.statut}</td>
                  <td className="px-2 py-2">{f.parentEmails?.join(", ") || "—"}</td>
                  <td className="px-2 py-2">
                    {(f.statut === "saisie_recue" ||
                      f.statut === "en_conseil" ||
                      f.statut === "en_attente_famille") &&
                      etapeCouranteDominante &&
                      (etapeCouranteDominante.kind === "conseil" ||
                        etapeCouranteDominante.kind === "decision_finale_conseil" ||
                        f.statut === "saisie_recue") && (
                        <button
                          type="button"
                          className="text-emerald-700 underline"
                          onClick={() => {
                            setConseilFicheId(f.id);
                            setConseilForm((prev) => ({
                              ...prev,
                              destinationProposee:
                                campagne.catalogue.destinations[0]?.id || "",
                            }));
                          }}
                        >
                          Saisir avis conseil
                        </button>
                      )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </ModuleCard>

      {conseilFicheId && (
        <ModuleCard bodyClassName="space-y-3 p-5">
          <h2 className={`text-lg font-semibold ${dash.ink}`}>Avis du conseil</h2>
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
          <div className="flex gap-2">
            <ModuleButton disabled={!!busy} onClick={() => void submitConseil()}>
              Enregistrer, PDF & mail
            </ModuleButton>
            <ModuleButton
              variant="secondary"
              onClick={() => setConseilFicheId(null)}
            >
              Annuler
            </ModuleButton>
          </div>
        </ModuleCard>
      )}
    </ModulePageShell>
  );
}
