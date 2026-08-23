"use client";

import { useCallback, useEffect, useMemo, useState } from "react";

type DirectoryUser = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
};

type Assignment = {
  className: string;
  externalUserId: string;
  name: string;
  email: string;
};

function userLabel(u: DirectoryUser): string {
  return u.displayName || `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim() || u.email;
}

function currentSchoolYearLabel() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  if (m >= 8) return `${y}-${y + 1}`;
  return `${y - 1}-${y}`;
}

export default function StageReferentsEditor({
  initialYear,
  onSaved,
}: {
  initialYear?: string;
  onSaved?: (message: string) => void;
}) {
  const [schoolYear, setSchoolYear] = useState(initialYear || currentSchoolYearLabel());
  const [classes, setClasses] = useState<string[]>([]);
  const [assignments, setAssignments] = useState<Record<string, string>>({});
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [updatedAt, setUpdatedAt] = useState<string | null>(null);
  const [updatedBy, setUpdatedBy] = useState<string | null>(null);
  const [previousConfig, setPreviousConfig] = useState<{ schoolYear: string; assignments: Assignment[] } | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [customClass, setCustomClass] = useState("");

  const userById = useMemo(() => {
    const map = new Map<string, DirectoryUser>();
    for (const u of users) map.set(u.externalUserId, u);
    return map;
  }, [users]);

  const load = useCallback(async (year: string) => {
    setLoading(true);
    setError(null);
    try {
      const [refRes, usersRes] = await Promise.all([
        fetch(`/api/stages/referents?schoolYear=${encodeURIComponent(year)}`, { cache: "no-store" }),
        fetch("/api/stages/directory-users", { cache: "no-store" }),
      ]);
      const refData = await refRes.json();
      const usersData = await usersRes.json();
      if (!refRes.ok) throw new Error(refData?.error || "Erreur chargement référents");
      if (!usersRes.ok) throw new Error(usersData?.error || "Erreur chargement utilisateurs");

      setClasses(refData.classes || []);
      setUsers(usersData.users || []);
      setPreviousConfig(refData.previousConfig || null);
      setUpdatedAt(refData.config?.updatedAt || null);
      setUpdatedBy(refData.config?.updatedBy || null);

      const map: Record<string, string> = {};
      for (const a of (refData.config?.assignments || []) as Assignment[]) {
        map[a.className] = a.externalUserId;
      }
      setAssignments(map);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load(schoolYear);
  }, [schoolYear, load]);

  function setClassReferent(className: string, externalUserId: string) {
    setAssignments((prev) => {
      const next = { ...prev };
      if (!externalUserId) delete next[className];
      else next[className] = externalUserId;
      return next;
    });
  }

  function copyFromPreviousYear() {
    if (!previousConfig?.assignments.length) return;
    const map: Record<string, string> = {};
    for (const a of previousConfig.assignments) {
      if (userById.has(a.externalUserId)) map[a.className] = a.externalUserId;
    }
    setAssignments(map);
    onSaved?.(`Affectations copiées depuis ${previousConfig.schoolYear}.`);
  }

  function addCustomClass() {
    const name = customClass.trim();
    if (!name || classes.includes(name)) {
      setCustomClass("");
      return;
    }
    setClasses((prev) => [...prev, name].sort((a, b) => a.localeCompare(b, "fr", { sensitivity: "base" })));
    setCustomClass("");
  }

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const payload: Assignment[] = [];
      for (const className of classes) {
        const externalUserId = assignments[className];
        if (!externalUserId) continue;
        const u = userById.get(externalUserId);
        if (!u) continue;
        payload.push({
          className,
          externalUserId,
          name: userLabel(u),
          email: u.email,
        });
      }
      const res = await fetch("/api/stages/referents", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ schoolYear, assignments: payload }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setUpdatedAt(data.config?.updatedAt || null);
      setUpdatedBy(data.config?.updatedBy || null);
      onSaved?.(`Professeurs référents enregistrés pour ${schoolYear} (${payload.length} classe(s)).`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const assignedCount = Object.keys(assignments).filter((k) => assignments[k]).length;

  if (loading) {
    return <p className="text-sm text-stone-500">Chargement des référents…</p>;
  }

  return (
    <div className="space-y-4">
      {error && (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</p>
      )}

      <div className="flex flex-wrap items-end gap-3">
        <label className="block text-sm">
          Année scolaire
          <input
            className="mt-1 block w-40 rounded-lg border border-stone-300 px-3 py-2"
            value={schoolYear}
            onChange={(e) => setSchoolYear(e.target.value)}
            placeholder="2025-2026"
          />
        </label>
        {previousConfig && previousConfig.assignments.length > 0 && (
          <button
            type="button"
            onClick={copyFromPreviousYear}
            className="rounded-lg border border-stone-300 px-3 py-2 text-sm font-semibold text-stone-700 hover:bg-stone-50"
          >
            Reprendre {previousConfig.schoolYear}
          </button>
        )}
      </div>

      <p className="text-sm text-stone-600">
        Assignez un utilisateur (professeur) à chaque classe. Lors d&apos;une candidature ou préconvention,
        le référent est rempli automatiquement selon la classe de l&apos;élève.
      </p>

      {updatedAt && (
        <p className="text-xs text-stone-500">
          Dernière mise à jour : {new Date(updatedAt).toLocaleString("fr-FR")}
          {updatedBy ? ` par ${updatedBy}` : ""} — {assignedCount} classe(s) assignée(s)
        </p>
      )}

      {users.length === 0 ? (
        <p className="text-sm text-amber-800 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2">
          Aucun utilisateur avec le rôle professeur trouvé.
        </p>
      ) : (
        <div className="max-h-[420px] overflow-y-auto rounded-xl border border-stone-200 divide-y divide-stone-100">
          {classes.map((className) => (
            <div
              key={className}
              className="flex flex-wrap items-center gap-3 px-4 py-3 bg-white even:bg-stone-50/50"
            >
              <span className="w-16 shrink-0 font-bold text-[#1F3D2B]">{className}</span>
              <select
                className="min-w-[220px] flex-1 rounded-lg border border-stone-300 px-3 py-2 text-sm"
                value={assignments[className] || ""}
                onChange={(e) => setClassReferent(className, e.target.value)}
              >
                <option value="">— Non assigné —</option>
                {users.map((u) => (
                  <option key={u.externalUserId} value={u.externalUserId}>
                    {userLabel(u)} ({u.email})
                  </option>
                ))}
              </select>
            </div>
          ))}
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        <input
          className="rounded-lg border border-stone-300 px-3 py-2 text-sm"
          placeholder="Classe hors liste (ex. 3G)"
          value={customClass}
          onChange={(e) => setCustomClass(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addCustomClass();
            }
          }}
        />
        <button
          type="button"
          onClick={addCustomClass}
          className="rounded-lg border border-stone-300 px-3 py-2 text-sm font-semibold text-stone-700"
        >
          Ajouter une classe
        </button>
      </div>

      <button
        type="button"
        disabled={busy}
        onClick={() => void save()}
        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
      >
        {busy ? "Enregistrement…" : "Enregistrer les référents"}
      </button>
    </div>
  );
}
