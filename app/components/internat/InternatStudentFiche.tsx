"use client";

import { useEffect, useState } from "react";
import InternatFicheAppels from "@/app/components/internat/InternatFicheAppels";
import InternatFicheFamille from "@/app/components/internat/InternatFicheFamille";
import InternatFicheSuivi from "@/app/components/internat/InternatFicheSuivi";
import type { InternatBuilding, InternatRoom, InternatStudent } from "@/app/lib/internat-types";
import { roomLocationLabel, studentDisplayName } from "@/app/lib/internat-types";

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
  const initials =
    `${student.eleveRef.prenom?.[0] ?? ""}${student.eleveRef.nom?.[0] ?? ""}`.toUpperCase() || "?";
  const room = rooms.find((r) => r.id === student.roomId);

  useEffect(() => {
    setMedical({
      allergies: student.medical?.allergies || "",
      pai: student.medical?.pai || "",
      treatments: student.medical?.treatments || "",
      notes: student.medical?.notes || "",
    });
    setUnderWatch(!!student.underWatch);
    setUnderWatchNote(student.underWatchNote || "");
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
      specialAuthorizations: newAuth,
      note: "Fiche interne",
    });
    setAuthLabel("");
    setAuthValidUntil("");
  };

  return (
    <div className="fixed inset-0 z-50 flex items-end sm:items-center justify-center bg-black/45 p-0 sm:p-4">
      <div className="bg-white w-full sm:max-w-3xl sm:rounded-2xl shadow-2xl max-h-[100dvh] sm:max-h-[92vh] flex flex-col overflow-hidden rounded-t-2xl">
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
              <InternatFicheFamille student={student} />

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
            <InternatFicheSuivi
              key={student.id}
              studentId={student.id}
              canManage={canManage}
              busy={busy}
            />
          )}

          {tab === "appel" && <InternatFicheAppels key={student.id} studentId={student.id} />}
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
