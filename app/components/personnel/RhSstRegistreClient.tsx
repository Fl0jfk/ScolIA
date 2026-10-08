"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import PartenariatSignaturePad from "@/app/components/partenariats/PartenariatSignaturePad";
import {
  SST_FICHE_STATUS_LABELS,
  type SstConsultationDto,
  type SstFicheDetail,
  type SstFicheListItem,
  type SstMyStatus,
  type SstSuiviPayload,
} from "@/app/lib/sst-registre/types";

type ContentPayload = {
  reglementation: {
    title: string;
    texts: string[];
    objectifs: string[];
  };
  notice: {
    title: string;
    qui: string;
    comment: string;
    ou: string;
    quiConsulte: string;
    quand: string[];
    frequence: string;
  };
  emargementTexte: string;
  urgences: Array<{ label: string; value: string }>;
};

type MePayload = SstMyStatus & { content: ContentPayload };

type ViewMode = "accueil" | "fiche" | "suivi";

function todayIso(): string {
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "Europe/Paris",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date());
  const y = parts.find((p) => p.type === "year")?.value ?? "2026";
  const m = parts.find((p) => p.type === "month")?.value ?? "01";
  const d = parts.find((p) => p.type === "day")?.value ?? "01";
  return `${y}-${m}-${d}`;
}

function statusBadgeClass(status: string): string {
  if (status === "cloturee") return "bg-emerald-50 text-emerald-800 ring-emerald-200";
  if (status === "ouverte") return "bg-amber-50 text-amber-900 ring-amber-200";
  return "bg-sky-50 text-sky-900 ring-sky-200";
}

export default function RhSstRegistreClient({ mode = "staff" }: { mode?: "staff" | "pilotage" }) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [me, setMe] = useState<MePayload | null>(null);
  const [view, setView] = useState<ViewMode>(mode === "pilotage" ? "suivi" : "accueil");
  const [signatureDataUrl, setSignatureDataUrl] = useState<string | null>(null);
  const [remarques, setRemarques] = useState("");
  const [signing, setSigning] = useState(false);
  const [signOk, setSignOk] = useState(false);

  const [fiches, setFiches] = useState<SstFicheListItem[]>([]);
  const [suivi, setSuivi] = useState<SstSuiviPayload | null>(null);
  const [selectedFiche, setSelectedFiche] = useState<SstFicheDetail | null>(null);

  const [ficheForm, setFicheForm] = useState({
    observedDate: todayIso(),
    observedTime: "",
    lieu: "",
    observations: "",
    suggestions: "",
  });
  const [ficheSignature, setFicheSignature] = useState<string | null>(null);
  const [savingFiche, setSavingFiche] = useState(false);

  const [advanceNotes, setAdvanceNotes] = useState("");
  const [advanceSignature, setAdvanceSignature] = useState<string | null>(null);
  const [advancing, setAdvancing] = useState(false);

  const [consultComments, setConsultComments] = useState("");
  const [consultSignature, setConsultSignature] = useState<string | null>(null);
  const [savingConsult, setSavingConsult] = useState(false);

  const loadMe = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/sst", { cache: "no-store" });
      const json = (await res.json()) as MePayload & { error?: string };
      if (!res.ok) throw new Error(json.error || "Registre SST indisponible");
      setMe(json);
      setSignOk(json.signed);
    } catch (e) {
      setMe(null);
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadFiches = useCallback(async () => {
    const res = await fetch("/api/rh/sst/fiches", { cache: "no-store" });
    const json = (await res.json()) as { fiches?: SstFicheListItem[]; error?: string };
    if (res.ok) setFiches(json.fiches ?? []);
  }, []);

  const loadSuivi = useCallback(async () => {
    if (!me?.canManage && mode !== "pilotage") return;
    const res = await fetch("/api/rh/sst/suivi", { cache: "no-store" });
    const json = (await res.json()) as SstSuiviPayload & { error?: string };
    if (res.ok) {
      setSuivi(json);
      setFiches(json.fiches);
    }
  }, [me?.canManage, mode]);

  useEffect(() => {
    void loadMe();
  }, [loadMe]);

  useEffect(() => {
    if (!me) return;
    void loadFiches();
    if (me.canManage || mode === "pilotage") void loadSuivi();
  }, [me, loadFiches, loadSuivi, mode]);

  const pendingPeople = useMemo(
    () => (suivi?.people ?? []).filter((p) => !p.signed),
    [suivi],
  );

  async function submitEmargement() {
    if (!signatureDataUrl) {
      alert("Signez dans le cadre (souris ou doigt).");
      return;
    }
    setSigning(true);
    try {
      const res = await fetch("/api/rh/sst/emarger", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ signatureDataUrl, remarques }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec signature");
      setSignOk(true);
      setSignatureDataUrl(null);
      await loadMe();
      if (me?.canManage) await loadSuivi();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSigning(false);
    }
  }

  async function submitFiche() {
    if (!ficheForm.observations.trim()) {
      alert("Indiquez vos observations.");
      return;
    }
    setSavingFiche(true);
    try {
      const res = await fetch("/api/rh/sst/fiches", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          ...ficheForm,
          signatureDataUrl: ficheSignature || undefined,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec création");
      setFicheForm({
        observedDate: todayIso(),
        observedTime: "",
        lieu: "",
        observations: "",
        suggestions: "",
      });
      setFicheSignature(null);
      setView("accueil");
      await loadFiches();
      await loadMe();
      if (me?.canManage) await loadSuivi();
      alert(`Fiche n°${json.fiche.numero} enregistrée.`);
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSavingFiche(false);
    }
  }

  async function openFiche(id: string) {
    const res = await fetch(`/api/rh/sst/fiches/${id}`, { cache: "no-store" });
    const json = (await res.json()) as { fiche?: SstFicheDetail; error?: string };
    if (!res.ok || !json.fiche) {
      alert(json.error || "Fiche introuvable");
      return;
    }
    setSelectedFiche(json.fiche);
    setAdvanceNotes("");
    setAdvanceSignature(null);
  }

  async function advanceFiche() {
    if (!selectedFiche) return;
    setAdvancing(true);
    try {
      const res = await fetch(`/api/rh/sst/fiches/${selectedFiche.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "advance",
          notes: advanceNotes,
          signatureDataUrl: advanceSignature || undefined,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec");
      setSelectedFiche(json.fiche);
      setAdvanceNotes("");
      setAdvanceSignature(null);
      await loadFiches();
      await loadSuivi();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setAdvancing(false);
    }
  }

  async function submitConsultation() {
    setSavingConsult(true);
    try {
      const res = await fetch("/api/rh/sst/consultations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          comments: consultComments,
          signatureDataUrl: consultSignature || undefined,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Échec");
      setConsultComments("");
      setConsultSignature(null);
      await loadSuivi();
      alert("Consultation enregistrée.");
    } catch (e) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSavingConsult(false);
    }
  }

  if (loading) {
    return <p className="py-10 text-center text-sm text-slate-500">Chargement du registre SST…</p>;
  }
  if (error || !me) {
    return <p className="py-10 text-center text-sm text-rose-600">{error || "Indisponible"}</p>;
  }

  const content = me.content;

  return (
    <div className="space-y-5">
      <header className="rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 via-white to-teal-50/50 p-5 sm:p-6">
        <p className="text-[11px] font-black uppercase tracking-widest text-teal-700">
          Santé &amp; sécurité au travail
        </p>
        <h2 className="mt-1 text-2xl font-black text-slate-900">{me.campagne.title}</h2>
        <p className="mt-2 max-w-3xl text-sm text-slate-600">
          Registre numérique : émargement annuel, fiches de signalement, suivi direction / CSE. Plus
          besoin de chercher le classeur à l&apos;accueil.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => setView("accueil")}
            className={`rounded-xl px-3.5 py-2 text-xs font-bold ${
              view === "accueil"
                ? "bg-teal-700 text-white"
                : "border border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
            }`}
          >
            Registre
          </button>
          <button
            type="button"
            onClick={() => setView("fiche")}
            className={`rounded-xl px-3.5 py-2 text-xs font-bold ${
              view === "fiche"
                ? "bg-teal-700 text-white"
                : "border border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
            }`}
          >
            Déposer une fiche
          </button>
          {(me.canManage || mode === "pilotage") && (
            <button
              type="button"
              onClick={() => {
                setView("suivi");
                void loadSuivi();
              }}
              className={`rounded-xl px-3.5 py-2 text-xs font-bold ${
                view === "suivi"
                  ? "bg-teal-700 text-white"
                  : "border border-slate-200 bg-white text-slate-700 hover:bg-slate-50"
              }`}
            >
              Suivi &amp; émargements
              {suivi && suivi.pendingCount > 0 ? ` (${suivi.pendingCount})` : ""}
            </button>
          )}
        </div>
      </header>

      {!signOk && view === "accueil" ? (
        <section className="rounded-2xl border border-amber-300 bg-amber-50 p-5 shadow-sm">
          <p className="text-[11px] font-black uppercase tracking-widest text-amber-800">
            À faire — année {me.campagne.anneeLabel}
          </p>
          <h3 className="mt-1 font-black text-slate-900">Émargement de l&apos;information</h3>
          <p className="mt-2 text-sm text-slate-700">{content.emargementTexte}</p>
          <div className="mt-4">
            <p className="mb-1 text-xs font-bold text-slate-600">Signature (souris ou doigt)</p>
            <PartenariatSignaturePad onChange={setSignatureDataUrl} />
          </div>
          <label className="mt-3 block">
            <span className="text-xs font-bold text-slate-600">Remarques (optionnel)</span>
            <input
              value={remarques}
              onChange={(e) => setRemarques(e.target.value)}
              className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
              maxLength={500}
            />
          </label>
          <button
            type="button"
            disabled={signing || !signatureDataUrl}
            onClick={() => void submitEmargement()}
            className="mt-4 rounded-xl bg-amber-700 px-4 py-2.5 text-xs font-bold text-white hover:bg-amber-800 disabled:opacity-50"
          >
            {signing ? "Enregistrement…" : "Je signe pour cette année scolaire"}
          </button>
        </section>
      ) : null}

      {signOk && view === "accueil" ? (
        <section className="rounded-2xl border border-emerald-200 bg-emerald-50/70 p-4 text-sm text-emerald-900">
          Émargement enregistré pour {me.campagne.anneeLabel}
          {me.emargement?.signedAt
            ? ` — le ${new Date(me.emargement.signedAt).toLocaleString("fr-FR")}`
            : ""}
          .
        </section>
      ) : null}

      {view === "accueil" ? (
        <div className="grid gap-4 lg:grid-cols-2">
          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <h3 className="font-black text-slate-900">{content.notice.title}</h3>
            <dl className="mt-3 space-y-3 text-sm text-slate-700">
              <div>
                <dt className="font-bold text-slate-900">Qui peut renseigner ?</dt>
                <dd>{content.notice.qui}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Comment ?</dt>
                <dd>{content.notice.comment}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Où le trouver ?</dt>
                <dd>{content.notice.ou}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Qui consulte ?</dt>
                <dd>{content.notice.quiConsulte}</dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Quand compléter ?</dt>
                <dd>
                  <ul className="mt-1 list-disc space-y-1 pl-5">
                    {content.notice.quand.map((q) => (
                      <li key={q}>{q}</li>
                    ))}
                  </ul>
                </dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Fréquence</dt>
                <dd>{content.notice.frequence}</dd>
              </div>
            </dl>
            <button
              type="button"
              onClick={() => setView("fiche")}
              className="mt-4 rounded-xl bg-teal-700 px-4 py-2.5 text-xs font-bold text-white hover:bg-teal-800"
            >
              Déposer une fiche de signalement
            </button>
          </section>

          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <h3 className="font-black text-slate-900">{content.reglementation.title}</h3>
            <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-slate-700">
              {content.reglementation.texts.map((t) => (
                <li key={t}>{t}</li>
              ))}
            </ul>
            <p className="mt-4 text-xs font-bold uppercase tracking-wide text-slate-500">Objectifs</p>
            <ul className="mt-1 list-disc space-y-1 pl-5 text-sm text-slate-700">
              {content.reglementation.objectifs.map((t) => (
                <li key={t}>{t}</li>
              ))}
            </ul>
            <p className="mt-4 text-xs font-bold uppercase tracking-wide text-slate-500">Urgences</p>
            <ul className="mt-1 flex flex-wrap gap-2 text-sm">
              {content.urgences.map((u) => (
                <li
                  key={u.label}
                  className="rounded-lg border border-slate-200 bg-slate-50 px-2.5 py-1 font-bold text-slate-800"
                >
                  {u.label} {u.value}
                </li>
              ))}
            </ul>
          </section>

          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm lg:col-span-2">
            <div className="mb-3 flex items-center justify-between gap-2">
              <h3 className="font-black text-slate-900">Fiches du registre</h3>
              <span className="text-xs text-slate-500">
                {me.openFichesCount} ouverte{me.openFichesCount > 1 ? "s" : ""}
              </span>
            </div>
            {fiches.length === 0 ? (
              <p className="text-sm italic text-slate-400">Aucune fiche pour l&apos;instant.</p>
            ) : (
              <ul className="divide-y divide-slate-100">
                {fiches.map((f) => (
                  <li key={f.id} className="flex flex-wrap items-center justify-between gap-2 py-2.5">
                    <div className="min-w-0">
                      <p className="text-sm font-bold text-slate-900">
                        Fiche n°{f.numero} — {f.declarantName || "Anonyme"}
                      </p>
                      <p className="truncate text-xs text-slate-500">
                        {f.observedDate}
                        {f.lieu ? ` · ${f.lieu}` : ""} — {f.observationsPreview}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`rounded-md px-2 py-0.5 text-[11px] font-bold ring-1 ${statusBadgeClass(f.status)}`}
                      >
                        {SST_FICHE_STATUS_LABELS[f.status]}
                      </span>
                      <button
                        type="button"
                        onClick={() => void openFiche(f.id)}
                        className="text-xs font-bold text-teal-700 underline"
                      >
                        Ouvrir
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      ) : null}

      {view === "fiche" ? (
        <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
          <h3 className="font-black text-slate-900">Nouvelle fiche du registre</h3>
          <p className="mt-1 text-sm text-slate-600">
            Signalez un risque, un incident, un dysfonctionnement ou une suggestion d&apos;amélioration.
          </p>
          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            <label className="block text-sm">
              <span className="font-bold text-slate-700">Date</span>
              <input
                type="date"
                value={ficheForm.observedDate}
                onChange={(e) => setFicheForm((f) => ({ ...f, observedDate: e.target.value }))}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </label>
            <label className="block text-sm">
              <span className="font-bold text-slate-700">Heure</span>
              <input
                type="time"
                value={ficheForm.observedTime}
                onChange={(e) => setFicheForm((f) => ({ ...f, observedTime: e.target.value }))}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </label>
            <label className="block text-sm sm:col-span-2">
              <span className="font-bold text-slate-700">Lieu (service, poste…)</span>
              <input
                value={ficheForm.lieu}
                onChange={(e) => setFicheForm((f) => ({ ...f, lieu: e.target.value }))}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                maxLength={200}
              />
            </label>
            <label className="block text-sm sm:col-span-2">
              <span className="font-bold text-slate-700">Observations</span>
              <textarea
                value={ficheForm.observations}
                onChange={(e) => setFicheForm((f) => ({ ...f, observations: e.target.value }))}
                rows={5}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                maxLength={8000}
              />
            </label>
            <label className="block text-sm sm:col-span-2">
              <span className="font-bold text-slate-700">Suggestions (facultatif)</span>
              <textarea
                value={ficheForm.suggestions}
                onChange={(e) => setFicheForm((f) => ({ ...f, suggestions: e.target.value }))}
                rows={3}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
                maxLength={4000}
              />
            </label>
          </div>
          <div className="mt-4">
            <p className="mb-1 text-xs font-bold text-slate-600">Signature du déclarant (recommandée)</p>
            <PartenariatSignaturePad onChange={setFicheSignature} />
          </div>
          <div className="mt-4 flex flex-wrap gap-2">
            <button
              type="button"
              disabled={savingFiche}
              onClick={() => void submitFiche()}
              className="rounded-xl bg-teal-700 px-4 py-2.5 text-xs font-bold text-white hover:bg-teal-800 disabled:opacity-50"
            >
              {savingFiche ? "Enregistrement…" : "Enregistrer la fiche"}
            </button>
            <button
              type="button"
              onClick={() => setView("accueil")}
              className="rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-xs font-bold text-slate-700"
            >
              Annuler
            </button>
          </div>
        </section>
      ) : null}

      {view === "suivi" && (me.canManage || mode === "pilotage") ? (
        <div className="space-y-4">
          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <h3 className="font-black text-slate-900">Émargements — {me.campagne.anneeLabel}</h3>
            {suivi ? (
              <>
                <p className="mt-2 text-sm text-slate-600">
                  <strong className="text-slate-900">{suivi.signedCount}</strong> / {suivi.total}{" "}
                  collaborateurs ont signé —{" "}
                  <strong className="text-amber-800">{suivi.pendingCount}</strong> en attente.
                </p>
                <div className="mt-3 max-h-80 overflow-auto rounded-xl border border-slate-100">
                  <table className="w-full text-left text-sm">
                    <thead className="sticky top-0 bg-slate-50 text-xs uppercase text-slate-500">
                      <tr>
                        <th className="px-3 py-2">Nom</th>
                        <th className="px-3 py-2">Fonction</th>
                        <th className="px-3 py-2">Statut</th>
                      </tr>
                    </thead>
                    <tbody>
                      {(suivi.people ?? []).map((p) => (
                        <tr key={p.userId} className="border-t border-slate-50">
                          <td className="px-3 py-2 font-medium text-slate-900">
                            {p.lastName} {p.firstName}
                            <span className="block text-xs font-normal text-slate-400">{p.email}</span>
                          </td>
                          <td className="px-3 py-2 text-slate-600">{p.fonction}</td>
                          <td className="px-3 py-2">
                            {p.signed ? (
                              <span className="font-bold text-emerald-700">
                                Signé
                                {p.signedAt
                                  ? ` · ${new Date(p.signedAt).toLocaleDateString("fr-FR")}`
                                  : ""}
                              </span>
                            ) : (
                              <span className="font-bold text-amber-700">En attente</span>
                            )}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                {pendingPeople.length > 0 ? (
                  <p className="mt-3 text-xs text-slate-500">
                    Les retardataires voient un rappel sur le tableau de bord tant qu&apos;ils n&apos;ont
                    pas signé.
                  </p>
                ) : null}
              </>
            ) : (
              <p className="mt-2 text-sm text-slate-400">Chargement du suivi…</p>
            )}
          </section>

          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <h3 className="font-black text-slate-900">Visa de consultation</h3>
            <p className="mt-1 text-sm text-slate-600">
              Direction, référent sécurité, CSE / OGEC — attestez avoir consulté le registre.
            </p>
            <label className="mt-3 block text-sm">
              <span className="font-bold text-slate-700">Commentaires</span>
              <textarea
                value={consultComments}
                onChange={(e) => setConsultComments(e.target.value)}
                rows={2}
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2"
              />
            </label>
            <div className="mt-3">
              <PartenariatSignaturePad onChange={setConsultSignature} />
            </div>
            <button
              type="button"
              disabled={savingConsult}
              onClick={() => void submitConsultation()}
              className="mt-3 rounded-xl bg-slate-800 px-4 py-2.5 text-xs font-bold text-white hover:bg-slate-900 disabled:opacity-50"
            >
              {savingConsult ? "…" : "Enregistrer mon visa de consultation"}
            </button>
            {suivi && suivi.consultations.length > 0 ? (
              <ul className="mt-4 space-y-2">
                {suivi.consultations.map((c: SstConsultationDto) => (
                  <li key={c.id} className="rounded-xl border border-slate-100 px-3 py-2 text-sm">
                    <span className="font-bold text-slate-900">
                      {c.lastName} {c.firstName}
                    </span>{" "}
                    <span className="text-slate-500">({c.fonction})</span>
                    <span className="block text-xs text-slate-400">
                      {new Date(c.consultedAt).toLocaleString("fr-FR")}
                      {c.comments ? ` — ${c.comments}` : ""}
                    </span>
                  </li>
                ))}
              </ul>
            ) : null}
          </section>

          <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
            <h3 className="font-black text-slate-900">Tableau de suivi des fiches</h3>
            {fiches.length === 0 ? (
              <p className="mt-2 text-sm italic text-slate-400">Aucune fiche.</p>
            ) : (
              <ul className="mt-3 divide-y divide-slate-100">
                {fiches.map((f) => (
                  <li key={f.id} className="flex flex-wrap items-center justify-between gap-2 py-2.5">
                    <div>
                      <p className="text-sm font-bold text-slate-900">
                        n°{f.numero} — {f.observationsPreview}
                      </p>
                      <p className="text-xs text-slate-500">
                        {f.observedDate} · {f.declarantName}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`rounded-md px-2 py-0.5 text-[11px] font-bold ring-1 ${statusBadgeClass(f.status)}`}
                      >
                        {SST_FICHE_STATUS_LABELS[f.status]}
                      </span>
                      <button
                        type="button"
                        onClick={() => void openFiche(f.id)}
                        className="text-xs font-bold text-teal-700 underline"
                      >
                        Traiter
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      ) : null}

      {selectedFiche ? (
        <div className="fixed inset-0 z-50 flex items-end justify-center bg-slate-900/40 p-4 sm:items-center">
          <div className="max-h-[90vh] w-full max-w-lg overflow-auto rounded-2xl bg-white p-5 shadow-xl">
            <div className="flex items-start justify-between gap-2">
              <div>
                <p className="text-[11px] font-black uppercase tracking-widest text-teal-700">
                  Fiche n°{selectedFiche.numero}
                </p>
                <h3 className="font-black text-slate-900">
                  {SST_FICHE_STATUS_LABELS[selectedFiche.status]}
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setSelectedFiche(null)}
                className="text-xs font-bold text-slate-500 underline"
              >
                Fermer
              </button>
            </div>
            <dl className="mt-3 space-y-2 text-sm text-slate-700">
              <div>
                <dt className="font-bold text-slate-900">Déclarant</dt>
                <dd>
                  {selectedFiche.declarantFirstName} {selectedFiche.declarantLastName} —{" "}
                  {selectedFiche.declarantFonction}
                </dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Quand / où</dt>
                <dd>
                  {selectedFiche.observedDate}
                  {selectedFiche.observedTime ? ` à ${selectedFiche.observedTime}` : ""}
                  {selectedFiche.lieu ? ` — ${selectedFiche.lieu}` : ""}
                </dd>
              </div>
              <div>
                <dt className="font-bold text-slate-900">Observations</dt>
                <dd className="whitespace-pre-wrap">{selectedFiche.observations}</dd>
              </div>
              {selectedFiche.suggestions ? (
                <div>
                  <dt className="font-bold text-slate-900">Suggestions</dt>
                  <dd className="whitespace-pre-wrap">{selectedFiche.suggestions}</dd>
                </div>
              ) : null}
            </dl>

            {(["responsable", "cse", "decision", "realisation"] as const).map((key) => {
              const visa = selectedFiche.workflow[key];
              if (!visa) return null;
              const labels = {
                responsable: "Visa responsable sécurité",
                cse: "Examen CSE / CHSCT",
                decision: "Décision",
                realisation: "Réalisation / suivi",
              };
              return (
                <div key={key} className="mt-3 rounded-xl border border-slate-100 bg-slate-50 p-3 text-sm">
                  <p className="font-bold text-slate-900">{labels[key]}</p>
                  <p className="text-xs text-slate-500">
                    {visa.name} — {new Date(visa.at).toLocaleString("fr-FR")}
                  </p>
                  {visa.notes ? <p className="mt-1 whitespace-pre-wrap text-slate-700">{visa.notes}</p> : null}
                </div>
              );
            })}

            {selectedFiche.canAdvance && selectedFiche.status !== "cloturee" ? (
              <div className="mt-4 border-t border-slate-100 pt-4">
                <p className="text-xs font-bold text-slate-600">
                  Passer à l&apos;étape suivante — notes &amp; visa
                </p>
                <textarea
                  value={advanceNotes}
                  onChange={(e) => setAdvanceNotes(e.target.value)}
                  rows={3}
                  className="mt-2 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                  placeholder="Observations / mesures…"
                />
                <div className="mt-2">
                  <PartenariatSignaturePad onChange={setAdvanceSignature} />
                </div>
                <button
                  type="button"
                  disabled={advancing}
                  onClick={() => void advanceFiche()}
                  className="mt-3 rounded-xl bg-teal-700 px-4 py-2.5 text-xs font-bold text-white hover:bg-teal-800 disabled:opacity-50"
                >
                  {advancing ? "…" : "Valider l’étape suivante"}
                </button>
              </div>
            ) : null}
          </div>
        </div>
      ) : null}
    </div>
  );
}
