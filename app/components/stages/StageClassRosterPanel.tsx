"use client";

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { StageClassRoster, StageGlobalSearchHit, StageRosterStudentStatus } from "@/app/lib/stage-class-roster";
import StageSignatureProgress from "@/app/components/stages/StageSignatureProgress";

type RosterStatusFilter = "all" | StageRosterStudentStatus;

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
}: {
  prenom: string;
  nom: string;
  photoUrl?: string | null;
}) {
  const [failed, setFailed] = useState(false);
  const initials = studentInitials(prenom, nom);

  if (photoUrl && !failed) {
    return (
      <img
        src={photoUrl}
        alt=""
        onError={() => setFailed(true)}
        className="h-9 w-9 shrink-0 rounded-full object-cover ring-1 ring-stone-200"
      />
    );
  }

  return (
    <div
      className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-[#2F6B4A] text-xs font-bold text-white"
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

/** Cache module : survit au changement d’onglet (démontage du panel). */
const ROSTER_MEMORY_TTL_MS = 90_000;
const rosterMemoryCache = new Map<string, { at: number; data: RosterResponse }>();

function readRosterMemory(key: string): RosterResponse | undefined {
  const hit = rosterMemoryCache.get(key);
  if (!hit) return undefined;
  if (Date.now() - hit.at > ROSTER_MEMORY_TTL_MS) {
    rosterMemoryCache.delete(key);
    return undefined;
  }
  return hit.data;
}

function writeRosterMemory(key: string, data: RosterResponse): void {
  rosterMemoryCache.set(key, { at: Date.now(), data });
}

function clearRosterMemory(): void {
  rosterMemoryCache.clear();
}

export default function StageClassRosterPanel({
  onOpenConvention,
  selectedConventionId,
  focusClassName,
  detailSlot,
  canFileOneDrive,
  oneDriveConnected,
  onFileOneDrive,
  filingConventionId,
  canCreateOffline,
  onCreateOffline,
  refreshToken,
}: {
  onOpenConvention: (conventionId: string) => void;
  selectedConventionId?: string | null;
  focusClassName?: string | null;
  detailSlot?: ReactNode;
  canFileOneDrive?: boolean;
  oneDriveConnected?: boolean;
  onFileOneDrive?: (conventionId: string) => void;
  filingConventionId?: string | null;
  canCreateOffline?: boolean;
  onCreateOffline?: (preset: {
    firstName: string;
    lastName: string;
    className: string;
    ine?: string;
  }) => void;
  refreshToken?: number;
}) {
  const [data, setData] = useState<RosterResponse | null>(null);
  const [selectedClass, setSelectedClass] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [assignBusyId, setAssignBusyId] = useState<string | null>(null);
  const [assignMsg, setAssignMsg] = useState<string | null>(null);
  const [globalQuery, setGlobalQuery] = useState("");
  const [globalResults, setGlobalResults] = useState<StageGlobalSearchHit[]>([]);
  const [globalSearching, setGlobalSearching] = useState(false);
  const [statusFilter, setStatusFilter] = useState<RosterStatusFilter>("all");
  const [selectedStudentKey, setSelectedStudentKey] = useState<string | null>(null);
  const detailAnchorRef = useRef<HTMLDivElement | null>(null);
  const loadSeqRef = useRef(0);
  const availableClassesRef = useRef<string[]>([]);
  const prefetchDoneRef = useRef<Set<string>>(new Set());

  const cacheKeyFor = (className: string) => className.trim().toLowerCase() || "__default__";

  const load = useCallback(async (className?: string, opts?: { force?: boolean; silent?: boolean }) => {
    const wanted = className?.trim() || "";
    const cacheKey = cacheKeyFor(wanted);
    const cached = !opts?.force ? readRosterMemory(cacheKey) : undefined;
    if (cached) {
      if (!opts?.silent) {
        setData(cached);
        if (cached.roster?.className) setSelectedClass(cached.roster.className);
        else if (wanted) setSelectedClass(wanted);
        setLoading(false);
        setError(null);
      }
    } else if (!opts?.silent) {
      setLoading(true);
      setError(null);
    }

    // Prefetch silencieux : ne pas incrémenter le seq (sinon on annule le chargement visible).
    const seq = opts?.silent ? loadSeqRef.current : ++loadSeqRef.current;
    const t0 = performance.now();
    try {
      const params = new URLSearchParams();
      if (wanted) params.set("className", wanted);
      const res = await fetch(`/api/stages/class-roster?${params}`, { cache: "no-store" });
      const json = (await res.json()) as RosterResponse & {
        error?: string;
        perf?: unknown;
        cache?: unknown;
      };
      if (!res.ok) throw new Error(json.error || "Erreur chargement");
      if (!opts?.silent && seq !== loadSeqRef.current) return;

      const resolvedClass = json.roster?.className || wanted || json.availableClasses[0] || "";
      writeRosterMemory(cacheKeyFor(resolvedClass), json);
      if (!wanted) writeRosterMemory("__default__", json);
      if (json.availableClasses?.length) {
        availableClassesRef.current = json.availableClasses;
      }

      if (!opts?.silent) {
        setData(json);
        if (json.roster?.className) setSelectedClass(json.roster.className);
        else if (wanted) setSelectedClass(wanted);
        else if (json.availableClasses[0]) setSelectedClass(json.availableClasses[0]);
      }

      console.info("[ScolIA][stages/roster]", {
        className: resolvedClass || wanted || "(défaut)",
        fromMemoryCache: Boolean(cached),
        silent: Boolean(opts?.silent),
        clientMs: Math.round(performance.now() - t0),
        server: json.perf ?? null,
        valkey: json.cache ?? null,
      });
    } catch (e: unknown) {
      if (!opts?.silent && seq !== loadSeqRef.current) return;
      if (!cached && !opts?.silent) setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      if (!opts?.silent && seq === loadSeqRef.current) setLoading(false);
    }
  }, []);

  /** Prefetch des classes voisines en arrière-plan (Valkey + cache mémoire). */
  const prefetchNeighbors = useCallback(
    (currentClass: string, classes: string[]) => {
      if (!classes.length) return;
      const idx = classes.findIndex(
        (c) => c.localeCompare(currentClass, "fr", { sensitivity: "base" }) === 0,
      );
      const neighbors = [classes[idx - 1], classes[idx + 1], classes[idx + 2]].filter(
        (c): c is string => Boolean(c?.trim()),
      );
      for (const cls of neighbors) {
        const key = cacheKeyFor(cls);
        if (readRosterMemory(key) || prefetchDoneRef.current.has(key)) continue;
        prefetchDoneRef.current.add(key);
        void load(cls, { silent: true });
      }
    },
    [load],
  );

  // Premier chargement / focus classe : utiliser le cache si possible (pas de wipe).
  useEffect(() => {
    const wanted = focusClassName?.trim() || "";
    void load(wanted || undefined).then(() => {
      const classes = availableClassesRef.current;
      const current = wanted || selectedClass || classes[0] || "";
      if (current) prefetchNeighbors(current, classes);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps -- focusClassName only
  }, [load, focusClassName, prefetchNeighbors]);

  // Refresh explicite (après mutation) : invalider le cache mémoire.
  useEffect(() => {
    if (refreshToken == null || refreshToken === 0) return;
    clearRosterMemory();
    prefetchDoneRef.current.clear();
    void load(selectedClass || focusClassName || undefined, { force: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [refreshToken]);

  useEffect(() => {
    if (!selectedConventionId || !data?.roster) return;
    const match = data.roster.students.find((s) =>
      s.conventions.some((c) => c.id === selectedConventionId),
    );
    if (match) setSelectedStudentKey(match.key);
  }, [selectedConventionId, data]);

  useEffect(() => {
    if (!selectedConventionId || !detailSlot) return;
    const t = window.setTimeout(() => {
      detailAnchorRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" });
    }, 80);
    return () => window.clearTimeout(t);
  }, [selectedConventionId, detailSlot, selectedStudentKey]);

  useEffect(() => {
    const q = globalQuery.trim();
    if (q.length < 2) {
      setGlobalResults([]);
      setGlobalSearching(false);
      return;
    }
    let cancelled = false;
    setGlobalSearching(true);
    const timer = window.setTimeout(() => {
      void (async () => {
        try {
          const params = new URLSearchParams({ q });
          const res = await fetch(`/api/stages/class-roster?${params}`, { cache: "no-store" });
          const json = (await res.json()) as {
            globalResults?: StageGlobalSearchHit[];
            error?: string;
          };
          if (cancelled) return;
          if (!res.ok) throw new Error(json.error || "Recherche impossible");
          setGlobalResults(json.globalResults ?? []);
        } catch (e: unknown) {
          if (!cancelled) {
            setGlobalResults([]);
            setError(e instanceof Error ? e.message : "Erreur recherche");
          }
        } finally {
          if (!cancelled) setGlobalSearching(false);
        }
      })();
    }, 280);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [globalQuery]);

  const onClassChange = (className: string) => {
    setSelectedClass(className);
    setStatusFilter("all");
    setSelectedStudentKey(null);
    void load(className).then(() => {
      prefetchNeighbors(className, availableClassesRef.current);
    });
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
      clearRosterMemory();
      await load(selectedClass, { force: true });
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
    return roster.students.filter((student) => {
      if (statusFilter !== "all" && student.rosterStatus !== statusFilter) return false;
      return true;
    });
  }, [roster, statusFilter]);

  const selectedStudent = useMemo(() => {
    if (!roster || !selectedStudentKey) return null;
    return roster.students.find((s) => s.key === selectedStudentKey) ?? null;
  }, [roster, selectedStudentKey]);

  const globalSearchBlock = (
    <label className="block text-sm font-medium text-stone-700">
      Recherche
      <input
        type="search"
        value={globalQuery}
        onChange={(e) => setGlobalQuery(e.target.value)}
        placeholder="Nom, entreprise, classe…"
        className="mt-1 w-full max-w-md rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm font-normal shadow-sm"
      />
    </label>
  );

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
      <div className="space-y-4">
        {globalSearchBlock}
        <div className="rounded-lg border border-amber-200 bg-amber-50 px-4 py-3 text-sm text-amber-900">
          {data.message}
        </div>
      </div>
    );
  }

  if (!data || !roster) {
    return <div className="space-y-4">{globalSearchBlock}</div>;
  }

  const mandatory = roster.expectsMandatoryStage === true;
  const sansStageLabel = mandatory ? "Sans stage" : "Aucun";
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
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-4">
        {globalSearchBlock}
        {classOptions.length >= 1 ? (
          <label className="text-sm font-medium text-stone-700">
            Classe
            <select
              className="mt-1 block min-w-[140px] rounded-lg border border-stone-300 bg-white px-3 py-2 text-sm shadow-sm"
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
          <p className="pb-2 text-base font-bold text-[#1F3D2B]">Classe {roster.className}</p>
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
        {loading ? (
          <p className="pb-2 text-xs text-stone-400">Mise à jour…</p>
        ) : null}
      </div>

      {globalQuery.trim().length >= 2 ? (
        <div className="rounded-lg border border-stone-200 bg-stone-50 px-3 py-2">
          <p className="text-xs font-semibold text-stone-600">
            Résultats{globalSearching ? "…" : ` · ${globalResults.length}`}
          </p>
          {globalSearching && globalResults.length === 0 ? (
            <p className="mt-1 text-sm text-stone-500">Recherche…</p>
          ) : globalResults.length === 0 ? (
            <p className="mt-1 text-sm text-stone-500">Aucun résultat.</p>
          ) : (
            <ul className="mt-1 divide-y divide-stone-200">
              {globalResults.map((hit) => (
                <li key={hit.conventionId} className="flex items-center gap-2 py-2">
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-semibold text-[#1F3D2B]">
                      {hit.studentFirstName} {hit.studentLastName}
                      <span className="ml-2 font-normal text-stone-500">{hit.className}</span>
                    </p>
                    <p className="truncate text-xs text-stone-600">
                      {hit.companyName}
                      {hit.stageLabel ? ` · ${hit.stageLabel}` : ""}
                      {" · "}
                      {hit.statusLabel}
                    </p>
                  </div>
                  <button
                    type="button"
                    onClick={() => onOpenConvention(hit.conventionId)}
                    className="shrink-0 rounded-lg bg-[#2F6B4A] px-2.5 py-1 text-xs font-bold text-white"
                  >
                    Ouvrir
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}

      <div className="flex flex-wrap gap-1.5">
        {statusFilters.map((f) => {
          const active = statusFilter === f.id;
          return (
            <button
              key={f.id}
              type="button"
              onClick={() => setStatusFilter(f.id)}
              className={`rounded-full px-3 py-1 text-xs font-semibold transition ${
                active
                  ? "bg-[#2F6B4A] text-white"
                  : "bg-stone-100 text-stone-700 hover:bg-stone-200"
              }`}
            >
              {f.label}
              <span className={`ml-1 tabular-nums ${active ? "text-white/80" : "text-stone-500"}`}>
                {f.count}
              </span>
            </button>
          );
        })}
      </div>

      {canAssign ? (
        <p className="text-xs text-stone-500">
          Professeur principal : vous pouvez déléguer un référent stage par dossier.
        </p>
      ) : null}
      {assignMsg ? (
        <p className="rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-xs text-emerald-900">
          {assignMsg}
        </p>
      ) : null}
      {error ? (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-xs text-rose-800">
          {error}
        </p>
      ) : null}

      {mandatory && roster.officialPeriods.length > 0 ? (
        <p className="text-xs text-stone-600">
          <span className="font-semibold text-sky-800">Périodes : </span>
          {roster.officialPeriods
            .map((p) => `${p.label} (${formatIsoDateFr(p.periodStart)} → ${formatIsoDateFr(p.periodEnd)})`)
            .join(" · ")}
        </p>
      ) : null}

      {roster.note ? (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-900">
          {roster.note}
        </p>
      ) : null}

      {/* Liste élèves — plate, sans cartes imbriquées */}
      <div className="overflow-hidden rounded-xl border border-stone-200 bg-white">
        <ul className="divide-y divide-stone-100">
          {filteredStudents.map((student) => {
            const selected = selectedStudentKey === student.key;
            const mainConvention = student.conventions[0];
            const hasSelectedConv = student.conventions.some(
              (c) => c.id === selectedConventionId,
            );
            return (
              <li key={student.key}>
                <button
                  type="button"
                  onClick={() => {
                    setSelectedStudentKey(selected ? null : student.key);
                    if (!selected && student.conventions.length === 1) {
                      onOpenConvention(student.conventions[0]!.id);
                    }
                  }}
                  className={`flex w-full items-center gap-3 px-3 py-2.5 text-left transition ${
                    selected || hasSelectedConv
                      ? "bg-[#2F6B4A]/06"
                      : "hover:bg-stone-50"
                  }`}
                >
                  <StudentAvatar
                    prenom={student.prenom}
                    nom={student.nom}
                    photoUrl={student.photoUrl}
                  />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-semibold text-[#1F3D2B]">
                      {student.prenom} {student.nom}
                    </p>
                    <p className="truncate text-xs text-stone-500">
                      {mainConvention
                        ? `${mainConvention.companyName || mainConvention.statusLabel}${
                            mainConvention.signatureSummary.total
                              ? ` · ${mainConvention.signatureSummary.signed}/${mainConvention.signatureSummary.total} sig.`
                              : ""
                          }`
                        : "Aucune convention"}
                    </p>
                  </div>
                  <span
                    className={`shrink-0 rounded-full px-2 py-0.5 text-[11px] font-semibold ring-1 ring-inset ${statusChipClass(student.rosterStatus, mandatory)}`}
                  >
                    {statusLabel(student.rosterStatus, mandatory)}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
        {filteredStudents.length === 0 ? (
          <p className="px-4 py-8 text-center text-sm text-stone-500">
            {roster.students.length === 0
              ? "Aucun élève pour cette classe."
              : "Aucun élève pour ce filtre."}
          </p>
        ) : null}
      </div>

      {/* Fiche élève sélectionné — un seul panneau, pas de nesting */}
      {selectedStudent ? (
        <div className="rounded-xl border border-stone-200 bg-white p-4 space-y-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <h3 className="text-sm font-bold text-[#1F3D2B]">
              {selectedStudent.prenom} {selectedStudent.nom}
            </h3>
            {canCreateOffline && onCreateOffline ? (
              <button
                type="button"
                onClick={() =>
                  onCreateOffline({
                    firstName: selectedStudent.prenom,
                    lastName: selectedStudent.nom,
                    className: roster.className || selectedClass,
                    ine: selectedStudent.ine,
                  })
                }
                className="text-xs font-semibold text-[#2F6B4A] underline"
              >
                + Stage hors plateforme
              </button>
            ) : null}
          </div>

          {selectedStudent.conventions.length === 0 ? (
            <p className="text-sm text-stone-500">Aucun stage déposé pour cet élève.</p>
          ) : (
            <ul className="space-y-2">
              {selectedStudent.conventions.map((c) => {
                const active = selectedConventionId === c.id;
                return (
                  <li
                    key={c.id}
                    className={`rounded-lg border px-3 py-2.5 ${
                      active ? "border-[#2F6B4A] bg-[#2F6B4A]/05" : "border-stone-200"
                    }`}
                  >
                    <div className="flex flex-wrap items-start justify-between gap-2">
                      <div className="min-w-0">
                        <p className="text-sm font-semibold text-[#1F3D2B]">
                          {c.stageLabel ? `${c.stageLabel} — ` : ""}
                          {c.companyName}
                        </p>
                        <p className="mt-0.5 text-xs text-stone-500">
                          {formatIsoDateFr(c.periodStart)} → {formatIsoDateFr(c.periodEnd)} ·{" "}
                          {c.statusLabel}
                          {c.teacherReferentName
                            ? ` · Réf. ${c.teacherReferentName}`
                            : " · Référent non assigné"}
                        </p>
                      </div>
                      <button
                        type="button"
                        onClick={() => onOpenConvention(c.id)}
                        className="shrink-0 rounded-lg bg-[#2F6B4A] px-3 py-1.5 text-xs font-bold text-white hover:bg-[#275a3e]"
                      >
                        {active && detailSlot ? "Dossier ouvert" : "Ouvrir le dossier"}
                      </button>
                    </div>
                    <div className="mt-2">
                      <StageSignatureProgress summary={c.signatureSummary} compact />
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-3">
                      {canAssign && teachers.length > 0 ? (
                        <select
                          className="rounded-md border border-stone-300 px-2 py-1 text-xs"
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
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      ) : null}

      {/* Dossier détail — sous la liste, plus de nesting dans les cartes */}
      {detailSlot ? (
        <div
          ref={detailAnchorRef}
          className="rounded-xl border border-stone-200 bg-white p-4 shadow-sm"
        >
          {detailSlot}
        </div>
      ) : null}
    </div>
  );
}
