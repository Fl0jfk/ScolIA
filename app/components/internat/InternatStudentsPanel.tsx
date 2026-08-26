"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import type { InternatBuilding, InternatRoom, InternatStudent } from "@/app/lib/internat-types";
import { roomLocationLabel, studentDisplayName } from "@/app/lib/internat-types";
import EstablishmentSelect from "@/app/components/establishments/EstablishmentSelect";
import InternatStudentFiche from "@/app/components/internat/InternatStudentFiche";
import { useAppContext } from "@/app/hooks/useAppContext";
import { internatEligibleEstablishments } from "@/app/lib/establishment-catalog";
import {
  INTERNAT_NIVEAUX,
  niveauFromClasse,
  niveauSortKey,
} from "@/app/lib/internat-level";

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
  lastApplySummary?: {
    added: number;
    updated: number;
    skipped: number;
    sorties?: number;
    reactivated?: number;
  };
};

function formatRoomOption(buildings: InternatBuilding[], room: InternatRoom) {
  const loc = roomLocationLabel(buildings, room);
  return loc === "Non classée" ? room.label : `${room.label} — ${loc}`;
}

export default function InternatStudentsPanel({
  students,
  rooms,
  buildings = [],
  photoUrls = {},
  canManage,
  onRefresh,
}: {
  students: InternatStudent[];
  rooms: InternatRoom[];
  buildings?: InternatBuilding[];
  photoUrls?: Record<string, string>;
  canManage: boolean;
  onRefresh: () => Promise<void>;
}) {
  const searchParams = useSearchParams();
  const { data: appCtx } = useAppContext();
  const internatSites = internatEligibleEstablishments(appCtx?.establishments || []);
  const defaultInternatEtab = internatSites[0]?.label || "";
  const [showTools, setShowTools] = useState(false);
  const [showRoster, setShowRoster] = useState(false);
  const [rosterEntries, setRosterEntries] = useState<RosterPreviewRow[]>([]);
  const [rosterMeta, setRosterMeta] = useState<RosterMeta | null>(null);
  const [rosterMessage, setRosterMessage] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const excelXmlInputRef = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [detailId, setDetailId] = useState<string | null>(null);
  const [filterSexe, setFilterSexe] = useState<"all" | "M" | "F">("all");
  const [filterEtab, setFilterEtab] = useState<string>("all");
  const [filterNiveau, setFilterNiveau] = useState<string>("all");
  const [query, setQuery] = useState("");

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
      } else {
        setRosterEntries([]);
        setRosterMeta(null);
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    }
  }, []);

  useEffect(() => {
    if (canManage && showTools) void loadRoster();
  }, [canManage, showTools, loadRoster]);

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

  const syncFromEleves = async () => {
    setBusy(true);
    setRosterMessage(null);
    setError(null);
    try {
      const res = await fetch("/api/internat/students/roster", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "syncFromEleves" }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Sync impossible");
      setRosterMessage(data.message);
      await loadRoster();
      await onRefresh();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const uploadExcelOrSiecle = async (file: File) => {
    setBusy(true);
    setRosterMessage(null);
    setError(null);
    try {
      const fd = new FormData();
      fd.set("file", file);
      const isXml = file.name.toLowerCase().endsWith(".xml");
      fd.set("action", isXml ? "importSiecle" : "importFile");
      const res = await fetch("/api/internat/students/roster", { method: "POST", body: fd });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Import impossible");
      setRosterMessage(data.message || "Import OK.");
      await loadRoster();
      await onRefresh();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
      if (excelXmlInputRef.current) excelXmlInputRef.current.value = "";
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
      setManual({
        nom: "",
        prenom: "",
        classe: "",
        sexe: "M",
        etablissement: defaultInternatEtab,
        roomId: "",
      });
      await onRefresh();
    } catch (e: unknown) {
      alert(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  useEffect(() => {
    const studentId = searchParams.get("student");
    if (!studentId) return;
    if (students.some((s) => s.id === studentId)) setDetailId(studentId);
  }, [searchParams, students]);

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

  const etablissements = useMemo(() => {
    const set = new Set<string>();
    for (const s of students) {
      if (s.actif && s.etablissement) set.add(s.etablissement);
    }
    return [...set].sort((a, b) => a.localeCompare(b, "fr"));
  }, [students]);

  const activeFiltered = useMemo(() => {
    const q = query.trim().toLowerCase();
    return students
      .filter((s) => s.actif)
      .filter((s) => (filterSexe === "all" ? true : s.sexe === filterSexe))
      .filter((s) => (filterEtab === "all" ? true : s.etablissement === filterEtab))
      .filter((s) => {
        if (filterNiveau === "all") return true;
        return niveauFromClasse(s.classe) === filterNiveau;
      })
      .filter((s) => {
        if (!q) return true;
        const name = studentDisplayName(s).toLowerCase();
        return name.includes(q) || s.classe.toLowerCase().includes(q);
      })
      .sort((a, b) => {
        const na = niveauSortKey(niveauFromClasse(a.classe));
        const nb = niveauSortKey(niveauFromClasse(b.classe));
        if (na !== nb) return na - nb;
        return studentDisplayName(a).localeCompare(studentDisplayName(b), "fr");
      });
  }, [students, filterSexe, filterEtab, filterNiveau, query]);

  const detailStudent = detailId ? students.find((s) => s.id === detailId) : undefined;

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-end gap-3 justify-between">
        <div>
          <h2 className="text-lg font-black text-slate-900">Internes</h2>
          <p className="text-sm text-slate-500">
            {activeFiltered.length} fiche{activeFiltered.length !== 1 ? "s" : ""} — cliquez pour ouvrir
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <input
            className="border border-slate-200 rounded-xl px-3 py-2 text-sm min-w-[10rem] flex-1 sm:flex-none"
            placeholder="Rechercher…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <select
            className="border border-slate-200 rounded-xl px-3 py-2 text-sm"
            value={filterSexe}
            onChange={(e) => setFilterSexe(e.target.value as "all" | "M" | "F")}
          >
            <option value="all">Tous</option>
            <option value="F">Filles</option>
            <option value="M">Garçons</option>
          </select>
          <select
            className="border border-slate-200 rounded-xl px-3 py-2 text-sm"
            value={filterEtab}
            onChange={(e) => setFilterEtab(e.target.value)}
          >
            <option value="all">Établissement</option>
            {etablissements.map((e) => (
              <option key={e} value={e}>
                {e}
              </option>
            ))}
          </select>
          <select
            className="border border-slate-200 rounded-xl px-3 py-2 text-sm"
            value={filterNiveau}
            onChange={(e) => setFilterNiveau(e.target.value)}
          >
            <option value="all">Niveau</option>
            {INTERNAT_NIVEAUX.map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </div>
      </div>

      {activeFiltered.length === 0 ? (
        <p className="text-sm text-slate-500 rounded-2xl border border-dashed border-slate-200 p-8 text-center">
          Aucun interne à afficher.
        </p>
      ) : (
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3 sm:gap-4">
          {activeFiltered.map((s) => {
            const photo = photoUrls[s.id];
            const initials =
              `${s.eleveRef.prenom?.[0] ?? ""}${s.eleveRef.nom?.[0] ?? ""}`.toUpperCase() || "?";
            const room = rooms.find((r) => r.id === s.roomId);
            return (
              <button
                key={s.id}
                type="button"
                onClick={() => setDetailId(s.id)}
                className="group text-left bg-white border border-slate-200 rounded-2xl overflow-hidden hover:border-slate-400 hover:shadow-md transition-all focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
              >
                <div className="aspect-[4/5] bg-slate-100 relative overflow-hidden">
                  {photo ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img
                      src={photo}
                      alt=""
                      className="h-full w-full object-cover group-hover:scale-[1.02] transition-transform"
                    />
                  ) : (
                    <span className="flex h-full w-full items-center justify-center text-2xl font-black text-slate-400">
                      {initials}
                    </span>
                  )}
                  {s.underWatch && (
                    <span className="absolute top-2 left-2 text-[10px] font-bold px-1.5 py-0.5 rounded-md bg-amber-500 text-white">
                      Suivi
                    </span>
                  )}
                  <span
                    className={`absolute top-2 right-2 text-[10px] font-bold px-1.5 py-0.5 rounded-md ${
                      s.sexe === "F" ? "bg-rose-500/90 text-white" : "bg-sky-600/90 text-white"
                    }`}
                  >
                    {s.sexe === "F" ? "F" : "G"}
                  </span>
                </div>
                <div className="p-2.5 sm:p-3">
                  <p className="font-bold text-slate-900 text-sm leading-tight truncate">
                    {s.eleveRef.prenom}
                  </p>
                  <p className="font-semibold text-slate-700 text-xs truncate uppercase tracking-wide">
                    {s.eleveRef.nom}
                  </p>
                  <p className="text-[11px] text-slate-500 mt-1 truncate">
                    {s.classe} · {s.etablissement}
                  </p>
                  <p className="text-[11px] text-slate-400 mt-0.5 truncate">
                    {room ? room.label : "Sans chambre"}
                  </p>
                </div>
              </button>
            );
          })}
        </div>
      )}

      {students.some((s) => !s.actif) && (
        <div className="bg-slate-50 border border-slate-200 rounded-2xl overflow-hidden opacity-90">
          <div className="px-4 py-3 border-b border-slate-200">
            <h3 className="font-bold text-slate-700 text-sm">
              Sorties en cours d&apos;année ({students.filter((s) => !s.actif).length})
            </h3>
            <p className="text-xs text-slate-500 mt-0.5">
              Fiches conservées — exclus de l&apos;appel. Réactivables en un clic.
            </p>
          </div>
          <ul className="divide-y divide-slate-200/80">
            {students
              .filter((s) => !s.actif)
              .sort((a, b) =>
                String(b.sortieAt || b.updatedAt).localeCompare(String(a.sortieAt || a.updatedAt)),
              )
              .map((s) => (
                <li
                  key={s.id}
                  className="px-4 py-3 flex flex-wrap items-center justify-between gap-2 text-sm text-slate-500"
                >
                  <button
                    type="button"
                    className="font-semibold line-through decoration-slate-300 text-left hover:text-slate-700"
                    onClick={() => setDetailId(s.id)}
                  >
                    {studentDisplayName(s)}
                  </button>
                  <span className="text-xs">{s.sortieMotif || "Sortie"}</span>
                  {canManage && (
                    <button
                      type="button"
                      className="text-xs text-emerald-700 font-bold"
                      onClick={() =>
                        void updateStudent(s.id, { actif: true, note: "Réactivation manuelle" })
                      }
                    >
                      Réactiver
                    </button>
                  )}
                </li>
              ))}
          </ul>
        </div>
      )}

      {canManage && (
        <div className="border-t border-slate-200 pt-6">
          <button
            type="button"
            onClick={() => setShowTools((v) => !v)}
            className="text-xs font-semibold text-slate-400 hover:text-slate-600"
          >
            {showTools ? "Masquer les outils" : "Outils — import, sync, ajout manuel"}
          </button>

          {showTools && (
            <div className="mt-4 space-y-4">
              <div className="rounded-2xl border border-slate-200 bg-slate-50/80 p-5 space-y-4 text-sm">
                <p className="font-bold text-slate-800">Import & synchronisation</p>
                <p className="text-slate-600 text-xs leading-relaxed">
                  Synchronisez les internes depuis le référentiel élèves (régime interne) ou importez un
                  Excel / XML Siècle. Les photos se gèrent dans Paramètres.
                </p>
                <div className="flex flex-wrap gap-2">
                  <input
                    ref={excelXmlInputRef}
                    type="file"
                    accept=".xlsx,.xls,.csv,.xml,application/xml,text/xml"
                    className="hidden"
                    onChange={(e) => {
                      const f = e.target.files?.[0];
                      if (f) void uploadExcelOrSiecle(f);
                    }}
                  />
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
                    onClick={() => excelXmlInputRef.current?.click()}
                    className="bg-slate-800 text-white px-3 py-2 rounded-xl font-bold text-xs"
                  >
                    Excel / XML
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => void syncFromEleves()}
                    className="bg-emerald-700 text-white px-3 py-2 rounded-xl font-bold text-xs"
                  >
                    Sync référentiel
                  </button>
                  <Link
                    href="/parametres?tab=photos"
                    className="bg-white border border-slate-200 text-slate-700 px-3 py-2 rounded-xl font-bold text-xs inline-flex items-center"
                  >
                    Photos
                  </Link>
                  <button
                    type="button"
                    disabled={busy}
                    onClick={() => fileInputRef.current?.click()}
                    className="border border-slate-300 bg-white text-slate-600 px-3 py-2 rounded-xl font-bold text-xs"
                  >
                    JSON
                  </button>
                  {rosterEntries.length > 0 && (
                    <>
                      <button
                        type="button"
                        disabled={busy}
                        onClick={() => void applyRoster()}
                        className="bg-teal-700 text-white px-3 py-2 rounded-xl font-bold text-xs"
                      >
                        Ré-appliquer ({rosterEntries.length})
                      </button>
                      <button
                        type="button"
                        className="text-slate-600 font-bold text-xs"
                        onClick={() => setShowRoster((v) => !v)}
                      >
                        {showRoster ? "Masquer aperçu" : "Aperçu"}
                      </button>
                    </>
                  )}
                </div>
                {error && (
                  <p className="rounded-lg bg-red-50 px-3 py-2 text-xs font-semibold text-red-700" role="alert">
                    {error}
                  </p>
                )}
                {rosterMeta?.updatedAt && (
                  <p className="text-xs text-slate-500">
                    Liste enregistrée le {new Date(rosterMeta.updatedAt).toLocaleString("fr-FR")}
                  </p>
                )}
                {rosterMessage && <p className="text-xs font-semibold text-emerald-800">{rosterMessage}</p>}
              </div>

              {showRoster && rosterEntries.length > 0 && (
                <div className="bg-white border border-slate-200 rounded-2xl p-4 max-h-[20rem] overflow-y-auto">
                  <h3 className="font-bold text-sm mb-2">Aperçu ({rosterEntries.length})</h3>
                  <table className="w-full text-sm">
                    <thead className="text-left text-xs text-slate-500">
                      <tr>
                        <th className="pb-2">Élève</th>
                        <th className="pb-2">Classe</th>
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
                          <td className="py-2">{e.preview?.etablissement || "—"}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}

              <div className="bg-white border border-slate-200 rounded-2xl p-4 space-y-3">
                <h3 className="font-bold text-slate-900 text-sm">Ajout manuel</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
                  <input
                    className="border rounded-xl px-3 py-2 text-sm"
                    placeholder="Nom"
                    value={manual.nom}
                    onChange={(e) => setManual({ ...manual, nom: e.target.value })}
                  />
                  <input
                    className="border rounded-xl px-3 py-2 text-sm"
                    placeholder="Prénom"
                    value={manual.prenom}
                    onChange={(e) => setManual({ ...manual, prenom: e.target.value })}
                  />
                  <input
                    className="border rounded-xl px-3 py-2 text-sm"
                    placeholder="Classe"
                    value={manual.classe}
                    onChange={(e) => setManual({ ...manual, classe: e.target.value })}
                  />
                  <select
                    className="border rounded-xl px-3 py-2 text-sm"
                    value={manual.sexe}
                    onChange={(e) => setManual({ ...manual, sexe: e.target.value as "M" | "F" })}
                  >
                    <option value="M">Garçon</option>
                    <option value="F">Fille</option>
                  </select>
                  <EstablishmentSelect
                    className="border rounded-xl px-3 py-2 text-sm"
                    value={manual.etablissement}
                    onChange={(label) => setManual({ ...manual, etablissement: label })}
                    kinds={["college", "lycee", "custom"]}
                  />
                  <select
                    className="border rounded-xl px-3 py-2 text-sm"
                    value={manual.roomId}
                    onChange={(e) => setManual({ ...manual, roomId: e.target.value })}
                  >
                    <option value="">Chambre —</option>
                    {rooms.map((r) => (
                      <option key={r.id} value={r.id}>
                        {formatRoomOption(buildings, r)}
                      </option>
                    ))}
                  </select>
                </div>
                <button
                  type="button"
                  disabled={busy}
                  onClick={() => void createManual()}
                  className="bg-slate-900 text-white px-4 py-2 rounded-xl font-bold text-sm"
                >
                  Créer l&apos;interne
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {detailStudent && (
        <InternatStudentFiche
          student={detailStudent}
          rooms={rooms}
          buildings={buildings}
          photoUrl={photoUrls[detailStudent.id]}
          canManage={canManage}
          busy={busy}
          onClose={() => setDetailId(null)}
          onSave={async (patch) => {
            await updateStudent(detailStudent.id, patch);
          }}
          onUpdateRoom={async (roomId) => {
            await updateStudent(detailStudent.id, { roomId });
          }}
          onSortie={async () => {
            if (!confirm("Marquer cet interne en sortie d'année ?")) return;
            await updateStudent(detailStudent.id, {
              actif: false,
              sortieMotif: "Désactivation manuelle",
              note: "Désactivation manuelle",
            });
            setDetailId(null);
          }}
        />
      )}
    </div>
  );
}
