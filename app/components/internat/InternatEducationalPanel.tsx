"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { InternatIncident, InternatStudent } from "@/app/lib/internat-types";
import { compareInternatStudentsByLastName, studentDisplayName } from "@/app/lib/internat-types";

type StaffWitness = { userId: string; name: string; email?: string };

type FormState = {
  studentIds: string[];
  title: string;
  description: string;
  location: string;
  occurredAt: string;
  occurredTime: string;
  witnessStudentIds: string[];
  witnessStaffIds: string[];
  actionsTaken: string;
};

function emptyForm(): FormState {
  const now = new Date();
  return {
    studentIds: [],
    title: "",
    description: "",
    location: "",
    occurredAt: now.toISOString().slice(0, 10),
    occurredTime: now.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
    witnessStudentIds: [],
    witnessStaffIds: [],
    actionsTaken: "",
  };
}

function toggleId(list: string[], id: string) {
  return list.includes(id) ? list.filter((x) => x !== id) : [...list, id];
}

function MiniTrombi({
  students,
  photoUrls,
  selectedIds,
  onToggle,
  excludeIds,
}: {
  students: InternatStudent[];
  photoUrls: Record<string, string>;
  selectedIds: string[];
  onToggle: (id: string) => void;
  excludeIds?: string[];
}) {
  const excluded = new Set(excludeIds || []);
  const list = students.filter((s) => s.actif && !excluded.has(s.id));
  return (
    <div className="grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 gap-2">
      {list.map((s) => {
        const selected = selectedIds.includes(s.id);
        const photo = photoUrls[s.id];
        const initials =
          `${s.eleveRef.prenom?.[0] ?? ""}${s.eleveRef.nom?.[0] ?? ""}`.toUpperCase() || "?";
        return (
          <button
            key={s.id}
            type="button"
            onClick={() => onToggle(s.id)}
            className={`rounded-xl border p-1.5 text-center transition-colors ${
              selected
                ? "border-orange-500 bg-orange-50 ring-2 ring-orange-200"
                : "border-slate-200 bg-white hover:border-slate-300"
            }`}
            title={`${studentDisplayName(s)} — ${s.classe}`}
          >
            <div className="mx-auto h-10 w-10 overflow-hidden rounded-full bg-slate-100 border border-slate-200">
              {photo ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={photo} alt="" className="h-full w-full object-cover" />
              ) : (
                <span className="flex h-full w-full items-center justify-center text-[10px] font-bold text-slate-500">
                  {initials}
                </span>
              )}
            </div>
            <p className="mt-1 text-[10px] font-semibold text-slate-800 truncate leading-tight">
              {s.eleveRef.nom}
            </p>
            <p className="text-[9px] text-slate-500 truncate">{s.classe}</p>
          </button>
        );
      })}
    </div>
  );
}

export default function InternatEducationalPanel({
  students,
  photoUrls = {},
  canManage,
}: {
  students: InternatStudent[];
  photoUrls?: Record<string, string>;
  canManage: boolean;
}) {
  const [incidents, setIncidents] = useState<InternatIncident[]>([]);
  const [staffWitnesses, setStaffWitnesses] = useState<StaffWitness[]>([]);
  const [busy, setBusy] = useState(false);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [search, setSearch] = useState("");

  const load = useCallback(async () => {
    const res = await fetch("/api/internat/incidents", { cache: "no-store" });
    const data = await res.json();
    if (res.ok) {
      setIncidents(data.incidents || []);
      setStaffWitnesses(data.staffWitnesses || []);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const active = useMemo(() => {
    const q = search.trim().toLowerCase();
    return students
      .filter((s) => s.actif)
      .filter((s) => {
        if (!q) return true;
        return (
          studentDisplayName(s).toLowerCase().includes(q) ||
          (s.classe || "").toLowerCase().includes(q)
        );
      })
      .sort(compareInternatStudentsByLastName);
  }, [students, search]);

  const create = async () => {
    if (form.studentIds.length === 0 || !form.title.trim()) {
      alert("Choisissez au moins un interne et un titre (quoi).");
      return;
    }
    setBusy(true);
    try {
      const res = await fetch("/api/internat/incidents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(form),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      setForm(emptyForm());
      setSearch("");
      await load();
      alert(
        data.mail?.sent
          ? "Incident enregistré. Mail envoyé à la direction / CPE concerné(s) (collège et/ou lycée)."
          : "Incident enregistré (mail non envoyé — vérifiez SMTP / destinataires).",
      );
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

  return (
    <div className="space-y-6 max-w-3xl">
      <div className="rounded-2xl border border-orange-100 bg-orange-50/50 p-4 text-sm text-orange-950">
        <p className="font-bold mb-0.5">Déclarer un incident</p>
        <p className="text-orange-900/90">
          Choisissez les internes, décrivez les faits et les mesures. Notification automatique
          collège et/ou lycée selon les élèves concernés.
        </p>
      </div>

      {canManage && (
        <div className="bg-white border border-slate-200 rounded-2xl p-5 space-y-5">
          <div className="space-y-2">
            <div className="flex flex-wrap items-end justify-between gap-2">
              <label className="text-sm font-bold text-slate-800">
                Internes concernés *
                {form.studentIds.length > 0 ? (
                  <span className="ml-2 font-semibold text-orange-700">
                    {form.studentIds.length} sélectionné
                    {form.studentIds.length > 1 ? "s" : ""}
                  </span>
                ) : null}
              </label>
              <input
                className="border rounded-xl px-3 py-1.5 text-sm w-full sm:w-56"
                placeholder="Filtrer nom / classe…"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
              />
            </div>
            <MiniTrombi
              students={active}
              photoUrls={photoUrls}
              selectedIds={form.studentIds}
              onToggle={(id) => setForm({ ...form, studentIds: toggleId(form.studentIds, id) })}
            />
          </div>

          <div className="grid sm:grid-cols-2 gap-3">
            <label className="text-xs font-semibold text-slate-600 space-y-1">
              Date
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
              Quoi *
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
              Faits
              <textarea
                className="w-full border rounded-xl px-3 py-2 text-sm font-normal text-slate-900 min-h-[110px]"
                placeholder="Ce qui s’est passé, chronologie, paroles entendues…"
                value={form.description}
                onChange={(e) => setForm({ ...form, description: e.target.value })}
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

          <div className="space-y-2">
            <p className="text-sm font-bold text-slate-800">Témoins — internes</p>
            <MiniTrombi
              students={active}
              photoUrls={photoUrls}
              selectedIds={form.witnessStudentIds}
              excludeIds={form.studentIds}
              onToggle={(id) =>
                setForm({ ...form, witnessStudentIds: toggleId(form.witnessStudentIds, id) })
              }
            />
          </div>

          <div className="space-y-2">
            <p className="text-sm font-bold text-slate-800">Témoins — surveillants / internat</p>
            {staffWitnesses.length === 0 ? (
              <p className="text-xs text-slate-500">Aucun personnel avec le rôle surveillant ou internat.</p>
            ) : (
              <div className="max-h-40 overflow-y-auto rounded-xl border border-slate-200 divide-y divide-slate-100">
                {staffWitnesses.map((m) => {
                  const selected = form.witnessStaffIds.includes(m.userId);
                  return (
                    <label
                      key={m.userId}
                      className={`flex items-center gap-3 px-3 py-2 text-sm cursor-pointer ${
                        selected ? "bg-orange-50" : "bg-white hover:bg-slate-50"
                      }`}
                    >
                      <input
                        type="checkbox"
                        checked={selected}
                        onChange={() =>
                          setForm({
                            ...form,
                            witnessStaffIds: toggleId(form.witnessStaffIds, m.userId),
                          })
                        }
                      />
                      <span className="font-semibold text-slate-800">{m.name}</span>
                    </label>
                  );
                })}
              </div>
            )}
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

      <div className="space-y-3">
        <h3 className="font-black text-slate-900">Historique</h3>
        {incidents.length === 0 && <p className="text-sm text-slate-500">Aucun incident.</p>}
        {incidents.map((i) => (
          <article key={i.id} className="bg-white border border-slate-200 rounded-2xl p-4">
            <div className="flex flex-wrap items-start justify-between gap-2">
              <div className="min-w-0 space-y-1">
                <div className="flex flex-wrap gap-1.5">
                  <span className="text-xs font-bold px-2 py-0.5 rounded-lg bg-red-100 text-red-900">
                    Incident
                  </span>
                  {i.notifySentAt && (
                    <span className="text-xs font-bold px-2 py-0.5 rounded-lg bg-indigo-50 text-indigo-800">
                      Notifié
                    </span>
                  )}
                </div>
                <h4 className="font-bold text-slate-900">{i.title}</h4>
                <p className="text-sm text-slate-600">
                  {(i.studentNames && i.studentNames.length > 0
                    ? i.studentNames.join(", ")
                    : i.studentName)}
                  {i.classe ? ` · ${i.classe}` : ""}
                  {" · "}
                  {i.occurredAt}
                  {i.occurredTime ? ` à ${i.occurredTime}` : ""}
                </p>
                {i.location && <p className="text-sm text-slate-500">Lieu : {i.location}</p>}
                {i.description && (
                  <p className="text-sm text-slate-600 mt-2 whitespace-pre-wrap">{i.description}</p>
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
                <button
                  type="button"
                  className="text-xs text-red-600 font-bold"
                  onClick={() => void remove(i.id)}
                >
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
