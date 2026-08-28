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

export default function StageReferentsEditor({
  onSaved,
}: {
  onSaved?: (message: string) => void;
}) {
  const [classes, setClasses] = useState<string[]>([]);
  const [assignments, setAssignments] = useState<Record<string, string[]>>({});
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [updatedAt, setUpdatedAt] = useState<string | null>(null);
  const [updatedBy, setUpdatedBy] = useState<string | null>(null);
  const [previousConfig, setPreviousConfig] = useState<{ schoolYear: string; assignments: Assignment[] } | null>(
    null,
  );
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pickerByClass, setPickerByClass] = useState<Record<string, string>>({});

  const userById = useMemo(() => {
    const map = new Map<string, DirectoryUser>();
    for (const u of users) map.set(u.externalUserId, u);
    return map;
  }, [users]);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [refRes, usersRes] = await Promise.all([
        fetch("/api/stages/referents", { cache: "no-store" }),
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

      const map: Record<string, string[]> = {};
      for (const className of refData.classes || []) {
        map[className] = [];
      }
      for (const a of (refData.config?.assignments || []) as Assignment[]) {
        if (!map[a.className]) map[a.className] = [];
        if (!map[a.className]!.includes(a.externalUserId)) {
          map[a.className]!.push(a.externalUserId);
        }
      }
      setAssignments(map);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  function addReferent(className: string, externalUserId: string) {
    if (!externalUserId) return;
    setAssignments((prev) => {
      const current = prev[className] ?? [];
      if (current.includes(externalUserId)) return prev;
      return { ...prev, [className]: [...current, externalUserId] };
    });
    setPickerByClass((prev) => ({ ...prev, [className]: "" }));
  }

  function removeReferent(className: string, externalUserId: string) {
    setAssignments((prev) => ({
      ...prev,
      [className]: (prev[className] ?? []).filter((id) => id !== externalUserId),
    }));
  }

  function copyFromPreviousYear() {
    if (!previousConfig?.assignments.length) return;
    const map: Record<string, string[]> = {};
    for (const className of classes) map[className] = [];
    for (const a of previousConfig.assignments) {
      if (!classes.includes(a.className)) continue;
      if (!userById.has(a.externalUserId)) continue;
      if (!map[a.className]) map[a.className] = [];
      if (!map[a.className]!.includes(a.externalUserId)) {
        map[a.className]!.push(a.externalUserId);
      }
    }
    setAssignments(map);
    onSaved?.(`Affectations copiées depuis ${previousConfig.schoolYear}.`);
  }

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const payload: Assignment[] = [];
      for (const className of classes) {
        for (const externalUserId of assignments[className] ?? []) {
          const u = userById.get(externalUserId);
          if (!u) continue;
          payload.push({
            className,
            externalUserId,
            name: userLabel(u),
            email: u.email,
          });
        }
      }
      const res = await fetch("/api/stages/referents", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ assignments: payload }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setUpdatedAt(data.config?.updatedAt || null);
      setUpdatedBy(data.config?.updatedBy || null);
      onSaved?.(`Professeurs référents enregistrés (${payload.length} affectation(s)).`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const assignedCount = Object.values(assignments).reduce((n, ids) => n + ids.length, 0);

  if (loading) {
    return <p className="text-sm text-stone-500">Chargement des référents…</p>;
  }

  return (
    <div className="space-y-4">
      {error && (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">{error}</p>
      )}

      {previousConfig && previousConfig.assignments.length > 0 && (
        <button
          type="button"
          onClick={copyFromPreviousYear}
          className="rounded-lg border border-stone-300 px-3 py-2 text-sm font-semibold text-stone-700 hover:bg-stone-50"
        >
          Reprendre les affectations de {previousConfig.schoolYear}
        </button>
      )}

      <p className="text-sm text-stone-600">
        Assignez un ou plusieurs professeurs référents à chaque classe. Chaque référent voit
        l&apos;onglet <strong>Suivi classe</strong> pour ses classes. Les classes proviennent du
        référentiel établissement (dossiers élèves, Siècle, planning).
      </p>

      {updatedAt && (
        <p className="text-xs text-stone-500">
          Dernière mise à jour : {new Date(updatedAt).toLocaleString("fr-FR")}
          {updatedBy ? ` par ${updatedBy}` : ""} — {assignedCount} affectation(s)
        </p>
      )}

      {classes.length === 0 ? (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          Aucune classe trouvée. Vérifiez la liste élèves (dossiers) ou importez Structures.xml dans
          Paramètres → Pont Siècle.
        </p>
      ) : users.length === 0 ? (
        <p className="text-sm text-amber-800 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2">
          Aucun utilisateur avec le rôle professeur trouvé.
        </p>
      ) : (
        <div className="max-h-[420px] overflow-y-auto rounded-xl border border-stone-200 divide-y divide-stone-100">
          {classes.map((className) => {
            const assigned = assignments[className] ?? [];
            const availableUsers = users.filter((u) => !assigned.includes(u.externalUserId));
            return (
              <div key={className} className="px-4 py-3 bg-white even:bg-stone-50/50 space-y-2">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="w-16 shrink-0 font-bold text-[#1F3D2B]">{className}</span>
                  {assigned.length === 0 && (
                    <span className="text-xs text-stone-500">Aucun référent assigné</span>
                  )}
                </div>
                {assigned.length > 0 && (
                  <ul className="flex flex-wrap gap-2">
                    {assigned.map((id) => {
                      const u = userById.get(id);
                      if (!u) return null;
                      return (
                        <li
                          key={id}
                          className="inline-flex items-center gap-1 rounded-full border border-stone-200 bg-stone-50 px-2.5 py-1 text-xs"
                        >
                          <span>
                            {userLabel(u)} ({u.email})
                          </span>
                          <button
                            type="button"
                            onClick={() => removeReferent(className, id)}
                            className="text-rose-700 font-bold leading-none"
                            aria-label={`Retirer ${userLabel(u)}`}
                          >
                            ×
                          </button>
                        </li>
                      );
                    })}
                  </ul>
                )}
                {availableUsers.length > 0 && (
                  <div className="flex flex-wrap items-center gap-2">
                    <select
                      className="min-w-[220px] flex-1 rounded-lg border border-stone-300 px-3 py-2 text-sm"
                      value={pickerByClass[className] || ""}
                      onChange={(e) => {
                        const val = e.target.value;
                        if (val) addReferent(className, val);
                        else setPickerByClass((prev) => ({ ...prev, [className]: "" }));
                      }}
                    >
                      <option value="">+ Ajouter un référent</option>
                      {availableUsers.map((u) => (
                        <option key={u.externalUserId} value={u.externalUserId}>
                          {userLabel(u)} ({u.email})
                        </option>
                      ))}
                    </select>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      <button
        type="button"
        disabled={busy || classes.length === 0}
        onClick={() => void save()}
        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-semibold text-white disabled:opacity-50"
      >
        {busy ? "Enregistrement…" : "Enregistrer les référents"}
      </button>
    </div>
  );
}
