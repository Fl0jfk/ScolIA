"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  InternatIncident,
  InternatIncidentKind,
  InternatIncidentSeverity,
  InternatStudent,
} from "@/app/lib/internat-types";
import { studentDisplayName } from "@/app/lib/internat-types";

const KIND_LABELS: Record<InternatIncidentKind, string> = {
  incident: "Incident",
  remarque: "Remarque",
  sanction: "Sanction",
  valorisation: "Valorisation",
};

const KIND_TONE: Record<InternatIncidentKind, string> = {
  incident: "bg-red-100 text-red-900",
  remarque: "bg-amber-100 text-amber-900",
  sanction: "bg-slate-200 text-slate-800",
  valorisation: "bg-emerald-100 text-emerald-900",
};

const SEVERITY_LABELS: Record<InternatIncidentSeverity, string> = {
  faible: "Faible",
  moyenne: "Moyenne",
  grave: "Grave",
};

type FormState = {
  studentId: string;
  kind: InternatIncidentKind;
  severity: InternatIncidentSeverity;
  title: string;
  description: string;
  location: string;
  occurredAt: string;
  occurredTime: string;
  witnesses: string;
  otherPeople: string;
  actionsTaken: string;
};

function emptyForm(): FormState {
  const now = new Date();
  return {
    studentId: "",
    kind: "incident",
    severity: "moyenne",
    title: "",
    description: "",
    location: "",
    occurredAt: now.toISOString().slice(0, 10),
    occurredTime: now.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
    witnesses: "",
    otherPeople: "",
    actionsTaken: "",
  };
}

export default function InternatEducationalPanel({
  students,
  canManage,
}: {
  students: InternatStudent[];
  canManage: boolean;
}) {
  const [incidents, setIncidents] = useState<InternatIncident[]>([]);
  const [busy, setBusy] = useState(false);
  const [filterKind, setFilterKind] = useState<"all" | InternatIncidentKind>("all");
  const [form, setForm] = useState<FormState>(emptyForm);

  const load = useCallback(async () => {
    const res = await fetch("/api/internat/incidents", { cache: "no-store" });
    const data = await res.json();
    if (res.ok) setIncidents(data.incidents || []);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const create = async () => {
    if (!form.studentId || !form.title.trim()) {
      alert("Choisissez un interne et un titre (quoi).");
      return;
    }
    setBusy(true);
    try {
      const kind = form.kind;
      const res = await fetch("/api/internat/incidents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(form),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      setForm(emptyForm());
      await load();
      if (kind === "incident" || kind === "sanction") {
        alert(
          data.mail?.sent
            ? "Incident enregistré. Un mail a été envoyé à la direction et au CPE de l’établissement."
            : "Incident enregistré (mail non envoyé — vérifiez SMTP / destinataires internat).",
        );
      }
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const remove = async (id: string) => {
    if (!confirm("Supprimer cette entrée ?")) return;
    await fetch(`/api/internat/incidents?id=${encodeURIComponent(id)}`, { method: "DELETE" });
    await load();
  };

  const active = useMemo(() => students.filter((s) => s.actif), [students]);
  const visible = useMemo(
    () => (filterKind === "all" ? incidents : incidents.filter((i) => i.kind === filterKind)),
    [incidents, filterKind],
  );

  return (
    <div className="space-y-6">
      <div className="rounded-2xl border border-orange-100 bg-orange-50/50 p-5 text-sm text-orange-950">
        <p className="font-bold mb-1">Incidents & suivi</p>
        <p>
          Déclarez un incident complet (qui, quoi, quand, où, témoins, mesures). Les incidents et
          sanctions partent automatiquement à la direction et au CPE de l’établissement de
          l’élève.
        </p>
      </div>

      {canManage && (
        <div className="bg-white border border-slate-200 rounded-2xl p-5 space-y-4">
          <h3 className="font-black text-slate-900">Déclarer un incident</h3>
          <p className="text-xs text-slate-500">
            Aide au rapport — remplissez ce que vous savez ; mieux vaut trop précis que trop vague.
          </p>
          <div className="grid sm:grid-cols-2 gap-3">
            <label className="text-xs font-semibold text-slate-600 space-y-1 sm:col-span-2">
              Qui (interne concerné) *
              <select
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                value={form.studentId}
                onChange={(e) => setForm({ ...form, studentId: e.target.value })}
              >
                <option value="">Choisir…</option>
                {active.map((s) => (
                  <option key={s.id} value={s.id}>
                    {studentDisplayName(s)} — {s.classe} · {s.etablissement}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Type
              <select
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                value={form.kind}
                onChange={(e) => setForm({ ...form, kind: e.target.value as InternatIncidentKind })}
              >
                {(Object.keys(KIND_LABELS) as InternatIncidentKind[]).map((k) => (
                  <option key={k} value={k}>
                    {KIND_LABELS[k]}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Gravité
              <select
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                value={form.severity}
                onChange={(e) =>
                  setForm({ ...form, severity: e.target.value as InternatIncidentSeverity })
                }
              >
                {(Object.keys(SEVERITY_LABELS) as InternatIncidentSeverity[]).map((s) => (
                  <option key={s} value={s}>
                    {SEVERITY_LABELS[s]}
                  </option>
                ))}
              </select>
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Quand (date)
              <input
                type="date"
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                value={form.occurredAt}
                onChange={(e) => setForm({ ...form, occurredAt: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Heure
              <input
                type="time"
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                value={form.occurredTime}
                onChange={(e) => setForm({ ...form, occurredTime: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1 sm:col-span-2">
              Quoi (titre court) *
              <input
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                placeholder="Ex. Altercation couloir 1er étage"
                value={form.title}
                onChange={(e) => setForm({ ...form, title: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1 sm:col-span-2">
              Où
              <input
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                placeholder="Ex. Chambre 12, cour, réfectoire…"
                value={form.location}
                onChange={(e) => setForm({ ...form, location: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1 sm:col-span-2">
              Faits (description)
              <textarea
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900 min-h-[100px]"
                placeholder="Ce qui s’est passé, chronologie, paroles entendues…"
                value={form.description}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Autres personnes impliquées
              <input
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                placeholder="Noms / rôles"
                value={form.otherPeople}
                onChange={(e) => setForm({ ...form, otherPeople: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Témoins
              <input
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900"
                placeholder="Qui a vu / entendu"
                value={form.witnesses}
                onChange={(e) => setForm({ ...form, witnesses: e.target.value })}
              />
            </label>
            <label className="text-xs font-semibold text-slate-600 space-y-1 sm:col-span-2">
              Mesures prises sur le moment
              <textarea
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900 min-h-[72px]"
                placeholder="Séparation, rappel à l’ordre, soin, appel parents…"
                value={form.actionsTaken}
                onChange={(e) => setForm({ ...form, actionsTaken: e.target.value })}
              />
            </label>
          </div>
          <button
            type="button"
            disabled={busy}
            onClick={() => void create()}
            className="bg-orange-600 text-white px-4 py-2.5 rounded-xl font-bold text-sm disabled:opacity-50"
          >
            Enregistrer & notifier
          </button>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <h3 className="font-black text-slate-900 mr-2">Historique</h3>
        <select
          className="border rounded-xl px-3 py-1.5 text-sm"
          value={filterKind}
          onChange={(e) => setFilterKind(e.target.value as "all" | InternatIncidentKind)}
        >
          <option value="all">Tous</option>
          {(Object.keys(KIND_LABELS) as InternatIncidentKind[]).map((k) => (
            <option key={k} value={k}>
              {KIND_LABELS[k]}
            </option>
          ))}
        </select>
      </div>

      <div className="space-y-3">
        {visible.length === 0 && <p className="text-sm text-slate-500">Aucune entrée.</p>}
        {visible.map((i) => (
          <article key={i.id} className="bg-white border border-slate-200 rounded-2xl p-4">
            <div className="flex flex-wrap items-start justify-between gap-2">
              <div className="min-w-0 space-y-1">
                <div className="flex flex-wrap gap-1.5">
                  <span className={`text-xs font-bold px-2 py-0.5 rounded-lg ${KIND_TONE[i.kind]}`}>
                    {KIND_LABELS[i.kind]}
                  </span>
                  {i.severity && (
                    <span className="text-xs font-bold px-2 py-0.5 rounded-lg bg-slate-100 text-slate-700">
                      {SEVERITY_LABELS[i.severity]}
                    </span>
                  )}
                  {i.notifySentAt && (
                    <span className="text-xs font-bold px-2 py-0.5 rounded-lg bg-indigo-50 text-indigo-800">
                      Notifié direction/CPE
                    </span>
                  )}
                </div>
                <h4 className="font-bold text-slate-900">{i.title}</h4>
                <p className="text-sm text-slate-600">
                  {i.studentName}
                  {i.classe ? ` · ${i.classe}` : ""}
                  {i.etablissement ? ` · ${i.etablissement}` : ""}
                  {" · "}
                  {i.occurredAt}
                  {i.occurredTime ? ` à ${i.occurredTime}` : ""}
                </p>
                {i.location && <p className="text-sm text-slate-500">Lieu : {i.location}</p>}
                {i.description && <p className="text-sm text-slate-600 mt-2 whitespace-pre-wrap">{i.description}</p>}
                {i.otherPeople && (
                  <p className="text-sm text-slate-500">Autres personnes : {i.otherPeople}</p>
                )}
                {i.witnesses && <p className="text-sm text-slate-500">Témoins : {i.witnesses}</p>}
                {i.actionsTaken && (
                  <p className="text-sm text-slate-600 mt-1">
                    <span className="font-semibold">Mesures : </span>
                    {i.actionsTaken}
                  </p>
                )}
                <p className="text-xs text-slate-400 mt-2">
                  Par {i.createdBy.name} — {new Date(i.createdAt).toLocaleString("fr-FR")}
                </p>
              </div>
              {canManage && (
                <button type="button" className="text-xs text-red-600 font-bold" onClick={() => void remove(i.id)}>
                  Supprimer
                </button>
              )}
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
