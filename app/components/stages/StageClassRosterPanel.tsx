"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { StageClassRoster, StageRosterStudentStatus } from "@/app/lib/stage-class-roster";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";

type RosterStatusFilter = "all" | StageRosterStudentStatus;

function normalizeSearch(value: string): string {
  return value
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .trim();
}

function statusLabel(
  status: StageRosterStudentStatus,
  expectsMandatoryStage: boolean,
): string {
  if (status === "sans_stage") {
    return expectsMandatoryStage ? "Sans stage" : "Aucun stage";
  }
  if (status === "en_cours") return "En cours";
  if (status === "valide") return "Stage validé";
  return "Plusieurs stages";
}

function statusStyle(
  status: StageRosterStudentStatus,
  expectsMandatoryStage: boolean,
): string {
  if (status === "sans_stage") {
    return expectsMandatoryStage
      ? "bg-rose-50 text-rose-800 border-rose-200"
      : "bg-stone-50 text-stone-600 border-stone-200";
  }
  if (status === "en_cours") return "bg-amber-50 text-amber-900 border-amber-200";
  if (status === "valide") return "bg-emerald-50 text-emerald-900 border-emerald-200";
  return "bg-violet-50 text-violet-900 border-violet-200";
}

type TeacherOption = {
  externalUserId: string;
  email: string;
  displayName: string;
};

type RosterResponse = {
  schoolYear: string;
  availableClasses: string[];
  referents: Array<{ name: string; email: string; role?: string }>;
  canAssignReferent?: boolean;
  teachers?: TeacherOption[];
  roster: StageClassRoster | null;
  message?: string;
};

export default function StageClassRosterPanel({
  onOpenConvention,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
}: {
  onOpenConvention: (conventionId: string) => void;
  canFileOneDrive?: boolean;
  oneDriveConnected?: boolean;
  onFileOneDrive?: (conventionId: string) => void;
  filingConventionId?: string | null;
}) {
  const [data, setData] = useState<RosterResponse | null>(null);
  const [selectedClass, setSelectedClass] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [assignBusyId, setAssignBusyId] = useState<string | null>(null);
  const [assignMsg, setAssignMsg] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<RosterStatusFilter>("all");

  const load = useCallback(async (className?: string) => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
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
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const onClassChange = (className: string) => {
    setSelectedClass(className);
    setQuery("");
    setStatusFilter("all");
    void load(className);
  };

  async function assignReferent(conventionId: string, teacherId: string) {
    if (!teacherId || !data?.teachers) return;
    const teacher = data.teachers.find((t) => t.externalUserId === teacherId);
    if (!teacher) return;
    setAssignBusyId(conventionId);
    setAssignMsg(null);
    setError(null);
    try {
      const res = await fetch(`/api/stages/conventions/${conventionId}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          action: "assign_referent",
          externalUserId: teacher.externalUserId,
          name: teacher.displayName,
          email: teacher.email,
        }),
      });
      const json = await res.json();
      if (!res.ok) throw new Error(json.error || "Erreur délégation");
      setAssignMsg(`Référent stage : ${teacher.displayName}`);
      await load(selectedClass);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setAssignBusyId(null);
    }
  }

  const roster = data?.roster ?? null;

  const filteredStudents = useMemo(() => {
    if (!roster) return [];
    const q = normalizeSearch(query);
    return roster.students.filter((student) => {
      if (statusFilter !== "all" && student.rosterStatus !== statusFilter) return false;
      if (!q) return true;
      const blob = normalizeSearch(
        [
          student.prenom,
          student.nom,
          student.ine,
          ...student.conventions.flatMap((c) => [c.companyName, c.stageLabel, c.statusLabel]),
        ]
          .filter(Boolean)
          .join(" "),
      );
      return blob.includes(q);
    });
  }, [roster, query, statusFilter]);

  if (loading && !data) {
    return <p className="text-sm text-stone-500">Chargement du suivi classe…</p>;
  }

  if (error && !data) {
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

  if (!data || !roster) return null;

  const mandatory = roster.expectsMandatoryStage === true;
  const sansStageLabel = mandatory ? "Sans stage" : "Aucun";
  const sansStageColor = mandatory ? "text-rose-700" : "text-stone-600";
  const canAssign = data.canAssignReferent === true;
  const teachers = data.teachers ?? [];

  const statusFilters: Array<{ id: RosterStatusFilter; label: string; count: number }> = [
    { id: "all", label: "Tous", count: roster.summary.total },
    { id: "valide", label: "Validés", count: roster.summary.valide },
    { id: "en_cours", label: "En cours", count: roster.summary.enCours },
    { id: "sans_stage", label: sansStageLabel, count: roster.summary.sansStage },
    { id: "plusieurs", label: "Plusieurs", count: roster.summary.plusieurs },
  ];

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
        {data.referents.length > 0 && (
          <p className="text-xs text-stone-500">
            {data.referents
              .map((r) =>
                r.role === "professeur_principal"
                  ? `PP ${r.name}`
                  : `Réf. ${r.name}`,
              )
              .join(" · ")}
          </p>
        )}
      </div>

      <div className="flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-end">
        <label className="block min-w-[220px] flex-1 text-sm font-semibold text-stone-700">
          Rechercher un élève
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Nom, INE, entreprise…"
            className="mt-1 w-full rounded-lg border border-stone-300 px-3 py-2 text-sm font-normal"
          />
        </label>
        <div className="flex flex-wrap gap-2">
          {statusFilters.map((f) => {
            const active = statusFilter === f.id;
            return (
              <button
                key={f.id}
                type="button"
                onClick={() => setStatusFilter(f.id)}
                className={`rounded-lg border px-3 py-1.5 text-xs font-semibold transition ${
                  active
                    ? "border-[#2F6B4A] bg-[#2F6B4A] text-white"
                    : "border-stone-200 bg-white text-stone-700 hover:border-[#2F6B4A]/40"
                }`}
              >
                {f.label} ({f.count})
              </button>
            );
          })}
        </div>
      </div>

      {canAssign && (
        <p className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
          En tant que professeur principal, vous pouvez déléguer un <strong>référent stage</strong>{" "}
          par dossier élève (sans accès aux réglages).
        </p>
      )}

      {assignMsg && (
        <p className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
          {assignMsg}
        </p>
      )}
      {error && (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-800">
          {error}
        </p>
      )}

      {mandatory && roster.officialPeriods.length > 0 && (
        <div className="rounded-xl border border-sky-200 bg-sky-50 px-4 py-3 text-sm text-sky-950">
          <p className="text-xs font-bold uppercase tracking-wide text-sky-800">
            Périodes officielles (rappel)
          </p>
          <ul className="mt-2 space-y-1 text-xs">
            {roster.officialPeriods.map((p) => (
              <li key={p.id}>
                <strong>{p.label}</strong> : {p.periodStart} → {p.periodEnd}
              </li>
            ))}
          </ul>
        </div>
      )}

      {roster.note && (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
          {roster.note}
        </p>
      )}

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
        {[
          ["all", "Élèves", roster.summary.total, "text-[#1F3D2B]"] as const,
          ["sans_stage", sansStageLabel, roster.summary.sansStage, sansStageColor] as const,
          ["en_cours", "En cours", roster.summary.enCours, "text-amber-800"] as const,
          ["valide", "Validés", roster.summary.valide, "text-emerald-800"] as const,
          ["plusieurs", "Plusieurs", roster.summary.plusieurs, "text-violet-800"] as const,
        ].map(([id, label, n, color]) => (
          <button
            key={id}
            type="button"
            onClick={() => setStatusFilter(id)}
            className={`rounded-xl border bg-white p-4 text-left shadow-sm transition ${
              statusFilter === id
                ? "border-[#2F6B4A] ring-1 ring-[#2F6B4A]/20"
                : "border-stone-200 hover:border-[#2F6B4A]/40"
            }`}
          >
            <p className="text-xs text-stone-500">{label}</p>
            <p className={`text-2xl font-black mt-1 ${color}`}>{n}</p>
          </button>
        ))}
      </div>

      <div className="overflow-x-auto rounded-2xl border border-stone-200 bg-white shadow-sm">
        <table className="min-w-full text-sm">
          <thead>
            <tr className="border-b border-stone-100 bg-stone-50/80 text-left text-xs font-bold uppercase tracking-wide text-stone-500">
              <th className="px-4 py-3">Élève</th>
              <th className="px-4 py-3">Statut</th>
              <th className="px-4 py-3">Entreprise / période</th>
              <th className="px-4 py-3">Référent stage</th>
              <th className="px-4 py-3">Signatures</th>
              <th className="px-4 py-3">Dossier</th>
              {canFileOneDrive ? <th className="px-4 py-3">OneDrive</th> : null}
            </tr>
          </thead>
          <tbody className="divide-y divide-stone-100">
            {filteredStudents.map((student) => (
              <tr key={student.key} className="hover:bg-stone-50/50">
                <td className="px-4 py-3 font-semibold text-[#1F3D2B]">
                  {student.prenom} {student.nom}
                  {student.ine ? (
                    <span className="ml-1 text-xs font-normal text-stone-400">({student.ine})</span>
                  ) : null}
                </td>
                <td className="px-4 py-3">
                  <span
                    className={`inline-flex rounded-full border px-2.5 py-0.5 text-xs font-semibold ${statusStyle(student.rosterStatus, mandatory)}`}
                  >
                    {statusLabel(student.rosterStatus, mandatory)}
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
                <td className="px-4 py-3 text-xs text-stone-600 min-w-[180px]">
                  {student.conventions.length === 0 ? (
                    <span className="text-stone-400">—</span>
                  ) : (
                    <ul className="space-y-2">
                      {student.conventions.map((c) => (
                        <li key={c.id} className="space-y-1">
                          <p>
                            {c.teacherReferentName ? (
                              <>
                                {c.teacherReferentName}
                                {c.teacherReferentEmail ? (
                                  <span className="text-stone-400"> · {c.teacherReferentEmail}</span>
                                ) : null}
                              </>
                            ) : (
                              <span className="italic text-stone-400">Non assigné</span>
                            )}
                          </p>
                          {canAssign && teachers.length > 0 ? (
                            <select
                              className="w-full rounded border border-stone-300 px-2 py-1 text-xs"
                              disabled={assignBusyId === c.id}
                              defaultValue=""
                              onChange={(e) => {
                                const id = e.target.value;
                                if (id) void assignReferent(c.id, id);
                                e.target.value = "";
                              }}
                            >
                              <option value="">
                                {assignBusyId === c.id ? "Enregistrement…" : "Déléguer un référent…"}
                              </option>
                              {teachers.map((t) => (
                                <option key={t.externalUserId} value={t.externalUserId}>
                                  {t.displayName}
                                </option>
                              ))}
                            </select>
                          ) : null}
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
        {filteredStudents.length === 0 && (
          <p className="px-4 py-8 text-center text-sm text-stone-500">
            {roster.students.length === 0
              ? "Aucun élève pour cette classe."
              : "Aucun élève ne correspond à cette recherche ou à ce filtre. Astuce : un stage terminé apparaît sous « Validés », pas sous « En cours »."}
          </p>
        )}
      </div>
    </div>
  );
}
