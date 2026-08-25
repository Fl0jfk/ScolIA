"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";

type CalendrierRow = {
  id: string;
  label: string;
  dateDebut: string;
  dateFin: string;
  type: string;
};

type CreneauRow = {
  id: string;
  jourSemaine: number;
  heureDebut: string;
  heureFin: string;
  classe: string | null;
  groupeId?: string | null;
  groupeCode?: string | null;
  enseignantNom: string | null;
  salle: string | null;
  semaine: string;
};
type Groupe = { id: string; code: string; libelle: string; memberCount: number };
type Conflit = { kind: string; label: string; detail: string; creneauIds: string[] };

const JOURS = ["", "Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];

export default function VsCalendrierClient() {
  const [calendrier, setCalendrier] = useState<CalendrierRow[]>([]);
  const [creneaux, setCreneaux] = useState<CreneauRow[]>([]);
  const [groupes, setGroupes] = useState<Groupe[]>([]);
  const [conflits, setConflits] = useState<Conflit[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [anneeLabel, setAnneeLabel] = useState<string | null>(null);
  const [selfOnly, setSelfOnly] = useState(false);
  const [canManage, setCanManage] = useState(false);
  const [calForm, setCalForm] = useState({ label: "", dateDebut: "", dateFin: "", type: "vacances" });
  const [creneauForm, setCreneauForm] = useState({
    jourSemaine: "1",
    heureDebut: "08:00",
    heureFin: "09:00",
    classe: "",
    groupeId: "",
    enseignantNom: "",
    salle: "",
    semaine: "AB",
  });
  const [filtreClasse, setFiltreClasse] = useState("");
  const [filtreGroupeId, setFiltreGroupeId] = useState("");

  const load = useCallback(async () => {
    setError(null);
    try {
      const qs = new URLSearchParams();
      if (!selfOnly) {
        if (filtreClasse.trim()) qs.set("classe", filtreClasse.trim());
        if (filtreGroupeId) qs.set("groupeId", filtreGroupeId);
      }
      const suffix = qs.toString() ? `?${qs.toString()}` : "";
      const res = await fetch(`/api/vie-scolaire/calendrier${suffix}`, { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      setCalendrier(data.calendrier || []);
      setCreneaux(data.creneaux || []);
      setGroupes(data.groupes || []);
      setConflits(Array.isArray(data.conflits) ? data.conflits : []);
      setSelfOnly(Boolean(data.selfOnly));
      setCanManage(Boolean(data.canManage));
      if (data.anneeCourante?.label) setAnneeLabel(String(data.anneeCourante.label));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, [filtreClasse, filtreGroupeId, selfOnly]);

  useEffect(() => {
    void load();
  }, [load]);

  const post = async (body: Record<string, unknown>) => {
    setBusy(true);
    setMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/vie-scolaire/calendrier", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const data = await res.json();
      if (!res.ok) {
        const errMsg = data?.error || "Échec";
        if (
          body.action === "upsertCreneau" &&
          typeof errMsg === "string" &&
          errMsg.includes("Conflit EDT") &&
          !body.force
        ) {
          const ok = window.confirm(`${errMsg}\n\nForcer l’enregistrement quand même ?`);
          if (ok) {
            await post({ ...body, force: true, entry: { ...(body.entry as object), force: true } });
            return;
          }
        }
        throw new Error(errMsg);
      }
      setMessage(data.seeded === false ? "Calendrier déjà initialisé." : "Enregistré.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-5xl">
      <ModulePageHeader
        eyebrow="Vie scolaire"
        title={selfOnly ? "Mon emploi du temps" : "Calendrier & créneaux EDT"}
        description={
          selfOnly
            ? anneeLabel
              ? `Année ${anneeLabel} — vos créneaux uniquement.`
              : "Consultation de vos créneaux EDT (vue personnelle)."
            : anneeLabel
              ? `Année ${anneeLabel} — vacances / hors classe et créneaux EDT classe ou groupe.`
              : "Vacances et périodes hors classe ; créneaux EDT classe."
        }
        actions={
          <div className="flex flex-wrap gap-3 text-sm font-bold">
            {!selfOnly ? (
              <Link href="/mon-planning" className="text-indigo-600 hover:underline">
                Mon planning (RH)
              </Link>
            ) : null}
            <Link href="/vie-scolaire" className="text-indigo-600 hover:underline">
              ← Vie scolaire
            </Link>
          </div>
        }
      />

      {!selfOnly && canManage ? (
        <div className="flex flex-wrap gap-3 mb-6">
          <button
            type="button"
            disabled={busy}
            onClick={() => void post({ action: "seedDefaults" })}
            className="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
          >
            Initialiser vacances (exemple)
          </button>
          <Link
            href="/vie-scolaire/presence?tab=appel"
            className="rounded-xl border border-indigo-200 bg-indigo-50 px-4 py-2 text-sm font-bold text-indigo-800 hover:bg-indigo-100"
          >
            Appels du jour →
          </Link>
          <Link
            href="/edt-classe"
            className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700 hover:bg-slate-50"
          >
            EDT agrégé profs →
          </Link>
          <Link
            href="/mon-planning"
            className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700 hover:bg-slate-50"
          >
            Planning personnel →
          </Link>
        </div>
      ) : (
        <div className="mb-6 flex flex-wrap gap-3">
          <Link
            href="/vie-scolaire/presence?tab=appel"
            className="rounded-xl border border-indigo-200 bg-indigo-50 px-4 py-2 text-sm font-bold text-indigo-800 hover:bg-indigo-100"
          >
            Appels du jour →
          </Link>
        </div>
      )}

      {message && <p className="mb-3 text-sm text-emerald-700 font-semibold">{message}</p>}
      {error && <p className="mb-3 text-sm text-red-600">{error}</p>}

      {conflits.length > 0 ? (
        <section className="mb-6 rounded-2xl border border-amber-200 bg-amber-50 p-4 space-y-2">
          <h2 className="text-sm font-black text-amber-950">
            Conflits EDT ({conflits.length}) — contraintes dures
          </h2>
          <ul className="max-h-40 space-y-1 overflow-y-auto text-sm text-amber-950">
            {conflits.slice(0, 20).map((c, i) => (
              <li key={`${c.kind}-${c.creneauIds.join("-")}-${i}`} className="rounded-lg bg-white/70 px-3 py-1.5">
                <span className="font-bold">{c.label}</span>
                <span className="text-xs text-amber-800 block">{c.detail}</span>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <div className={`grid gap-6 ${selfOnly ? "" : "md:grid-cols-2"}`}>
        {!selfOnly ? (
          <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
            <h2 className="font-black text-slate-900">Calendrier scolaire ({calendrier.length})</h2>
            {canManage ? (
              <div className="grid gap-2">
                <input
                  className="border rounded-xl px-3 py-2 text-sm"
                  placeholder="Libellé (ex. Toussaint)"
                  value={calForm.label}
                  onChange={(e) => setCalForm({ ...calForm, label: e.target.value })}
                />
                <div className="flex gap-2">
                  <input
                    type="date"
                    className="border rounded-xl px-3 py-2 text-sm flex-1"
                    value={calForm.dateDebut}
                    onChange={(e) => setCalForm({ ...calForm, dateDebut: e.target.value })}
                  />
                  <input
                    type="date"
                    className="border rounded-xl px-3 py-2 text-sm flex-1"
                    value={calForm.dateFin}
                    onChange={(e) => setCalForm({ ...calForm, dateFin: e.target.value })}
                  />
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void post({ action: "upsertCalendrier", entry: calForm })}
                  className="rounded-xl bg-slate-900 text-white px-3 py-2 text-sm font-bold disabled:opacity-50"
                >
                  Ajouter période
                </button>
              </div>
            ) : null}
            <ul className="space-y-2 max-h-64 overflow-y-auto text-sm">
              {calendrier.map((c) => (
                <li key={c.id} className="flex justify-between gap-2 border-b border-slate-100 pb-2">
                  <span>
                    <span className="font-semibold">{c.label}</span>
                    <span className="text-slate-500 block text-xs">
                      {c.dateDebut} → {c.dateFin} · {c.type}
                    </span>
                  </span>
                  {canManage ? (
                    <button
                      type="button"
                      className="text-xs text-red-600 font-bold"
                      onClick={() => void post({ action: "deleteCalendrier", id: c.id })}
                    >
                      Suppr.
                    </button>
                  ) : null}
                </li>
              ))}
              {!calendrier.length && <li className="text-slate-500">Aucune période</li>}
            </ul>
          </section>
        ) : null}

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="font-black text-slate-900">
            {selfOnly ? `Mes créneaux (${creneaux.length})` : `Créneaux EDT (${creneaux.length})`}
          </h2>
          {!selfOnly ? (
            <div className="flex flex-wrap gap-2">
              <input
                className="border rounded-xl px-3 py-2 text-sm w-28"
                placeholder="Filtrer classe"
                value={filtreClasse}
                onChange={(e) => {
                  setFiltreClasse(e.target.value);
                  if (e.target.value.trim()) setFiltreGroupeId("");
                }}
              />
              <select
                className="border rounded-xl px-3 py-2 text-sm min-w-[10rem]"
                value={filtreGroupeId}
                onChange={(e) => {
                  setFiltreGroupeId(e.target.value);
                  if (e.target.value) setFiltreClasse("");
                }}
              >
                <option value="">Filtrer groupe</option>
                {groupes.map((g) => (
                  <option key={g.id} value={g.id}>
                    {g.code} ({g.memberCount})
                  </option>
                ))}
              </select>
            </div>
          ) : null}
          {canManage && !selfOnly ? (
            <div className="grid gap-2 text-sm">
              <div className="flex gap-2">
                <select
                  className="border rounded-xl px-2 py-2"
                  value={creneauForm.jourSemaine}
                  onChange={(e) => setCreneauForm({ ...creneauForm, jourSemaine: e.target.value })}
                >
                  {[1, 2, 3, 4, 5].map((d) => (
                    <option key={d} value={String(d)}>
                      {JOURS[d]}
                    </option>
                  ))}
                </select>
                <input
                  className="border rounded-xl px-2 py-2 w-20"
                  value={creneauForm.heureDebut}
                  onChange={(e) => setCreneauForm({ ...creneauForm, heureDebut: e.target.value })}
                />
                <input
                  className="border rounded-xl px-2 py-2 w-20"
                  value={creneauForm.heureFin}
                  onChange={(e) => setCreneauForm({ ...creneauForm, heureFin: e.target.value })}
                />
              </div>
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="Classe (optionnel si groupe)"
                value={creneauForm.classe}
                onChange={(e) =>
                  setCreneauForm({ ...creneauForm, classe: e.target.value, groupeId: "" })
                }
              />
              <select
                className="border rounded-xl px-3 py-2"
                value={creneauForm.groupeId}
                onChange={(e) =>
                  setCreneauForm({
                    ...creneauForm,
                    groupeId: e.target.value,
                    classe: e.target.value ? "" : creneauForm.classe,
                  })
                }
              >
                <option value="">Groupe pédagogique (LV2, option…)</option>
                {groupes.map((g) => (
                  <option key={g.id} value={g.id}>
                    {g.code} — {g.libelle}
                  </option>
                ))}
              </select>
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="Enseignant"
                value={creneauForm.enseignantNom}
                onChange={(e) => setCreneauForm({ ...creneauForm, enseignantNom: e.target.value })}
              />
              <input
                className="border rounded-xl px-3 py-2"
                placeholder="Salle"
                value={creneauForm.salle}
                onChange={(e) => setCreneauForm({ ...creneauForm, salle: e.target.value })}
              />
              <button
                type="button"
                disabled={busy}
                onClick={() =>
                  void post({
                    action: "upsertCreneau",
                    creneau: {
                      ...creneauForm,
                      jourSemaine: Number(creneauForm.jourSemaine),
                      groupeId: creneauForm.groupeId || null,
                    },
                  })
                }
                className="rounded-xl bg-slate-900 text-white px-3 py-2 text-sm font-bold disabled:opacity-50"
              >
                Ajouter créneau
              </button>
            </div>
          ) : null}
          <ul className="space-y-2 max-h-64 overflow-y-auto text-sm">
            {creneaux.map((c) => {
              const enConflit = conflits.some((x) => x.creneauIds.includes(c.id));
              return (
                <li
                  key={c.id}
                  className={`flex justify-between gap-2 border-b pb-2 ${
                    enConflit ? "border-amber-200 bg-amber-50/50 rounded-lg px-2" : "border-slate-100"
                  }`}
                >
                  <span>
                    <span className="font-semibold">
                      {JOURS[c.jourSemaine]} {c.heureDebut}–{c.heureFin}
                    </span>
                    <span className="text-slate-500 block text-xs">
                      {c.classe || c.groupeCode || "—"} · {c.enseignantNom || "—"} · {c.salle || "—"}{" "}
                      · sem. {c.semaine}
                    </span>
                  </span>
                  {canManage && !selfOnly ? (
                    <button
                      type="button"
                      className="text-xs text-red-600 font-bold"
                      onClick={() => void post({ action: "deleteCreneau", id: c.id })}
                    >
                      Suppr.
                    </button>
                  ) : null}
                </li>
              );
            })}
            {!creneaux.length && (
              <li className="text-slate-500">
                {selfOnly ? "Aucun créneau trouvé pour votre profil." : "Aucun créneau"}
              </li>
            )}
          </ul>
        </section>
      </div>
    </ModulePageShell>
  );
}
