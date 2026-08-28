"use client";

import { useCallback, useEffect, useState } from "react";
import type { StageClassRoster, StageRosterStudentStatus } from "@/app/lib/stage-class-roster";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";

const STATUS_LABELS: Record<StageRosterStudentStatus, string> = {
  sans_stage: "Sans stage",
  en_cours: "En cours",
  valide: "Stage validé",
  plusieurs: "Plusieurs stages",
};

const STATUS_STYLES: Record<StageRosterStudentStatus, string> = {
  sans_stage: "bg-rose-50 text-rose-800 border-rose-200",
  en_cours: "bg-amber-50 text-amber-900 border-amber-200",
  valide: "bg-emerald-50 text-emerald-900 border-emerald-200",
  plusieurs: "bg-violet-50 text-violet-900 border-violet-200",
};

type RosterResponse = {
  schoolYear: string;
  availableClasses: string[];
  referent: { name: string; email: string } | null;
  roster: StageClassRoster | null;
  message?: string;
};

export default function StageClassRosterPanel({
  onOpenConvention,
  defaultSchoolYear,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
}: {
  onOpenConvention: (conventionId: string) => void;
  defaultSchoolYear?: string;
  canFileOneDrive?: boolean;
  oneDriveConnected?: boolean;
  onFileOneDrive?: (conventionId: string) => void;
  filingConventionId?: string | null;
}) {
  const [data, setData] = useState<RosterResponse | null>(null);
  const [selectedClass, setSelectedClass] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (className?: string) => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
      if (defaultSchoolYear) params.set("schoolYear", defaultSchoolYear);
      if (className) params.set("className", className);
      const res = await fetch(`/api/stages/class-roster?${params}`, { cache: "no-store" });
      const json = (await res.json()) as RosterResponse & { error?: string };
      if (!res.ok) throw new Error(json.error || "Erreur chargement");
      setData(json);
      if (json.roster?.className) setSelectedClass(json.roster.className);
      else if (json.availableClasses[0]) setSelectedClass(json.availableClasses[0]);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [defaultSchoolYear]);

  useEffect(() => {
    void load();
  }, [load]);

  const onClassChange = (className: string) => {
    setSelectedClass(className);
    void load(className);
  };

  if (loading && !data) {
    return <p className="text-sm text-stone-500">Chargement du suivi classe…</p>;
  }

  if (error) {
    return (
      <p className="rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
        {error}
      </p>
    );
  }

  if (data?.message && !data.roster) {
    return (
      <div className="rounded-xl border border-amber-200 bg-amber-50 px-4 py-4 text-sm text-amber-900">
        {data.message}
      </div>
    );
  }

  const roster = data?.roster;
  if (!roster) return null;

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-end gap-4">
        {data.availableClasses.length > 1 && (
          <label className="text-sm font-semibold text-stone-700">
            Classe
            <select
              className="mt-1 block min-w-[140px] rounded-lg border border-stone-300 px-3 py-2 text-sm"
              value={selectedClass}
              onChange={(e) => onClassChange(e.target.value)}
            >
              {data.availableClasses.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
        )}
        {data.availableClasses.length === 1 && (
          <p className="text-lg font-bold text-[#1F3D2B]">Classe {roster.className}</p>
        )}
        <p className="text-xs text-stone-500">Année {roster.schoolYear}</p>
      </div>

      {roster.note && (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
          {roster.note}
        </p>
      )}

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
        {[
          ["Élèves", roster.summary.total, "text-[#1F3D2B]"],
          ["Sans stage", roster.summary.sansStage, "text-rose-700"],
          ["En cours", roster.summary.enCours, "text-amber-800"],
          ["Validés", roster.summary.valide, "text-emerald-800"],
          ["Plusieurs", roster.summary.plusieurs, "text-violet-800"],
        ].map(([label, n, color]) => (
          <div key={String(label)} className="rounded-xl border border-stone-200 bg-white p-4 shadow-sm">
            <p className="text-xs text-stone-500">{label}</p>
            <p className={`text-2xl font-black mt-1 ${color}`}>{n}</p>
          </div>
        ))}
      </div>

      <div className="overflow-x-auto rounded-2xl border border-stone-200 bg-white shadow-sm">
        <table className="min-w-full text-sm">
          <thead>
            <tr className="border-b border-stone-100 bg-stone-50/80 text-left text-xs font-bold uppercase tracking-wide text-stone-500">
              <th className="px-4 py-3">Élève</th>
              <th className="px-4 py-3">Statut</th>
              <th className="px-4 py-3">Entreprise / période</th>
              <th className="px-4 py-3">Signatures</th>
              <th className="px-4 py-3">Dossier</th>
              {canFileOneDrive ? <th className="px-4 py-3">OneDrive</th> : null}
            </tr>
          </thead>
          <tbody className="divide-y divide-stone-100">
            {roster.students.map((student) => (
              <tr key={student.key} className="hover:bg-stone-50/50">
                <td className="px-4 py-3 font-semibold text-[#1F3D2B]">
                  {student.prenom} {student.nom}
                  {student.ine ? (
                    <span className="ml-1 text-xs font-normal text-stone-400">({student.ine})</span>
                  ) : null}
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex rounded-full border px-2.5 py-0.5 text-xs font-semibold ${STATUS_STYLES[student.rosterStatus]}`}
                  >
                    {STATUS_LABELS[student.rosterStatus]}
                  </span>
                </td>
                <td className="px-4 py-3 text-stone-600">
                  {student.conventions.length === 0 ? (
                    <span className="text-stone-400">—</span>
                  ) : (
                    <ul className="space-y-1">
                      {student.conventions.map((c) => (
                        <li key={c.id}>
                          <button
                            type="button"
                            onClick={() => onOpenConvention(c.id)}
                            className="text-left text-[#2F6B4A] underline hover:no-underline"
                          >
                            {c.stageLabel ? `${c.stageLabel} — ` : ""}
                            {c.companyName}
                          </button>
                          <span className="text-xs text-stone-400">
                            {" "}
                            · {c.periodStart} → {c.periodEnd}
                          </span>
                          <span className="ml-1 text-xs text-stone-500">({c.statusLabel})</span>
                        </li>
                      ))}
                    </ul>
                  )}
                </td>
                <td className="px-4 py-3 min-w-[140px]">
                  {student.conventions.length === 0 ? (
                    <span className="text-stone-400">—</span>
                  ) : (
                    <ul className="space-y-2">
                      {student.conventions.map((c) => (
                        <li key={c.id}>
                          <StageSignatureProgress summary={c.signatureSummary} compact />
                        </li>
                      ))}
                    </ul>
                  )}
                </td>
                <td className="px-4 py-3 text-xs text-stone-500">
                  {student.conventions.length > 0 ? (
                    <button
                      type="button"
                      className="font-semibold text-[#2F6B4A] underline"
                      onClick={() => onOpenConvention(student.conventions[0]!.id)}
                    >
                      Ouvrir
                    </button>
                  ) : (
                    "—"
                  )}
                </td>
                {canFileOneDrive ? (
                  <td className="px-4 py-3 text-xs">
                    {student.conventions.length === 0 ? (
                      "—"
                    ) : (
                      <ul className="space-y-1">
                        {student.conventions.map((c) => (
                          <li key={c.id}>
                            {c.oneDriveFiled ? (
                              <span className="text-emerald-700 font-semibold">Déposé</span>
                            ) : c.canFileOneDrive ? (
                              <button
                                type="button"
                                disabled={!oneDriveConnected || filingConventionId === c.id}
                                onClick={() => onFileOneDrive?.(c.id)}
                                className="font-semibold text-[#2F6B4A] underline disabled:opacity-50"
                              >
                                {filingConventionId === c.id ? "Envoi…" : "→ OneDrive"}
                              </button>
                            ) : (
                              <span className="text-stone-400">—</span>
                            )}
                          </li>
                        ))}
                      </ul>
                    )}
                  </td>
                ) : null}
              </tr>
            ))}
          </tbody>
        </table>
        {roster.students.length === 0 && (
          <p className="px-4 py-8 text-center text-sm text-stone-500">Aucun élève pour cette classe.</p>
        )}
      </div>
    </div>
  );
}
