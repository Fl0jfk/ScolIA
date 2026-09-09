"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import type { StageWatcherKind } from "@/app/lib/stage-watchers-config";

type DirectoryUser = {
  externalUserId: string;
  email: string;
  firstName?: string;
  lastName?: string;
  displayName?: string;
  roles?: string[];
};

type ClassAssignment = {
  scope: "class";
  className: string;
  externalUserId: string;
  name: string;
  email: string;
  kind: StageWatcherKind;
};

type StudentAssignment = {
  scope: "student";
  studentKey: string;
  studentFirstName: string;
  studentLastName: string;
  studentClassName: string;
  externalUserId: string;
  name: string;
  email: string;
  kind: StageWatcherKind;
};

type Assignment = ClassAssignment | StudentAssignment;

function userLabel(u: DirectoryUser): string {
  return u.displayName || `${u.firstName ?? ""} ${u.lastName ?? ""}`.trim() || u.email;
}

export default function StageWatchersEditor({
  onSaved,
}: {
  onSaved?: (message: string) => void;
}) {
  const [classes, setClasses] = useState<string[]>([]);
  const [assignments, setAssignments] = useState<Assignment[]>([]);
  const [users, setUsers] = useState<DirectoryUser[]>([]);
  const [schoolYear, setSchoolYear] = useState("");
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [classPick, setClassPick] = useState<Record<string, { cpe: string; resto: string }>>({});
  const [studentForm, setStudentForm] = useState({
    firstName: "",
    lastName: "",
    className: "",
    userId: "",
    kind: "cpe" as StageWatcherKind,
  });

  const userById = useMemo(() => {
    const map = new Map<string, DirectoryUser>();
    for (const u of users) map.set(u.externalUserId, u);
    return map;
  }, [users]);

  const cpeUsers = useMemo(
    () => users.filter((u) => (u.roles || []).includes("cpe") || (u.roles || []).includes("surveillant")),
    [users],
  );
  const restoUsers = useMemo(
    () =>
      users.filter(
        (u) =>
          (u.roles || []).includes("accueil") ||
          (u.roles || []).includes("administratif") ||
          (u.roles || []).includes("cpe"),
      ),
    [users],
  );

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [wRes, usersRes] = await Promise.all([
        fetch("/api/stages/watchers", { cache: "no-store" }),
        fetch("/api/stages/directory-users", { cache: "no-store" }),
      ]);
      const wData = await wRes.json();
      const usersData = await usersRes.json();
      if (!wRes.ok) throw new Error(wData?.error || "Erreur watchers");
      if (!usersRes.ok) throw new Error(usersData?.error || "Erreur utilisateurs");
      setClasses(wData.classes || []);
      setSchoolYear(wData.schoolYear || "");
      setAssignments((wData.config?.assignments || []) as Assignment[]);
      setUsers(usersData.users || []);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  function addClassWatcher(className: string, kind: StageWatcherKind, externalUserId: string) {
    const u = userById.get(externalUserId);
    if (!u) return;
    setAssignments((prev) => {
      if (
        prev.some(
          (a) =>
            a.scope === "class" &&
            a.className === className &&
            a.kind === kind &&
            a.externalUserId === externalUserId,
        )
      ) {
        return prev;
      }
      return [
        ...prev,
        {
          scope: "class",
          className,
          externalUserId,
          name: userLabel(u),
          email: u.email,
          kind,
        },
      ];
    });
    setClassPick((prev) => ({
      ...prev,
      [className]: {
        cpe: prev[className]?.cpe || "",
        resto: prev[className]?.resto || "",
        ...(kind === "cpe" ? { cpe: "" } : { resto: "" }),
      },
    }));
  }

  function removeAssignment(pred: (a: Assignment) => boolean) {
    setAssignments((prev) => prev.filter((a) => !pred(a)));
  }

  function addStudentWatcher() {
    const u = userById.get(studentForm.userId);
    if (!u || !studentForm.firstName.trim() || !studentForm.lastName.trim()) return;
    const studentKey = `${studentForm.lastName.trim().toLowerCase()}|${studentForm.firstName.trim().toLowerCase()}|${studentForm.className.trim().toLowerCase()}`;
    setAssignments((prev) => [
      ...prev,
      {
        scope: "student",
        studentKey,
        studentFirstName: studentForm.firstName.trim(),
        studentLastName: studentForm.lastName.trim(),
        studentClassName: studentForm.className.trim(),
        externalUserId: u.externalUserId,
        name: userLabel(u),
        email: u.email,
        kind: studentForm.kind,
      },
    ]);
    setStudentForm({ firstName: "", lastName: "", className: "", userId: "", kind: "cpe" });
  }

  async function save() {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/watchers", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ schoolYear, assignments }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Enregistrement impossible");
      onSaved?.("Affectations CPE / restauration enregistrées.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (loading) return <p className="text-sm text-stone-500">Chargement…</p>;

  return (
    <div className="space-y-6">
      {error && (
        <p className="rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-800">
          {error}
        </p>
      )}
      <p className="text-sm text-stone-600">
        Les <strong>CPE</strong> voient les stages des classes (ou élèves) qui leur sont assignés —
        sans signer. La <strong>restauration</strong> voit les jours d&apos;absence repas liés aux
        stages.
      </p>

      <div className="max-h-[420px] space-y-3 overflow-y-auto rounded-xl border border-stone-200 divide-y">
        {classes.map((className) => {
          const classCpe = assignments.filter(
            (a) => a.scope === "class" && a.className === className && a.kind === "cpe",
          );
          const classResto = assignments.filter(
            (a) => a.scope === "class" && a.className === className && a.kind === "restauration",
          );
          const pick = classPick[className] || { cpe: "", resto: "" };
          return (
            <div key={className} className="space-y-2 bg-white px-4 py-3 even:bg-stone-50/60">
              <p className="font-bold text-[#1F3D2B]">{className}</p>
              <div className="grid gap-3 sm:grid-cols-2">
                <div>
                  <p className="text-xs font-semibold uppercase text-stone-500">CPE</p>
                  <ul className="mt-1 flex flex-wrap gap-1">
                    {classCpe.length === 0 ? (
                      <li className="text-xs text-stone-400">Aucun</li>
                    ) : (
                      classCpe.map((a) => (
                        <li
                          key={`${a.externalUserId}-cpe`}
                          className="inline-flex items-center gap-1 rounded-full border bg-stone-50 px-2 py-0.5 text-xs"
                        >
                          {a.name}
                          <button
                            type="button"
                            className="font-bold text-rose-700"
                            onClick={() =>
                              removeAssignment(
                                (x) =>
                                  x.scope === "class" &&
                                  x.className === className &&
                                  x.kind === "cpe" &&
                                  x.externalUserId === a.externalUserId,
                              )
                            }
                          >
                            ×
                          </button>
                        </li>
                      ))
                    )}
                  </ul>
                  <select
                    className="mt-1 w-full rounded border px-2 py-1 text-xs"
                    value={pick.cpe}
                    onChange={(e) => {
                      const id = e.target.value;
                      if (id) addClassWatcher(className, "cpe", id);
                    }}
                  >
                    <option value="">Ajouter un CPE…</option>
                    {cpeUsers.map((u) => (
                      <option key={u.externalUserId} value={u.externalUserId}>
                        {userLabel(u)}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <p className="text-xs font-semibold uppercase text-stone-500">Restauration</p>
                  <ul className="mt-1 flex flex-wrap gap-1">
                    {classResto.length === 0 ? (
                      <li className="text-xs text-stone-400">Aucun</li>
                    ) : (
                      classResto.map((a) => (
                        <li
                          key={`${a.externalUserId}-resto`}
                          className="inline-flex items-center gap-1 rounded-full border bg-amber-50 px-2 py-0.5 text-xs"
                        >
                          {a.name}
                          <button
                            type="button"
                            className="font-bold text-rose-700"
                            onClick={() =>
                              removeAssignment(
                                (x) =>
                                  x.scope === "class" &&
                                  x.className === className &&
                                  x.kind === "restauration" &&
                                  x.externalUserId === a.externalUserId,
                              )
                            }
                          >
                            ×
                          </button>
                        </li>
                      ))
                    )}
                  </ul>
                  <select
                    className="mt-1 w-full rounded border px-2 py-1 text-xs"
                    value={pick.resto}
                    onChange={(e) => {
                      const id = e.target.value;
                      if (id) addClassWatcher(className, "restauration", id);
                    }}
                  >
                    <option value="">Ajouter restauration…</option>
                    {restoUsers.map((u) => (
                      <option key={u.externalUserId} value={u.externalUserId}>
                        {userLabel(u)}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      <div className="rounded-xl border border-indigo-200 bg-indigo-50/50 p-4">
        <h3 className="text-sm font-bold text-[#1F3D2B]">Élève hors parcours (affectation individuelle)</h3>
        <p className="mt-1 text-xs text-stone-600">
          Pour un stage atypique : rattacher un CPE ou la restauration à un élève précis.
        </p>
        <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-5">
          <input
            className="rounded border px-2 py-1.5 text-sm"
            placeholder="Nom"
            value={studentForm.lastName}
            onChange={(e) => setStudentForm((s) => ({ ...s, lastName: e.target.value }))}
          />
          <input
            className="rounded border px-2 py-1.5 text-sm"
            placeholder="Prénom"
            value={studentForm.firstName}
            onChange={(e) => setStudentForm((s) => ({ ...s, firstName: e.target.value }))}
          />
          <input
            className="rounded border px-2 py-1.5 text-sm"
            placeholder="Classe"
            value={studentForm.className}
            onChange={(e) => setStudentForm((s) => ({ ...s, className: e.target.value }))}
          />
          <select
            className="rounded border px-2 py-1.5 text-sm"
            value={studentForm.kind}
            onChange={(e) =>
              setStudentForm((s) => ({ ...s, kind: e.target.value as StageWatcherKind }))
            }
          >
            <option value="cpe">CPE</option>
            <option value="restauration">Restauration</option>
          </select>
          <select
            className="rounded border px-2 py-1.5 text-sm"
            value={studentForm.userId}
            onChange={(e) => setStudentForm((s) => ({ ...s, userId: e.target.value }))}
          >
            <option value="">Personne…</option>
            {(studentForm.kind === "cpe" ? cpeUsers : restoUsers).map((u) => (
              <option key={u.externalUserId} value={u.externalUserId}>
                {userLabel(u)}
              </option>
            ))}
          </select>
        </div>
        <button
          type="button"
          onClick={addStudentWatcher}
          className="mt-3 rounded-lg border border-indigo-300 px-3 py-1.5 text-xs font-semibold text-indigo-900"
        >
          Ajouter l&apos;élève
        </button>
        <ul className="mt-3 space-y-1">
          {assignments
            .filter((a) => a.scope === "student")
            .map((a) => (
              <li
                key={`${a.studentKey}-${a.kind}-${a.externalUserId}`}
                className="flex flex-wrap items-center gap-2 text-xs"
              >
                <span className="font-semibold">
                  {a.studentLastName} {a.studentFirstName}
                </span>
                <span className="text-stone-500">({a.studentClassName || "—"})</span>
                <span className="rounded bg-white px-1.5 py-0.5 border">
                  {a.kind === "cpe" ? "CPE" : "Restauration"} · {a.name}
                </span>
                <button
                  type="button"
                  className="text-rose-700 font-bold"
                  onClick={() =>
                    removeAssignment(
                      (x) =>
                        x.scope === "student" &&
                        x.studentKey === a.studentKey &&
                        x.kind === a.kind &&
                        x.externalUserId === a.externalUserId,
                    )
                  }
                >
                  Retirer
                </button>
              </li>
            ))}
        </ul>
      </div>

      <button
        type="button"
        disabled={busy}
        onClick={() => void save()}
        className="rounded-lg bg-[#2F6B4A] px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
      >
        {busy ? "Enregistrement…" : "Enregistrer les affectations"}
      </button>
    </div>
  );
}
