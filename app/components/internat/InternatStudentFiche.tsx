"use client";

import { useCallback, useEffect, useState } from "react";
import type {
  InternatBuilding,
  InternatIncident,
  InternatIncidentKind,
  InternatRoom,
  InternatStudent,
} from "@/app/lib/internat-types";
import {
  INTERNAT_ROLL_MARK_LABELS,
  roomLocationLabel,
  studentDisplayName,
  type InternatRollMark,
} from "@/app/lib/internat-types";

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

type HistoryRow = {
  date: string;
  period: string;
  mark: InternatRollMark;
  markLabel: string;
};

function formatRoomOption(buildings: InternatBuilding[], room: InternatRoom) {
  const loc = roomLocationLabel(buildings, room);
  return loc === "Non classée" ? room.label : `${room.label} — ${loc}`;
}

export default function InternatStudentFiche({
  student,
  rooms,
  buildings,
  photoUrl,
  canManage,
  busy,
  onClose,
  onSave,
  onUpdateRoom,
  onSortie,
}: {
  student: InternatStudent;
  rooms: InternatRoom[];
  buildings: InternatBuilding[];
  photoUrl?: string;
  canManage: boolean;
  busy: boolean;
  onClose: () => void;
  onSave: (patch: Record<string, unknown>) => Promise<void>;
  onUpdateRoom: (roomId: string | null) => Promise<void>;
  onSortie: () => Promise<void>;
}) {
  const [tab, setTab] = useState<"infos" | "suivi" | "appel">("infos");
  const [medical, setMedical] = useState({
    allergies: student.medical?.allergies || "",
    pai: student.medical?.pai || "",
    treatments: student.medical?.treatments || "",
    notes: student.medical?.notes || "",
  });
  const [underWatch, setUnderWatch] = useState(!!student.underWatch);
  const [underWatchNote, setUnderWatchNote] = useState(student.underWatchNote || "");
  const [authLabel, setAuthLabel] = useState("");
  const [authValidUntil, setAuthValidUntil] = useState("");
  const [parent1, setParent1] = useState({
    nom: student.parent1?.nom || "",
    email: student.parent1?.email || "",
    telephone: student.parent1?.telephone || "",
  });
  const [parent2, setParent2] = useState({
    nom: student.parent2?.nom || "",
    email: student.parent2?.email || "",
    telephone: student.parent2?.telephone || "",
  });

  const [incidents, setIncidents] = useState<InternatIncident[]>([]);
  const [history, setHistory] = useState<HistoryRow[]>([]);
  const [loadingExtra, setLoadingExtra] = useState(false);
  const [incidentForm, setIncidentForm] = useState({
    kind: "sanction" as InternatIncidentKind,
    title: "",
    description: "",
    occurredAt: new Date().toISOString().slice(0, 10),
  });

  const initials =
    `${student.eleveRef.prenom?.[0] ?? ""}${student.eleveRef.nom?.[0] ?? ""}`.toUpperCase() || "?";
  const room = rooms.find((r) => r.id === student.roomId);

  const loadExtra = useCallback(async () => {
    setLoadingExtra(true);
    try {
      const [incRes, histRes] = await Promise.all([
        fetch(`/api/internat/incidents?studentId=${encodeURIComponent(student.id)}`, {
          cache: "no-store",
        }),
        fetch(
          `/api/internat/roll-call/history?studentId=${encodeURIComponent(student.id)}&limit=40`,
          { cache: "no-store" },
        ),
      ]);
      const incData = await incRes.json();
      const histData = await histRes.json();
      if (incRes.ok) setIncidents(incData.incidents || []);
      if (histRes.ok) {
        const rows = (histData.history || []) as Array<{
          date: string;
          period: string;
          mark: InternatRollMark;
          markLabel?: string;
        }>;
        setHistory(
          rows.map((r) => ({
            date: r.date,
            period: r.period,
            mark: r.mark,
            markLabel: r.markLabel || INTERNAT_ROLL_MARK_LABELS[r.mark] || r.mark,
          })),
        );
      }
    } finally {
      setLoadingExtra(false);
    }
  }, [student.id]);

  useEffect(() => {
    void loadExtra();
  }, [loadExtra]);

  useEffect(() => {
    setMedical({
      allergies: student.medical?.allergies || "",
      pai: student.medical?.pai || "",
      treatments: student.medical?.treatments || "",
      notes: student.medical?.notes || "",
    });
    setUnderWatch(!!student.underWatch);
    setUnderWatchNote(student.underWatchNote || "");
    setParent1({
      nom: student.parent1?.nom || "",
      email: student.parent1?.email || "",
      telephone: student.parent1?.telephone || "",
    });
    setParent2({
      nom: student.parent2?.nom || "",
      email: student.parent2?.email || "",
      telephone: student.parent2?.telephone || "",
    });
  }, [student]);

  const saveInfos = async () => {
    const newAuth =
      authLabel.trim()
        ? [
            ...(student.specialAuthorizations || []),
            {
              id: `auth_${Date.now()}`,
              label: authLabel.trim(),
              validUntil: authValidUntil || undefined,
            },
          ]
        : student.specialAuthorizations;
    await onSave({
      medical,
      underWatch,
      underWatchNote,
      parent1,
      parent2,
      specialAuthorizations: newAuth,
      note: "Fiche interne",
    });
    setAuthLabel("");
    setAuthValidUntil("");
  };

  const createIncident = async () => {
    if (!incidentForm.title.trim()) {
      alert("Titre requis.");
      return;
    }
    const res = await fetch("/api/internat/incidents", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...incidentForm, studentId: student.id }),
    });
    const data = await res.json();
    if (!res.ok) {
      alert(data?.error || "Enregistrement impossible");
      return;
    }
    setIncidentForm({
      kind: "sanction",
      title: "",
      description: "",
      occurredAt: new Date().toISOString().slice(0, 10),
    });
    await loadExtra();
  };

  const removeIncident = async (id: string) => {
    if (!confirm("Supprimer cette entrée ?")) return;
    await fetch(`/api/internat/incidents?id=${encodeURIComponent(id)}`, { method: "DELETE" });
    await loadExtra();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-black/45 p-0 sm:p-4">
      <div className="bg-white w-full sm:max-w-2xl sm:rounded-2xl shadow-2xl max-h-[100dvh] sm:max-h-[92vh] flex flex-col overflow-hidden rounded-t-2xl">
        <div className="relative bg-gradient-to-br from-slate-800 to-slate-900 text-white px-5 pt-5 pb-6">
          <button
            type="button"
            onClick={onClose}
            className="absolute top-3 right-3 h-9 w-9 rounded-full bg-white/10 hover:bg-white/20 text-lg font-bold"
            aria-label="Fermer"
          >
            ×
          </button>
          <div className="flex items-center gap-4">
            <div className="h-20 w-20 shrink-0 overflow-hidden rounded-2xl bg-white/10 border border-white/20 ring-2 ring-white/10">
              {photoUrl ? (
                // eslint-disable-next-line @next/next/no-img-element
                <img src={photoUrl} alt="" className="h-full w-full object-cover" />
              ) : (
                <span className="flex h-full w-full items-center justify-center text-xl font-black text-white/70">
                  {initials}
                </span>
              )}
            </div>
            <div className="min-w-0">
              <h2 className="text-xl font-black truncate">{studentDisplayName(student)}</h2>
              <p className="text-sm text-white/75 mt-0.5">
                {student.classe} · {student.etablissement} · {student.sexe === "F" ? "Fille" : "Garçon"}
              </p>
              <p className="text-xs text-white/55 mt-1">
                Chambre : {room ? formatRoomOption(buildings, room) : "Non assignée"}
              </p>
              {student.underWatch && (
                <span className="inline-block mt-2 text-[11px] font-bold px-2 py-0.5 rounded-md bg-amber-400/20 text-amber-100">
                  Sous surveillance
                </span>
              )}
            </div>
          </div>
        </div>

        <div className="flex border-b border-slate-200 bg-slate-50">
          {(
            [
              ["infos", "Fiche"],
              ["suivi", "Suivi"],
              ["appel", "Appels"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              onClick={() => setTab(id)}
              className={`flex-1 py-3 text-sm font-bold ${
                tab === id
                  ? "text-slate-900 border-b-2 border-slate-900 bg-white"
                  : "text-slate-500"
              }`}
            >
              {label}
            </button>
          ))}
        </div>

        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {tab === "infos" && (
            <>
              <section className="space-y-3">
                <h3 className="text-xs font-bold uppercase tracking-wide text-slate-500">Famille</h3>
                <div className="grid sm:grid-cols-2 gap-3">
                  <div className="space-y-2 rounded-xl border border-slate-200 p-3">
                    <p className="text-xs font-bold text-slate-500">Parent 1</p>
                    <input
                      className="w-full border rounded-lg px-3 py-2 text-sm"
                      placeholder="Nom"
                      disabled={!canManage}
                      value={parent1.nom}
                      onChange={(e) => setParent1({ ...parent1, nom: e.target.value })}
                    />
                    <input
                      className="w-full border rounded-lg px-3 py-2 text-sm"
                      placeholder="E-mail"
                      type="email"
                      disabled={!canManage}
                      value={parent1.email}
                      onChange={(e) => setParent1({ ...parent1, email: e.target.value })}
                    />
                    <input
                      className="w-full border rounded-lg px-3 py-2 text-sm"
                      placeholder="Téléphone"
                      disabled={!canManage}
                      value={parent1.telephone}
                      onChange={(e) => setParent1({ ...parent1, telephone: e.target.value })}
                    />
                  </div>
                  <div className="space-y-2 rounded-xl border border-slate-200 p-3">
                    <p className="text-xs font-bold text-slate-500">Parent 2</p>
                    <input
                      className="w-full border rounded-lg px-3 py-2 text-sm"
                      placeholder="Nom"
                      disabled={!canManage}
                      value={parent2.nom}
                      onChange={(e) => setParent2({ ...parent2, nom: e.target.value })}
                    />
                    <input
                      className="w-full border rounded-lg px-3 py-2 text-sm"
                      placeholder="E-mail"
                      type="email"
                      disabled={!canManage}
                      value={parent2.email}
                      onChange={(e) => setParent2({ ...parent2, email: e.target.value })}
                    />
                    <input
                      className="w-full border rounded-lg px-3 py-2 text-sm"
                      placeholder="Téléphone"
                      disabled={!canManage}
                      value={parent2.telephone}
                      onChange={(e) => setParent2({ ...parent2, telephone: e.target.value })}
                    />
                  </div>
                </div>
              </section>

              <section className="space-y-3">
                <h3 className="text-xs font-bold uppercase tracking-wide text-slate-500">Médical</h3>
                <input
                  className="w-full border rounded-xl px-3 py-2 text-sm"
                  placeholder="Allergies"
                  disabled={!canManage}
                  value={medical.allergies}
                  onChange={(e) => setMedical({ ...medical, allergies: e.target.value })}
                />
                <input
                  className="w-full border rounded-xl px-3 py-2 text-sm"
                  placeholder="PAI"
                  disabled={!canManage}
                  value={medical.pai}
                  onChange={(e) => setMedical({ ...medical, pai: e.target.value })}
                />
                <input
                  className="w-full border rounded-xl px-3 py-2 text-sm"
                  placeholder="Traitements"
                  disabled={!canManage}
                  value={medical.treatments}
                  onChange={(e) => setMedical({ ...medical, treatments: e.target.value })}
                />
                <textarea
                  className="w-full border rounded-xl px-3 py-2 text-sm min-h-[64px]"
                  placeholder="Notes médicales"
                  disabled={!canManage}
                  value={medical.notes}
                  onChange={(e) => setMedical({ ...medical, notes: e.target.value })}
                />
                <label className="flex items-center gap-2 text-sm font-semibold">
                  <input
                    type="checkbox"
                    disabled={!canManage}
                    checked={underWatch}
                    onChange={(e) => setUnderWatch(e.target.checked)}
                  />
                  Élève sous surveillance
                </label>
                {underWatch && (
                  <input
                    className="w-full border rounded-xl px-3 py-2 text-sm"
                    placeholder="Motif surveillance"
                    disabled={!canManage}
                    value={underWatchNote}
                    onChange={(e) => setUnderWatchNote(e.target.value)}
                  />
                )}
              </section>

              {canManage && (
                <section className="space-y-3">
                  <h3 className="text-xs font-bold uppercase tracking-wide text-slate-500">
                    Chambre & autorisations
                  </h3>
                  <select
                    className="w-full border rounded-xl px-3 py-2 text-sm"
                    value={student.roomId || ""}
                    onChange={(e) => void onUpdateRoom(e.target.value || null)}
                  >
                    <option value="">Chambre —</option>
                    {rooms.map((r) => (
                      <option key={r.id} value={r.id}>
                        {formatRoomOption(buildings, r)}
                      </option>
                    ))}
                  </select>
                  <input
                    className="w-full border rounded-xl px-3 py-2 text-sm"
                    placeholder="Nouvelle autorisation (ex. sortie vendredi)"
                    value={authLabel}
                    onChange={(e) => setAuthLabel(e.target.value)}
                  />
                  <input
                    type="date"
                    className="w-full border rounded-xl px-3 py-2 text-sm"
                    value={authValidUntil}
                    onChange={(e) => setAuthValidUntil(e.target.value)}
                  />
                  {student.specialAuthorizations?.length ? (
                    <ul className="text-xs text-slate-500 space-y-1">
                      {student.specialAuthorizations.map((a) => (
                        <li key={a.id}>
                          {a.label}
                          {a.validUntil ? ` (jusqu'au ${a.validUntil})` : ""}
                        </li>
                      ))}
                    </ul>
                  ) : null}
                </section>
              )}
            </>
          )}

          {tab === "suivi" && (
            <section className="space-y-4">
              {canManage && (
                <div className="rounded-xl border border-slate-200 p-4 space-y-3 bg-slate-50/80">
                  <h3 className="font-bold text-slate-900 text-sm">Nouvelle entrée</h3>
                  <div className="grid sm:grid-cols-2 gap-2">
                    <select
                      className="border rounded-xl px-3 py-2 text-sm"
                      value={incidentForm.kind}
                      onChange={(e) =>
                        setIncidentForm({
                          ...incidentForm,
                          kind: e.target.value as InternatIncidentKind,
                        })
                      }
                    >
                      {(Object.keys(KIND_LABELS) as InternatIncidentKind[]).map((k) => (
                        <option key={k} value={k}>
                          {KIND_LABELS[k]}
                        </option>
                      ))}
                    </select>
                    <input
                      type="date"
                      className="border rounded-xl px-3 py-2 text-sm"
                      value={incidentForm.occurredAt}
                      onChange={(e) =>
                        setIncidentForm({ ...incidentForm, occurredAt: e.target.value })
                      }
                    />
                    <input
                      className="border rounded-xl px-3 py-2 text-sm sm:col-span-2"
                      placeholder="Titre *"
                      value={incidentForm.title}
                      onChange={(e) => setIncidentForm({ ...incidentForm, title: e.target.value })}
                    />
                    <textarea
                      className="border rounded-xl px-3 py-2 text-sm sm:col-span-2 min-h-[64px]"
                      placeholder="Description"
                      value={incidentForm.description}
                      onChange={(e) =>
                        setIncidentForm({ ...incidentForm, description: e.target.value })
                      }
                    />
                  </div>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void createIncident()}
                    className="bg-slate-900 text-white px-4 py-2 rounded-xl text-sm font-bold"
                  >
                    Enregistrer
                  </button>
                </div>
              )}
              {loadingExtra && <p className="text-sm text-slate-500">Chargement…</p>}
              {!loadingExtra && incidents.length === 0 && (
                <p className="text-sm text-slate-500">Aucune entrée de suivi.</p>
              )}
              {incidents.map((i) => (
                <article key={i.id} className="rounded-xl border border-slate-200 p-3">
                  <div className="flex justify-between gap-2">
                    <div>
                      <span
                        className={`text-[11px] font-bold px-2 py-0.5 rounded-md ${KIND_TONE[i.kind]}`}
                      >
                        {KIND_LABELS[i.kind]}
                      </span>
                      <h4 className="font-bold text-slate-900 mt-1.5">{i.title}</h4>
                      <p className="text-xs text-slate-500">{i.occurredAt}</p>
                      {i.description && (
                        <p className="text-sm text-slate-600 mt-1">{i.description}</p>
                      )}
                    </div>
                    {canManage && (
                      <button
                        type="button"
                        className="text-xs text-red-600 font-bold shrink-0"
                        onClick={() => void removeIncident(i.id)}
                      >
                        Suppr.
                      </button>
                    )}
                  </div>
                </article>
              ))}
            </section>
          )}

          {tab === "appel" && (
            <section className="space-y-2">
              {loadingExtra && <p className="text-sm text-slate-500">Chargement…</p>}
              {!loadingExtra && history.length === 0 && (
                <p className="text-sm text-slate-500">Aucun appel enregistré pour cet interne.</p>
              )}
              {history.map((h, idx) => (
                <div
                  key={`${h.date}-${h.period}-${idx}`}
                  className="flex items-center justify-between rounded-xl border border-slate-200 px-3 py-2.5 text-sm"
                >
                  <div>
                    <p className="font-semibold text-slate-900">
                      {new Date(h.date + "T12:00:00").toLocaleDateString("fr-FR", {
                        weekday: "short",
                        day: "numeric",
                        month: "short",
                      })}
                    </p>
                    <p className="text-xs text-slate-500 capitalize">Appel du {h.period}</p>
                  </div>
                  <span
                    className={`text-xs font-bold px-2.5 py-1 rounded-lg ${
                      h.mark === "present"
                        ? "bg-emerald-100 text-emerald-800"
                        : h.mark === "absent"
                          ? "bg-red-100 text-red-800"
                          : h.mark === "excuse"
                            ? "bg-amber-100 text-amber-800"
                            : "bg-sky-100 text-sky-800"
                    }`}
                  >
                    {h.markLabel}
                  </span>
                </div>
              ))}
            </section>
          )}
        </div>

        <div className="border-t border-slate-200 p-4 flex flex-wrap gap-2 justify-between bg-white">
          <div>
            {canManage && student.actif && (
              <button
                type="button"
                disabled={busy}
                onClick={() => void onSortie()}
                className="px-3 py-2 rounded-xl text-sm font-bold text-red-600 hover:bg-red-50"
              >
                Sortie d&apos;année
              </button>
            )}
          </div>
          <div className="flex gap-2 ml-auto">
            <button
              type="button"
              className="px-4 py-2 rounded-xl text-sm font-bold text-slate-600"
              onClick={onClose}
            >
              Fermer
            </button>
            {canManage && tab === "infos" && (
              <button
                type="button"
                disabled={busy}
                onClick={() => void saveInfos()}
                className="px-4 py-2 rounded-xl text-sm font-bold bg-slate-900 text-white disabled:opacity-40"
              >
                Enregistrer
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
