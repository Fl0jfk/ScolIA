"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
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

function formatIsoDateFr(iso: string): string {
  const raw = iso.trim().slice(0, 10);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(raw)) return iso;
  return new Date(`${raw}T12:00:00`).toLocaleDateString("fr-FR");
}

function statusLabel(
  status: StageRosterStudentStatus,
  expectsMandatoryStage: boolean,
): string {
  if (status === "sans_stage") {
    return expectsMandatoryStage ? "Sans stage" : "Aucun stage";
  }
  if (status === "en_cours") return "En cours";
  if (status === "valide") return "Validé";
  return "Plusieurs";
}

function statusChipClass(
  status: StageRosterStudentStatus,
  expectsMandatoryStage: boolean,
): string {
  if (status === "sans_stage") {
    return expectsMandatoryStage
      ? "bg-rose-50 text-rose-800 ring-rose-200"
      : "bg-stone-100 text-stone-600 ring-stone-200";
  }
  if (status === "en_cours") return "bg-amber-50 text-amber-900 ring-amber-200";
  if (status === "valide") return "bg-emerald-50 text-emerald-900 ring-emerald-200";
  return "bg-violet-50 text-violet-900 ring-violet-200";
}

function studentInitials(prenom: string, nom: string): string {
  const a = prenom.trim().charAt(0);
  const b = nom.trim().charAt(0);
  return `${a}${b}`.toUpperCase() || "?";
}

function StudentAvatar({
  prenom,
  nom,
  photoUrl,
  size = "md",
}: {
  prenom: string;
  nom: string;
  photoUrl?: string | null;
  size?: "md" | "lg";
}) {
  const [failed, setFailed] = useState(false);
  const dim = size === "lg" ? "h-14 w-14 text-base" : "h-11 w-11 text-sm";
  const initials = studentInitials(prenom, nom);

  if (photoUrl && !failed) {
    return (
      <img
        src={photoUrl}
        alt=""
        onError={() => setFailed(true)}
        className={`${dim} shrink-0 rounded-2xl object-cover ring-2 ring-white shadow-sm`}
      />
    );
  }

  return (
    <div
      className={`${dim} flex shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-[#2F6B4A] to-[#1F3D2B] font-bold text-white shadow-sm ring-2 ring-white`}
      aria-hidden
    >
      {initials}
    </div>
  );
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
  selectedConventionId,
  focusClassName,
  detailSlot,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
}: {
  onOpenConvention: (conventionId: string) => void;
  selectedConventionId?: string | null;
  focusClassName?: string | null;
  detailSlot?: ReactNode;
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
  const [expandedKey, setExpandedKey] = useState<string | null>(null);
  const detailAnchorRef = useRef<HTMLDivElement | null>(null);

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
      else if (className) setSelectedClass(className);
      else if (json.availableClasses[0]) setSelectedClass(json.availableClasses[0]);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const wanted = focusClassName?.trim() || "";
    if (wanted) {
      void load(wanted);
      return;
    }
    void load();
  }, [load, focusClassName]);

  useEffect(() => {
    if (!selectedConventionId || !data?.roster) return;
    const match = data.roster.students.find((s) =>
      s.conventions.some((c) => c.id === selectedConventionId),
    );
    if (match) setExpandedKey(match.key);
  }, [selectedConventionId, data]);

  useEffect(() => {
    if (!selectedConventionId || !detailSlot) return;
    const t = window.setTimeout(() => {
      detailAnchorRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }, 80);
    return () => window.clearTimeout(t);
  }, [selectedConventionId, detailSlot, expandedKey]);

  const onClassChange = (className: string) => {
    setSelectedClass(className);
    setQuery("");
    setStatusFilter("all");
    setExpandedKey(null);
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

  const classOptions = useMemo(() => {
    const list = [...(data?.availableClasses ?? [])];
    const focus = focusClassName?.trim();
    if (focus && !list.some((c) => c.localeCompare(focus, "fr", { sensitivity: "base" }) === 0)) {
      list.push(focus);
      list.sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" }));
    }
    return list;
  }, [data?.availableClasses, focusClassName]);

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
  const canAssign = data.canAssignReferent === true;
  const teachers = data.teachers ?? [];

  const statusFilters: Array<{ id: RosterStatusFilter; label: string; count: number; tone: string }> =
    [
      { id: "all", label: "Tous", count: roster.summary.total, tone: "text-[#1F3D2B]" },
      { id: "valide", label: "Validés", count: roster.summary.valide, tone: "text-emerald-800" },
      { id: "en_cours", label: "En cours", count: roster.summary.enCours, tone: "text-amber-800" },
      {
        id: "sans_stage",
        label: sansStageLabel,
        count: roster.summary.sansStage,
        tone: mandatory ? "text-rose-700" : "text-stone-600",
      },
      {
        id: "plusieurs",
        label: "Plusieurs",
        count: roster.summary.plusieurs,
        tone: "text-violet-800",
      },
    ];

  return (
    <div className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="flex flex-wrap items-end gap-3">
          {classOptions.length >= 1 ? (
            <label className="text-sm font-semibold text-stone-700">
              Classe
              <select
                className="mt-1 block min-w-[140px] rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm shadow-sm"
                value={selectedClass}
                onChange={(e) => onClassChange(e.target.value)}
              >
                {classOptions.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <p className="text-lg font-bold text-[#1F3D2B]">Classe {roster.className}</p>
          )}
          {data.referents.length > 0 ? (
            <p className="pb-2 text-xs text-stone-500">
              {data.referents
                .map((r) =>
                  r.role === "professeur_principal" ? `PP ${r.name}` : `Réf. ${r.name}`,
                )
                .join(" · ")}
            </p>
          ) : null}
        </div>
        <label className="block min-w-[200px] flex-1 text-sm font-semibold text-stone-700 sm:max-w-xs">
          Rechercher
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Nom, entreprise…"
            className="mt-1 w-full rounded-xl border border-stone-200 bg-white px-3 py-2 text-sm font-normal shadow-sm"
          />
        </label>
      </div>

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5">
        {statusFilters.map((f) => {
          const active = statusFilter === f.id;
          return (
            <button
              key={f.id}
              type="button"
              onClick={() => setStatusFilter(f.id)}
              className={`rounded-2xl border px-3 py-3 text-left transition ${
                active
                  ? "border-[#2F6B4A] bg-[#2F6B4A]/08 ring-1 ring-[#2F6B4A]/25"
                  : "border-stone-200 bg-white hover:border-[#2F6B4A]/35"
              }`}
            >
              <p className="text-[11px] font-medium text-stone-500">{f.label}</p>
              <p className={`mt-0.5 text-xl font-black tabular-nums ${f.tone}`}>{f.count}</p>
            </button>
          );
        })}
      </div>

      {canAssign ? (
        <p className="rounded-xl border border-emerald-200/80 bg-emerald-50/80 px-3 py-2 text-xs text-emerald-900">
          Professeur principal : vous pouvez déléguer un <strong>référent stage</strong> par dossier.
        </p>
      ) : null}

      {assignMsg ? (
        <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
          {assignMsg}
        </p>
      ) : null}
      {error ? (
        <p className="rounded-xl border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-800">
          {error}
        </p>
      ) : null}

      {mandatory && roster.officialPeriods.length > 0 ? (
        <div className="rounded-2xl border border-sky-100 bg-gradient-to-r from-sky-50 to-white px-4 py-3 text-sm text-sky-950">
          <p className="text-[11px] font-bold uppercase tracking-wide text-sky-700">
            Périodes officielles
          </p>
          <ul className="mt-1.5 flex flex-wrap gap-x-4 gap-y-1 text-xs">
            {roster.officialPeriods.map((p) => (
              <li key={p.id}>
                <strong>{p.label}</strong> · {formatIsoDateFr(p.periodStart)} →{" "}
                {formatIsoDateFr(p.periodEnd)}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {roster.note ? (
        <p className="rounded-xl border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
          {roster.note}
        </p>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
        {filteredStudents.map((student) => {
          const open = expandedKey === student.key;
          const mainConvention = student.conventions[0];
          return (
            <div
              key={student.key}
              className={`overflow-hidden rounded-3xl border bg-white shadow-sm transition ${
                open
                  ? "border-[#2F6B4A]/45 ring-2 ring-[#2F6B4A]/15 sm:col-span-2 xl:col-span-3"
                  : "border-stone-200/90 hover:border-[#2F6B4A]/30 hover:shadow-md"
              }`}
            >
              <button
                type="button"
                onClick={() => setExpandedKey(open ? null : student.key)}
                className="flex w-full items-center gap-3 px-3.5 py-3 text-left"
              >
                <StudentAvatar
                  prenom={student.prenom}
                  nom={student.nom}
                  photoUrl={student.photoUrl}
                  size={open ? "lg" : "md"}
                />
                <div className="min-w-0 flex-1">
                  <p className="truncate font-bold text-[#1F3D2B]">
                    {student.prenom} {student.nom}
                  </p>
                  <p className="mt-0.5 truncate text-xs text-stone-500">
                    {mainConvention
                      ? mainConvention.companyName || mainConvention.statusLabel
                      : "Aucune convention"}
                    {mainConvention?.signatureSummary.total
                      ? ` · ${mainConvention.signatureSummary.signed}/${mainConvention.signatureSummary.total} sig.`
                      : ""}
                  </p>
                </div>
                <span
                  className={`shrink-0 rounded-full px-2.5 py-1 text-[11px] font-semibold ring-1 ring-inset ${statusChipClass(student.rosterStatus, mandatory)}`}
                >
                  {statusLabel(student.rosterStatus, mandatory)}
                </span>
              </button>

              {open ? (
                <div className="space-y-3 border-t border-stone-100 bg-gradient-to-b from-[#f6faf8] to-white px-3.5 py-4">
                  {student.conventions.length === 0 ? (
                    <p className="text-sm text-stone-500">
                      Aucun stage déposé pour cet élève pour le moment.
                    </p>
                  ) : (
                    student.conventions.map((c) => {
                      const selected = selectedConventionId === c.id;
                      return (
                        <div
                          key={c.id}
                          className={`rounded-2xl border bg-white p-4 space-y-3 shadow-sm ${
                            selected
                              ? "border-[#2F6B4A] ring-1 ring-[#2F6B4A]/20"
                              : "border-stone-200"
                          }`}
                        >
                          <div className="flex flex-wrap items-start justify-between gap-2">
                            <div className="min-w-0">
                              <p className="text-sm font-bold text-[#1F3D2B]">
                                {c.stageLabel ? `${c.stageLabel} — ` : ""}
                                {c.companyName}
                              </p>
                              <p className="mt-0.5 text-xs text-stone-500">
                                {formatIsoDateFr(c.periodStart)} → {formatIsoDateFr(c.periodEnd)} ·{" "}
                                {c.statusLabel}
                              </p>
                              <p className="mt-1 text-xs text-stone-600">
                                Référent :{" "}
                                {c.teacherReferentName ? (
                                  <span className="font-medium">{c.teacherReferentName}</span>
                                ) : (
                                  <span className="italic text-stone-400">Non assigné</span>
                                )}
                              </p>
                            </div>
                            <button
                              type="button"
                              onClick={() => onOpenConvention(c.id)}
                              className="shrink-0 rounded-xl bg-[#2F6B4A] px-3 py-1.5 text-xs font-bold text-white shadow-sm hover:bg-[#275a3e]"
                            >
                              {selected && detailSlot ? "Dossier ouvert" : "Ouvrir le dossier"}
                            </button>
                          </div>

                          <StageSignatureProgress summary={c.signatureSummary} compact />

                          <div className="flex flex-wrap items-center gap-3">
                            {canAssign && teachers.length > 0 ? (
                              <select
                                className="rounded-lg border border-stone-300 px-2 py-1 text-xs"
                                disabled={assignBusyId === c.id}
                                defaultValue=""
                                onChange={(e) => {
                                  const id = e.target.value;
                                  if (id) void assignReferent(c.id, id);
                                  e.target.value = "";
                                }}
                              >
                                <option value="">
                                  {assignBusyId === c.id
                                    ? "Enregistrement…"
                                    : "Déléguer un référent…"}
                                </option>
                                {teachers.map((t) => (
                                  <option key={t.externalUserId} value={t.externalUserId}>
                                    {t.displayName}
                                  </option>
                                ))}
                              </select>
                            ) : null}

                            {canFileOneDrive ? (
                              c.oneDriveFiled ? (
                                <span className="text-xs font-semibold text-emerald-700">
                                  OneDrive : déposé
                                </span>
                              ) : c.canFileOneDrive ? (
                                <button
                                  type="button"
                                  disabled={!oneDriveConnected || filingConventionId === c.id}
                                  onClick={() => onFileOneDrive?.(c.id)}
                                  className="text-xs font-semibold text-[#2F6B4A] underline disabled:opacity-50"
                                >
                                  {filingConventionId === c.id ? "Envoi…" : "→ OneDrive"}
                                </button>
                              ) : null
                            ) : null}
                          </div>

                          {selected && detailSlot ? (
                            <div ref={detailAnchorRef} className="border-t border-stone-100 pt-3">
                              {detailSlot}
                            </div>
                          ) : null}
                        </div>
                      );
                    })
                  )}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>

      {filteredStudents.length === 0 ? (
        <p className="rounded-2xl border border-stone-200 bg-stone-50 px-4 py-10 text-center text-sm text-stone-500">
          {roster.students.length === 0
            ? "Aucun élève pour cette classe."
            : "Aucun élève ne correspond à cette recherche ou à ce filtre."}
        </p>
      ) : null}
    </div>
  );
}
