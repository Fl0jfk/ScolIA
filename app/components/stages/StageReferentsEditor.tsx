"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { StageReferentRole } from "@/app/lib/stage-referents-config";

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
  role: StageReferentRole;
};

type ClassSlots = {
  principalIds: string[];
  referentIds: string[];
};

function userLabel(u: DirectoryUser): string {
  return u.displayName || `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim() || u.email;
}

function emptySlots(): ClassSlots {
  return { principalIds: [], referentIds: [] };
}

export default function StageReferentsEditor({
  onSaved,
}: {
  onSaved?: (message: string) => void;
}) {
  const [classes, setClasses] = useState<string[]>([]);
  const [slots, setSlots] = useState<Record<string, ClassSlots>>({});
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [updatedAt, setUpdatedAt] = useState<string | null>(null);
  const [updatedBy, setUpdatedBy] = useState<string | null>(null);
  const [previousConfig, setPreviousConfig] = useState<{
    schoolYear: string;
    assignments: Assignment[];
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pickerByClass, setPickerByClass] = useState<
    Record<string, { principal: string; referent: string }>
  >({});

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

      const map: Record<string, ClassSlots> = {};
      for (const className of refData.classes || []) {
        map[className] = emptySlots();
      }
      for (const a of (refData.config?.assignments || []) as Assignment[]) {
        if (!map[a.className]) map[a.className] = emptySlots();
        const role = a.role === "professeur_principal" ? "principalIds" : "referentIds";
        if (!map[a.className]![role].includes(a.externalUserId)) {
          map[a.className]![role].push(a.externalUserId);
        }
      }
      setSlots(map);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  function addTeacher(className: string, role: StageReferentRole, externalUserId: string) {
    if (!externalUserId) return;
    const key = role === "professeur_principal" ? "principalIds" : "referentIds";
    setSlots((prev) => {
      const current = prev[className] ?? emptySlots();
      if (current[key].includes(externalUserId)) return prev;
      return {
        ...prev,
        [className]: { ...current, [key]: [...current[key], externalUserId] },
      };
    });
    setPickerByClass((prev) => ({
      ...prev,
      [className]: {
        principal: prev[className]?.principal || "",
        referent: prev[className]?.referent || "",
        ...(role === "professeur_principal" ? { principal: "" } : { referent: "" }),
      },
    }));
  }

  function removeTeacher(className: string, role: StageReferentRole, externalUserId: string) {
    const key = role === "professeur_principal" ? "principalIds" : "referentIds";
    setSlots((prev) => {
      const current = prev[className] ?? emptySlots();
      return {
        ...prev,
        [className]: {
          ...current,
          [key]: current[key].filter((id) => id !== externalUserId),
        },
      };
    });
  }

  function copyFromPreviousYear() {
    if (!previousConfig?.assignments.length) return;
    const map: Record<string, ClassSlots> = {};
    for (const className of classes) map[className] = emptySlots();
    for (const a of previousConfig.assignments) {
      if (!classes.includes(a.className)) continue;
      if (!userById.has(a.externalUserId)) continue;
      if (!map[a.className]) map[a.className] = emptySlots();
      const key = a.role === "professeur_principal" ? "principalIds" : "referentIds";
      if (!map[a.className]![key].includes(a.externalUserId)) {
        map[a.className]![key].push(a.externalUserId);
      }
    }
    setSlots(map);
    onSaved?.(`Affectations copiées depuis ${previousConfig.schoolYear}.`);
  }

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const payload: Assignment[] = [];
      for (const className of classes) {
        const s = slots[className] ?? emptySlots();
        for (const externalUserId of s.principalIds) {
          const u = userById.get(externalUserId);
          if (!u) continue;
          payload.push({
            className,
            externalUserId,
            name: userLabel(u),
            email: u.email,
            role: "professeur_principal",
          });
        }
        for (const externalUserId of s.referentIds) {
          const u = userById.get(externalUserId);
          if (!u) continue;
          payload.push({
            className,
            externalUserId,
            name: userLabel(u),
            email: u.email,
            role: "professeur_referent",
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
      onSaved?.(`Professeurs enregistrés (${payload.length} affectation(s)).`);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const assignedCount = Object.values(slots).reduce(
    (n, s) => n + s.principalIds.length + s.referentIds.length,
    0,
  );

  if (loading) {
    return <p className="text-sm text-stone-500">Chargement des référents…</p>;
  }

  return (
    <div className="space-y-4">
      {error && (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
          {error}
        </p>
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
        Pour chaque classe : un <strong>professeur principal</strong> (peut déléguer un référent
        stage par élève) et un ou plusieurs <strong>professeurs référents</strong> stages. Ce
        n&apos;est pas forcément la même personne. Les professeurs n&apos;ont pas accès aux
        réglages.
      </p>

      {updatedAt && (
        <p className="text-xs text-stone-500">
          Dernière mise à jour : {new Date(updatedAt).toLocaleString("fr-FR")}
          {updatedBy ? ` par ${updatedBy}` : ""} — {assignedCount} affectation(s)
        </p>
      )}

      {classes.length === 0 ? (
        <p className="rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900">
          Aucune classe activée dans « Classes concernées par les stages ». Une classe hors liste
          s&apos;ajoute automatiquement dès qu&apos;un élève y dépose une préconvention.
        </p>
      ) : users.length === 0 ? (
        <p className="text-sm text-amber-800 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2">
          Aucun utilisateur avec le rôle professeur trouvé.
        </p>
      ) : (
        <div className="max-h-[520px] overflow-y-auto rounded-xl border border-stone-200 divide-y divide-stone-100">
          {classes.map((className) => {
            const s = slots[className] ?? emptySlots();
            const availablePrincipal = users.filter((u) => !s.principalIds.includes(u.externalUserId));
            const availableReferent = users.filter((u) => !s.referentIds.includes(u.externalUserId));
            const picker = pickerByClass[className] || { principal: "", referent: "" };

            const renderChips = (ids: string[], role: StageReferentRole) =>
              ids.map((id) => {
                const u = userById.get(id);
                if (!u) return null;
                return (
                  <li
                    key={`${role}-${id}`}
                    className="inline-flex items-center gap-1 rounded-full border border-stone-200 bg-stone-50 px-2.5 py-1 text-xs"
                  >
                    <span>
                      {userLabel(u)} ({u.email})
                    </span>
                    <button
                      type="button"
                      onClick={() => removeTeacher(className, role, id)}
                      className="text-rose-700 font-bold leading-none"
                      aria-label={`Retirer ${userLabel(u)}`}
                    >
                      ×
                    </button>
                  </li>
                );
              });

            return (
              <div key={className} className="px-4 py-3 bg-white even:bg-stone-50/50 space-y-3">
                <span className="font-bold text-[#1F3D2B]">{className}</span>

                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-wide text-stone-500">
                    Professeur principal
                  </p>
                  <ul className="flex flex-wrap gap-2">
                    {s.principalIds.length === 0 ? (
                      <li className="text-xs text-stone-500">Aucun</li>
                    ) : (
                      renderChips(s.principalIds, "professeur_principal")
                    )}
                  </ul>
                  {availablePrincipal.length > 0 && (
                    <select
                      className="min-w-[220px] w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
                      value={picker.principal}
                      onChange={(e) => {
                        const val = e.target.value;
                        if (val) addTeacher(className, "professeur_principal", val);
                        else {
                          setPickerByClass((prev) => ({
                            ...prev,
                            [className]: { ...picker, principal: "" },
                          }));
                        }
                      }}
                    >
                      <option value="">+ Ajouter un professeur principal</option>
                      {availablePrincipal.map((u) => (
                        <option key={u.externalUserId} value={u.externalUserId}>
                          {userLabel(u)} ({u.email})
                        </option>
                      ))}
                    </select>
                  )}
                </div>

                <div className="space-y-2">
                  <p className="text-xs font-semibold uppercase tracking-wide text-stone-500">
                    Professeur(s) référent(s) stage
                  </p>
                  <ul className="flex flex-wrap gap-2">
                    {s.referentIds.length === 0 ? (
                      <li className="text-xs text-stone-500">Aucun (le PP peut déléguer par élève)</li>
                    ) : (
                      renderChips(s.referentIds, "professeur_referent")
                    )}
                  </ul>
                  {availableReferent.length > 0 && (
                    <select
                      className="min-w-[220px] w-full rounded-lg border border-stone-300 px-3 py-2 text-sm"
                      value={picker.referent}
                      onChange={(e) => {
                        const val = e.target.value;
                        if (val) addTeacher(className, "professeur_referent", val);
                        else {
                          setPickerByClass((prev) => ({
                            ...prev,
                            [className]: { ...picker, referent: "" },
                          }));
                        }
                      }}
                    >
                      <option value="">+ Ajouter un référent stage</option>
                      {availableReferent.map((u) => (
                        <option key={u.externalUserId} value={u.externalUserId}>
                          {userLabel(u)} ({u.email})
                        </option>
                      ))}
                    </select>
                  )}
                </div>
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
        {busy ? "Enregistrement…" : "Enregistrer PP / référents"}
      </button>
    </div>
  );
}
