"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useSearchParams } from "next/navigation";
import type { InternatBuilding, InternatRoom, InternatStudent } from "@/app/lib/internat-types";
import { roomLocationLabel, studentDisplayName } from "@/app/lib/internat-types";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import { useAppContext } from "@/app/hooks/useAppContext";
import { internatEligibleEstablishments } from "@/app/lib/establishment-catalog";

type RosterPreviewRow = {
  nom: string;
  prenom: string;
  classe?: string;
  mef?: string;
  preview?: { etablissement: string; classe: string; mefResolved: boolean };
};

type RosterMeta = {
  updatedAt?: string;
  updatedBy?: string;
  lastAppliedAt?: string;
  lastApplySummary?: { added: number; updated: number; skipped: number };
};

function formatRoomOption(buildings: InternatBuilding[], room: InternatRoom) {
  const loc = roomLocationLabel(buildings, room);
  return loc === "Non classée" ? room.label : `${room.label} — ${loc}`;
}

export default function InternatStudentsPanel({
  students,
  rooms,
  buildings = [],
  canManage,
  onRefresh,
}: {
  students: InternatStudent[];
  rooms: InternatRoom[];
  buildings?: InternatBuilding[];
  canManage: boolean;
  onRefresh: () => Promise<void>;
}) {
  const searchParams = useSearchParams();
  const { data: appCtx } = useAppContext();
  const internatSites = internatEligibleEstablishments(appCtx?.establishments || []);
  const defaultInternatEtab = internatSites[0]?.label || "";
  const [showRoster, setShowRoster] = useState(false);
  const [rosterEntries, setRosterEntries] = useState<RosterPreviewRow[]>([]);
  const [rosterMeta, setRosterMeta] = useState<RosterMeta | null>(null);
  const [rosterMessage, setRosterMessage] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editingParentsId, setEditingParentsId] = useState<string | null>(null);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [detailDraft, setDetailDraft] = useState({
    allergies: "",
    pai: "",
    treatments: "",
    medicalNotes: "",
    underWatch: false,
    underWatchNote: "",
    authLabel: "",
    authValidUntil: "",
  });
  const [parentDraft, setParentDraft] = useState({
    p1nom: "",
    p1email: "",
    p1tel: "",
    p2nom: "",
    p2email: "",
    p2tel: "",
  });

  const [manual, setManual] = useState({
    nom: "",
    prenom: "",
    classe: "",
    sexe: "M" as "M" | "F",
    etablissement: defaultInternatEtab,
    roomId: "",
  });

  const loadRoster = useCallback(async () => {
    try {
      const res = await fetch("/api/internat/students/roster", { cache: "no-store" });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Chargement impossible");
      if (data.count > 0) {
        setRosterEntries(data.entries || []);
        setRosterMeta(data.meta || null);
        setShowRoster(true);
      } else {
        setRosterEntries([]);
        setRosterMeta(null);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    if (canManage) void loadRoster();
  }, [canManage, loadRoster]);

  const uploadRosterFile = async (file: File) => {
    setBusy(true);
    setRosterMessage(null);
    setError(null);
    try {
      const text = await file.text();
      const parsed = JSON.parse(text);
      const res = await fetch("/api/internat/students/roster", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(parsed),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      setRosterMessage(data.message || "Liste enregistrée.");
      await loadRoster();
      setShowRoster(true);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
      if (fileInputRef.current) fileInputRef.current.value = "";
    }
  };

  const applyRoster = async () => {
    if (!rosterEntries.length) return alert("Chargez d'abord une liste internat.");
    if (!confirm(`Importer / synchroniser ${rosterEntries.length} interne(s) depuis la liste ?`)) return;
    setBusy(true);
    setRosterMessage(null);
    try {
      const res = await fetch("/api/internat/students/roster", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "apply" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Import impossible");
      setRosterMessage(data.message);
      await loadRoster();
      await onRefresh();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const createManual = async () => {
    setBusy(true);
    try {
      const res = await fetch("/api/internat/students", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          ...manual,
          roomId: manual.roomId || null,
        }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Création impossible");
      setManual({ nom: "", prenom: "", classe: "", sexe: "M", etablissement: defaultInternatEtab, roomId: "" });
      await onRefresh();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const openDetail = (s: InternatStudent) => {
    setDetailId(s.id);
    setDetailDraft({
      allergies: s.medical?.allergies || "",
      pai: s.medical?.pai || "",
      treatments: s.medical?.treatments || "",
      medicalNotes: s.medical?.notes || "",
      underWatch: !!s.underWatch,
      underWatchNote: s.underWatchNote || "",
      authLabel: "",
      authValidUntil: "",
    });
  };

  useEffect(() => {
    const studentId = searchParams.get("student");
    if (!studentId) return;
    const student = students.find((s) => s.id === studentId);
    if (student) openDetail(student);
  }, [searchParams, students]);

  const saveDetail = async () => {
    if (!detailId) return;
    const student = students.find((s) => s.id === detailId);
    const newAuth =
      detailDraft.authLabel.trim() && student
        ? [
            ...(student.specialAuthorizations || []),
            {
              id: `auth_${Date.now()}`,
              label: detailDraft.authLabel.trim(),
              validUntil: detailDraft.authValidUntil || undefined,
            },
          ]
        : student?.specialAuthorizations;
    await updateStudent(detailId, {
      medical: {
        allergies: detailDraft.allergies,
        pai: detailDraft.pai,
        treatments: detailDraft.treatments,
        notes: detailDraft.medicalNotes,
      },
      specialAuthorizations: newAuth,
      underWatch: detailDraft.underWatch,
      underWatchNote: detailDraft.underWatchNote,
      note: "Fiche enrichie",
    });
    setDetailId(null);
  };

  const openParentEdit = (s: InternatStudent) => {
    setEditingParentsId(s.id);
    setParentDraft({
      p1nom: s.parent1?.nom || "",
      p1email: s.parent1?.email || "",
      p1tel: s.parent1?.telephone || "",
      p2nom: s.parent2?.nom || "",
      p2email: s.parent2?.email || "",
      p2tel: s.parent2?.telephone || "",
    });
  };

  const saveParents = async (id: string) => {
    await updateStudent(id, {
      parent1: {
        nom: parentDraft.p1nom,
        email: parentDraft.p1email,
        telephone: parentDraft.p1tel,
      },
      parent2: {
        nom: parentDraft.p2nom,
        email: parentDraft.p2email,
        telephone: parentDraft.p2tel,
      },
      note: "Contacts parents",
    });
    setEditingParentsId(null);
  };

  const updateStudent = async (id: string, patch: Record<string, unknown>) => {
    setBusy(true);
    try {
      const res = await fetch("/api/internat/students", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, ...patch }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Mise à jour impossible");
      await onRefresh();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      {canManage && (
        <div className="rounded-2xl border border-indigo-100 bg-indigo-50/50 p-5 space-y-4 text-sm text-indigo-950">
          <div>
            <p className="font-bold mb-1">Liste des internes (fichier dédié)</p>
            <p>
              Chargez un JSON séparé de <code className="text-xs bg-white px-1 rounded">eleves.json</code> : uniquement
              les élèves internes. Le collège / lycée est déduit du MEF (table Paramètres → MEF) ou du champ{" "}
              <code className="text-xs bg-white px-1 rounded">etablissement</code>.
            </p>
          </div>
          <div className="flex flex-wrap gap-3 items-center">
            <input
              ref={fileInputRef}
              type="file"
              accept=".json,application/json"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void uploadRosterFile(f);
              }}
            />
            <button
              type="button"
              disabled={busy}
              onClick={() => fileInputRef.current?.click()}
              className="bg-indigo-600 text-white px-4 py-2 rounded-xl font-bold text-sm"
            >
              Charger la liste JSON
            </button>
            {rosterEntries.length > 0 && (
              <button
                type="button"
                disabled={busy}
                onClick={applyRoster}
                className="bg-emerald-600 text-white px-4 py-2 rounded-xl font-bold text-sm"
              >
                Importer les {rosterEntries.length} internes
              </button>
            )}
            {rosterEntries.length > 0 && (
              <button
                type="button"
                className="text-indigo-700 font-bold text-sm"
                onClick={() => setShowRoster((v) => !v)}
              >
                {showRoster ? "Masquer l'aperçu" : "Voir l'aperçu"}
              </button>
            )}
          </div>
          {rosterMeta?.updatedAt && (
            <p className="text-xs text-indigo-800/80">
              Liste enregistrée le {new Date(rosterMeta.updatedAt).toLocaleString("fr-FR")}
              {rosterMeta.updatedBy ? ` par ${rosterMeta.updatedBy}` : ""}
              {rosterMeta.lastAppliedAt
                ? ` — dernier import : ${new Date(rosterMeta.lastAppliedAt).toLocaleString("fr-FR")}`
                : ""}
            </p>
          )}
          {rosterMessage && <p className="text-xs font-semibold text-emerald-800">{rosterMessage}</p>}
          <details className="text-xs">
            <summary className="cursor-pointer font-bold">Format JSON attendu</summary>
            <pre className="mt-2 p-3 bg-white rounded-lg overflow-x-auto text-[11px]">{`[
  {
    "nom": "Dupont",
    "prenom": "Marie",
    "ine": "123456789AB",
    "classe": "3eA",
    "mef": "32033421320",
    "sexe": "F",
    "parent1": { "email": "parent@exemple.fr" }
  }
]`}</pre>
          </details>
        </div>
      )}

      {showRoster && rosterEntries.length > 0 && (
        <div className="bg-white border border-slate-200 rounded-2xl p-5 max-h-[24rem] overflow-y-auto">
          <div className="flex justify-between items-center mb-3">
            <h3 className="font-black">Aperçu liste internat ({rosterEntries.length})</h3>
            <button type="button" className="text-sm font-bold text-slate-500" onClick={() => setShowRoster(false)}>
              Fermer
            </button>
          </div>
          <table className="w-full text-sm">
            <thead className="text-left text-xs text-slate-500">
              <tr>
                <th className="pb-2">Élève</th>
                <th className="pb-2">Classe</th>
                <th className="pb-2">MEF</th>
                <th className="pb-2">Établ.</th>
              </tr>
            </thead>
            <tbody>
              {rosterEntries.map((e) => (
                <tr key={`${e.nom}-${e.prenom}`} className="border-t border-slate-100">
                  <td className="py-2 font-medium">
                    {e.prenom} {e.nom}
                  </td>
                  <td className="py-2">{e.preview?.classe || e.classe || "—"}</td>
                  <td className="py-2 text-xs text-slate-500">
                    {e.mef || "—"}
                    {e.preview?.mefResolved === false && e.mef && (
                      <span className="text-amber-700 block">MEF non reconnu</span>
                    )}
                  </td>
                  <td className="py-2">{e.preview?.etablissement || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {canManage && (
        <div className="bg-slate-50 border border-slate-200 rounded-2xl p-5 space-y-3">
          <h3 className="font-black text-slate-900">Ajout manuel</h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
            <input className="border rounded-xl px-3 py-2 text-sm" placeholder="Nom" value={manual.nom} onChange={(e) => setManual({ ...manual, nom: e.target.value })} />
            <input className="border rounded-xl px-3 py-2 text-sm" placeholder="Prénom" value={manual.prenom} onChange={(e) => setManual({ ...manual, prenom: e.target.value })} />
            <input className="border rounded-xl px-3 py-2 text-sm" placeholder="Classe" value={manual.classe} onChange={(e) => setManual({ ...manual, classe: e.target.value })} />
            <select className="border rounded-xl px-3 py-2 text-sm" value={manual.sexe} onChange={(e) => setManual({ ...manual, sexe: e.target.value as "M" | "F" })}>
              <option value="M">Garçon</option>
              <option value="F">Fille</option>
            </select>
            <EstablishmentSelect
              className="border rounded-xl px-3 py-2 text-sm"
              value={manual.etablissement}
              onChange={(label) => setManual({ ...manual, etablissement: label })}
              kinds={["college", "lycee", "custom"]}
            />
            <select className="border rounded-xl px-3 py-2 text-sm" value={manual.roomId} onChange={(e) => setManual({ ...manual, roomId: e.target.value })}>
              <option value="">Chambre —</option>
              {rooms.map((r) => (
                <option key={r.id} value={r.id}>{formatRoomOption(buildings, r)}</option>
              ))}
            </select>
          </div>
          <button type="button" disabled={busy} onClick={createManual} className="bg-indigo-600 text-white px-4 py-2 rounded-xl font-bold text-sm">
            Créer l&apos;interne
          </button>
        </div>
      )}

      {error && <p className="text-sm text-red-600">{error}</p>}

      <div className="bg-white border border-slate-200 rounded-2xl overflow-hidden">
        <table className="w-full text-sm">
          <thead className="bg-slate-50 text-left">
            <tr>
              <th className="p-3 font-bold">Interne</th>
              <th className="p-3 font-bold">Classe</th>
              <th className="p-3 font-bold">Établ.</th>
              <th className="p-3 font-bold">Sexe</th>
              <th className="p-3 font-bold">Chambre</th>
              <th className="p-3 font-bold">Parents</th>
              <th className="p-3 font-bold">Fiche</th>
              {canManage && <th className="p-3 font-bold">Actions</th>}
            </tr>
          </thead>
          <tbody>
            {students.filter((s) => s.actif).map((s) => (
              <tr key={s.id} className="border-t border-slate-100">
                <td className="p-3 font-semibold">{studentDisplayName(s)}</td>
                <td className="p-3">{s.classe}</td>
                <td className="p-3">{s.etablissement}</td>
                <td className="p-3">{s.sexe === "M" ? "G" : "F"}</td>
                <td className="p-3">
                  {canManage ? (
                    <select
                      className="border rounded-lg px-2 py-1 text-xs"
                      value={s.roomId || ""}
                      onChange={(e) => updateStudent(s.id, { roomId: e.target.value || null })}
                    >
                      <option value="">—</option>
                      {rooms.map((r) => (
                        <option key={r.id} value={r.id}>{formatRoomOption(buildings, r)}</option>
                      ))}
                    </select>
                  ) : (
                    (() => {
                      const room = rooms.find((r) => r.id === s.roomId);
                      return room ? formatRoomOption(buildings, room) : "—";
                    })()
                  )}
                </td>
                <td className="p-3 text-xs">
                  {s.parent1?.email || s.parent2?.email ? (
                    <span className="text-slate-600">
                      {[s.parent1?.email, s.parent2?.email].filter(Boolean).join(" · ")}
                    </span>
                  ) : (
                    <span className="text-amber-700 font-semibold">À compléter</span>
                  )}
                  {canManage && (
                    <button
                      type="button"
                      className="block mt-1 text-indigo-600 font-bold"
                      onClick={() => openParentEdit(s)}
                    >
                      Modifier
                    </button>
                  )}
                </td>
                <td className="p-3 text-xs">
                  {s.underWatch && (
                    <span className="text-amber-800 font-bold block">Sous surveillance</span>
                  )}
                  {(s.medical?.allergies || s.medical?.pai) && (
                    <span className="text-slate-500">Médical renseigné</span>
                  )}
                  <button type="button" className="block mt-1 text-indigo-600 font-bold" onClick={() => openDetail(s)}>
                    Voir / modifier
                  </button>
                </td>
                {canManage && (
                  <td className="p-3">
                    <button
                      type="button"
                      className="text-xs text-red-600 font-bold"
                      onClick={() => updateStudent(s.id, { actif: false })}
                    >
                      Désactiver
                    </button>
                  </td>
                )}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {detailId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 overflow-y-auto">
          <div className="bg-white rounded-2xl shadow-xl max-w-lg w-full p-5 space-y-4 my-8">
            <h3 className="font-black text-slate-900">Fiche interne</h3>
            <div className="space-y-3 text-sm">
              <p className="text-xs font-bold uppercase text-slate-500">Médical</p>
              <input className="w-full border rounded-xl px-3 py-2" placeholder="Allergies" value={detailDraft.allergies} onChange={(e) => setDetailDraft({ ...detailDraft, allergies: e.target.value })} />
              <input className="w-full border rounded-xl px-3 py-2" placeholder="PAI" value={detailDraft.pai} onChange={(e) => setDetailDraft({ ...detailDraft, pai: e.target.value })} />
              <input className="w-full border rounded-xl px-3 py-2" placeholder="Traitements" value={detailDraft.treatments} onChange={(e) => setDetailDraft({ ...detailDraft, treatments: e.target.value })} />
              <textarea className="w-full border rounded-xl px-3 py-2 min-h-[60px]" placeholder="Notes médicales" value={detailDraft.medicalNotes} onChange={(e) => setDetailDraft({ ...detailDraft, medicalNotes: e.target.value })} />
              <label className="flex items-center gap-2 font-semibold">
                <input type="checkbox" checked={detailDraft.underWatch} onChange={(e) => setDetailDraft({ ...detailDraft, underWatch: e.target.checked })} />
                Élève sous surveillance
              </label>
              {detailDraft.underWatch && (
                <input className="w-full border rounded-xl px-3 py-2" placeholder="Motif surveillance" value={detailDraft.underWatchNote} onChange={(e) => setDetailDraft({ ...detailDraft, underWatchNote: e.target.value })} />
              )}
              {canManage && (
                <>
                  <p className="text-xs font-bold uppercase text-slate-500 pt-2">Autorisation spéciale</p>
                  <input className="w-full border rounded-xl px-3 py-2" placeholder="Libellé (ex. sortie régulière vendredi)" value={detailDraft.authLabel} onChange={(e) => setDetailDraft({ ...detailDraft, authLabel: e.target.value })} />
                  <input type="date" className="w-full border rounded-xl px-3 py-2" value={detailDraft.authValidUntil} onChange={(e) => setDetailDraft({ ...detailDraft, authValidUntil: e.target.value })} />
                  {students.find((s) => s.id === detailId)?.specialAuthorizations?.length ? (
                    <ul className="text-xs text-slate-500 space-y-1">
                      {students
                        .find((s) => s.id === detailId)!
                        .specialAuthorizations!.map((a) => (
                          <li key={a.id}>
                            {a.label}
                            {a.validUntil ? ` (jusqu'au ${a.validUntil})` : ""}
                          </li>
                        ))}
                    </ul>
                  ) : null}
                </>
              )}
            </div>
            <div className="flex gap-2 justify-end">
              <button type="button" className="px-4 py-2 rounded-xl text-sm font-bold text-slate-600" onClick={() => setDetailId(null)}>
                Fermer
              </button>
              {canManage && (
                <button type="button" disabled={busy} className="px-4 py-2 rounded-xl text-sm font-bold bg-indigo-600 text-white" onClick={saveDetail}>
                  Enregistrer
                </button>
              )}
            </div>
          </div>
        </div>
      )}

      {editingParentsId && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
          <div className="bg-white rounded-2xl shadow-xl max-w-md w-full p-5 space-y-4">
            <h3 className="font-black text-slate-900">Contacts parents</h3>
            <div className="space-y-3">
              <p className="text-xs font-bold uppercase text-slate-500">Parent 1</p>
              <input className="w-full border rounded-xl px-3 py-2 text-sm" placeholder="Nom" value={parentDraft.p1nom} onChange={(e) => setParentDraft({ ...parentDraft, p1nom: e.target.value })} />
              <input className="w-full border rounded-xl px-3 py-2 text-sm" placeholder="E-mail *" type="email" value={parentDraft.p1email} onChange={(e) => setParentDraft({ ...parentDraft, p1email: e.target.value })} />
              <input className="w-full border rounded-xl px-3 py-2 text-sm" placeholder="Téléphone" value={parentDraft.p1tel} onChange={(e) => setParentDraft({ ...parentDraft, p1tel: e.target.value })} />
              <p className="text-xs font-bold uppercase text-slate-500 pt-2">Parent 2 (optionnel)</p>
              <input className="w-full border rounded-xl px-3 py-2 text-sm" placeholder="Nom" value={parentDraft.p2nom} onChange={(e) => setParentDraft({ ...parentDraft, p2nom: e.target.value })} />
              <input className="w-full border rounded-xl px-3 py-2 text-sm" placeholder="E-mail" type="email" value={parentDraft.p2email} onChange={(e) => setParentDraft({ ...parentDraft, p2email: e.target.value })} />
              <input className="w-full border rounded-xl px-3 py-2 text-sm" placeholder="Téléphone" value={parentDraft.p2tel} onChange={(e) => setParentDraft({ ...parentDraft, p2tel: e.target.value })} />
            </div>
            <div className="flex gap-2 justify-end">
              <button type="button" className="px-4 py-2 rounded-xl text-sm font-bold text-slate-600" onClick={() => setEditingParentsId(null)}>
                Annuler
              </button>
              <button
                type="button"
                disabled={busy}
                className="px-4 py-2 rounded-xl text-sm font-bold bg-indigo-600 text-white"
                onClick={() => saveParents(editingParentsId)}
              >
                Enregistrer
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
