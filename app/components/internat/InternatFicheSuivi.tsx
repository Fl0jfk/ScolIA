"use client";

import { useCallback, useEffect, useState } from "react";
import { filterIncidents, schoolYearOf } from "@/app/lib/internat-suivi-shared";
import { parisDateKey } from "@/app/lib/paris-time";
import type { InternatIncident, InternatIncidentKind } from "@/app/lib/internat-types";

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

const KINDS = Object.keys(KIND_LABELS) as InternatIncidentKind[];
const PAGE_SIZE = 10;

const frDate = (d: string) => d.split("-").reverse().join("/");

/** Récapitulatif des incidents et sanctions de l'internat (uniquement) pour un interne. */
export default function InternatFicheSuivi({
  studentId,
  canManage,
  busy,
}: {
  studentId: string;
  canManage: boolean;
  busy: boolean;
}) {
  const today = parisDateKey(new Date());
  const emptyForm = {
    kind: "incident" as InternatIncidentKind,
    title: "",
    description: "",
    occurredAt: today,
  };
  const [incidents, setIncidents] = useState<InternatIncident[] | null>(null);
  const [error, setError] = useState("");
  const [form, setForm] = useState(emptyForm);
  const [formOpen, setFormOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [kind, setKind] = useState<InternatIncidentKind | "all">("all");
  const [schoolYear, setSchoolYear] = useState<number | "all">(schoolYearOf(today));
  const [query, setQuery] = useState("");
  const [ascending, setAscending] = useState(false);
  const [shown, setShown] = useState(PAGE_SIZE);
  const [openId, setOpenId] = useState<string | null>(null);

  const load = useCallback(async () => {
    setError("");
    try {
      const res = await fetch(`/api/internat/incidents?studentId=${encodeURIComponent(studentId)}`, {
        cache: "no-store",
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement du suivi impossible.");
      setIncidents(data.incidents || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Chargement du suivi impossible.");
    }
  }, [studentId]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => setShown(PAGE_SIZE), [kind, schoolYear, query, ascending]);

  const create = async () => {
    if (!form.title.trim()) {
      alert("Titre requis.");
      return;
    }
    setSaving(true);
    try {
      const res = await fetch("/api/internat/incidents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ...form, studentId }),
      });
      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        alert(data?.error || "Enregistrement impossible");
        return;
      }
      setForm(emptyForm);
      setFormOpen(false);
      await load();
    } finally {
      setSaving(false);
    }
  };

  const remove = async ({ id, studentIds }: InternatIncident) => {
    const shared = studentIds?.length ?? 0;
    const question =
      shared > 1
        ? `Cette entrée concerne ${shared} internes : elle sera supprimée pour tous. Supprimer ?`
        : "Supprimer cette entrée ?";
    if (!confirm(question)) return;
    const res = await fetch(`/api/internat/incidents?id=${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
    if (!res.ok) alert("Suppression impossible.");
    await load();
  };

  const all = incidents || [];
  const years = [...new Set([schoolYearOf(today), ...all.map((i) => schoolYearOf(i.occurredAt))])].sort(
    (a, b) => b - a,
  );
  const ofYear = filterIncidents(all, { kind: "all", schoolYear, query: "", ascending: false });
  const list = filterIncidents(all, { kind, schoolYear, query, ascending });

  return (
    <section className="space-y-3">
      {canManage && !formOpen && (
        <button
          type="button"
          onClick={() => setFormOpen(true)}
          className="bg-slate-900 text-white px-4 py-2 rounded-xl text-sm font-bold"
        >
          + Nouvelle entrée
        </button>
      )}
      {canManage && formOpen && (
        <div className="rounded-xl border border-slate-200 p-4 space-y-3 bg-slate-50/80">
          <h3 className="font-bold text-slate-900 text-sm">Nouvelle entrée</h3>
          <div className="grid sm:grid-cols-2 gap-2">
            <select
              className="border rounded-xl px-3 py-2 text-sm"
              value={form.kind}
              onChange={(e) => setForm({ ...form, kind: e.target.value as InternatIncidentKind })}
            >
              {KINDS.map((k) => (
                <option key={k} value={k}>
                  {KIND_LABELS[k]}
                </option>
              ))}
            </select>
            <input
              type="date"
              className="border rounded-xl px-3 py-2 text-sm"
              value={form.occurredAt}
              onChange={(e) => setForm({ ...form, occurredAt: e.target.value })}
            />
            <input
              className="border rounded-xl px-3 py-2 text-sm sm:col-span-2"
              placeholder="Titre *"
              value={form.title}
              onChange={(e) => setForm({ ...form, title: e.target.value })}
            />
            <textarea
              className="border rounded-xl px-3 py-2 text-sm sm:col-span-2 min-h-[64px]"
              placeholder="Description"
              value={form.description}
              onChange={(e) => setForm({ ...form, description: e.target.value })}
            />
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              disabled={busy || saving}
              onClick={() => void create()}
              className="bg-slate-900 text-white px-4 py-2 rounded-xl text-sm font-bold disabled:opacity-40"
            >
              Enregistrer
            </button>
            <button
              type="button"
              onClick={() => setFormOpen(false)}
              className="px-4 py-2 rounded-xl text-sm font-bold text-slate-600"
            >
              Annuler
            </button>
          </div>
        </div>
      )}

      <div className="grid grid-cols-4 gap-2">
        {KINDS.map((k) => (
          <button
            key={k}
            type="button"
            onClick={() => setKind(kind === k ? "all" : k)}
            className={`rounded-xl px-2 py-2 text-center ${KIND_TONE[k]} ${
              kind === k ? "ring-2 ring-slate-900" : ""
            }`}
          >
            <span className="block text-lg font-black">{ofYear.filter((i) => i.kind === k).length}</span>
            <span className="block text-[11px] font-bold truncate">{KIND_LABELS[k]}s</span>
          </button>
        ))}
      </div>

      <div className="flex flex-wrap gap-2">
        <select
          className="border rounded-xl px-3 py-2 text-sm"
          value={String(schoolYear)}
          onChange={(e) => setSchoolYear(e.target.value === "all" ? "all" : Number(e.target.value))}
          aria-label="Année scolaire"
        >
          {years.map((y) => (
            <option key={y} value={y}>
              Année {y}-{y + 1}
            </option>
          ))}
          <option value="all">Toutes les années</option>
        </select>
        <button
          type="button"
          onClick={() => setAscending(!ascending)}
          className="border rounded-xl px-3 py-2 text-sm font-semibold"
        >
          {ascending ? "Plus anciens d'abord ↑" : "Plus récents d'abord ↓"}
        </button>
        <input
          type="search"
          className="border rounded-xl px-3 py-2 text-sm flex-1 min-w-[10rem]"
          placeholder="Chercher (titre, lieu, date…)"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      {error && (
        <p className="text-sm text-red-700">
          {error}{" "}
          <button type="button" className="font-bold underline" onClick={() => void load()}>
            Réessayer
          </button>
        </p>
      )}
      {!error && incidents === null && <p className="text-sm text-slate-500">Chargement…</p>}
      {incidents !== null && !list.length && (
        <p className="text-sm text-slate-500">Aucune entrée de suivi pour ces critères.</p>
      )}

      <ul className="space-y-2">
        {list.slice(0, shown).map((i) => {
          const open = openId === i.id;
          return (
            <li key={i.id} className="rounded-xl border border-slate-200">
              <button
                type="button"
                onClick={() => setOpenId(open ? null : i.id)}
                aria-expanded={open}
                className="w-full flex items-center gap-2 px-3 py-2.5 text-left text-sm"
              >
                <span className="text-xs text-slate-500 shrink-0 w-[4.75rem]">{frDate(i.occurredAt)}</span>
                <span className={`shrink-0 text-[11px] font-bold px-2 py-0.5 rounded-md ${KIND_TONE[i.kind]}`}>
                  {KIND_LABELS[i.kind]}
                </span>
                <span className="font-bold text-slate-900 truncate flex-1">{i.title}</span>
                <span className="text-slate-400">{open ? "▴" : "▾"}</span>
              </button>
              {open && (
                <div className="border-t border-slate-100 px-3 py-2.5 text-sm text-slate-600 space-y-1">
                  <p className="font-bold text-slate-900">{i.title}</p>
                  {i.description && <p className="whitespace-pre-wrap">{i.description}</p>}
                  {(i.occurredTime || i.location) && (
                    <p>{[i.occurredTime, i.location].filter(Boolean).join(" · ")}</p>
                  )}
                  {(i.studentNames?.length || 0) > 1 && <p>Internes concernés : {i.studentNames!.join(", ")}</p>}
                  {i.witnesses && <p>Témoins : {i.witnesses}</p>}
                  {i.actionsTaken && <p>Mesures prises : {i.actionsTaken}</p>}
                  <div className="flex justify-between gap-2 pt-1">
                    <p className="text-xs text-slate-400">Saisi par {i.createdBy?.name || "—"}</p>
                    {canManage && (
                      <button
                        type="button"
                        className="text-xs text-red-600 font-bold"
                        onClick={() => void remove(i)}
                      >
                        Supprimer
                      </button>
                    )}
                  </div>
                </div>
              )}
            </li>
          );
        })}
      </ul>
      {list.length > shown && (
        <button
          type="button"
          onClick={() => setShown(shown + PAGE_SIZE)}
          className="w-full border rounded-xl py-2 text-sm font-bold text-slate-600"
        >
          Voir plus ({list.length - shown} restants)
        </button>
      )}
    </section>
  );
}
