"use client";

import { useMemo } from "react";
import type {
  RequestsRoutingConfig,
  RoutingAssignment,
  RoutingTask,
} from "@/app/lib/app-config-schemas";
import type { ClerkMemberOption } from "@/app/components/prof-room/ProfRoomAdminPicker";

function uid(prefix: string) {
  return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
}

type Props = {
  config: RequestsRoutingConfig;
  onChange: (next: RequestsRoutingConfig) => void;
  members: ClerkMemberOption[];
  membersLoading: boolean;
};

export default function RequestsRoutingEditor({ config, onChange, members, membersLoading }: Props) {
  const activeTaskIds = useMemo(
    () => new Set(config.tasks.filter((t) => t.active).map((t) => t.id)),
    [config.tasks],
  );

  const updateTask = (idx: number, patch: Partial<RoutingTask>) => {
    const tasks = [...config.tasks];
    tasks[idx] = { ...tasks[idx]!, ...patch };
    onChange({ ...config, tasks });
  };

  const addTask = () => {
    const id = uid("task");
    onChange({
      ...config,
      tasks: [
        ...config.tasks,
        { id, label: "Nouvelle tâche", hint: "", keywords: [], active: true },
      ],
    });
  };

  const removeTask = (idx: number) => {
    const taskId = config.tasks[idx]?.id;
    onChange({
      ...config,
      tasks: config.tasks.filter((_, i) => i !== idx),
      assignments: config.assignments.filter((a) => a.taskId !== taskId),
    });
  };

  const updateAssignment = (idx: number, patch: Partial<RoutingAssignment>) => {
    const assignments = [...config.assignments];
    assignments[idx] = { ...assignments[idx]!, ...patch };
    onChange({ ...config, assignments });
  };

  const addAssignment = (taskId?: string) => {
    const firstTask = config.tasks.find((t) => t.active) || config.tasks[0];
    const tid = taskId || firstTask?.id || "corbeille";
    const service = config.services.find((s) => !s.manualOnly) || config.services[0];
    onChange({
      ...config,
      assignments: [
        ...config.assignments,
        {
          id: uid("asg"),
          taskId: tid,
          email: "",
          personName: "",
          serviceId: service?.id || "administratif",
          active: true,
        },
      ],
    });
  };

  const removeAssignment = (idx: number) => {
    onChange({
      ...config,
      assignments: config.assignments.filter((_, i) => i !== idx),
    });
  };

  const memberEmails = useMemo(() => {
    const fromMembers = members
      .map((m) => m.email?.trim())
      .filter((e): e is string => Boolean(e));
    const fromAssignments = config.assignments.map((a) => a.email).filter(Boolean);
    return [...new Set([...fromMembers, ...fromAssignments])].sort((a, b) => a.localeCompare(b, "fr"));
  }, [members, config.assignments]);

  return (
    <div className="space-y-8">
      <section className="space-y-4 rounded-2xl border border-amber-200 bg-amber-50/40 p-6">
        <div>
          <h2 className="text-lg font-bold text-slate-900">Page publique parents</h2>
          <p className="mt-1 text-sm text-slate-600">
            Ouvre une page simple pour que les familles déposent une demande. Confirmation par
            e-mail anti-spam ; reconnaissance automatique si l&apos;e-mail figure dans la liste
            élèves.
          </p>
        </div>
        <label className="flex items-center gap-3 text-sm font-bold text-slate-800">
          <input
            type="checkbox"
            className="h-4 w-4 rounded border-slate-300"
            checked={config.parentPortal?.enabled === true}
            onChange={(e) =>
              onChange({
                ...config,
                parentPortal: { enabled: e.target.checked },
              })
            }
          />
          Ouvrir la page de demandes parents
        </label>
        {config.parentPortal?.enabled ? (
          <div className="rounded-xl border border-amber-200/80 bg-white px-4 py-3 text-sm">
            <p className="font-bold text-slate-800">Lien à partager</p>
            <p className="mt-1 break-all font-mono text-xs text-amber-900">
              {typeof window !== "undefined"
                ? `${window.location.origin}/demande-parents`
                : "/demande-parents"}
            </p>
            <p className="mt-2 text-xs text-slate-500">
              Formulaire : nom, e-mail, téléphone facultatif, texte, pièce jointe facultative.
            </p>
          </div>
        ) : (
          <p className="text-xs text-slate-500">
            Désactivé par défaut. Activez puis enregistrez le routage pour publier le lien.
          </p>
        )}
        <p className="text-xs text-slate-500 border-t border-amber-200/60 pt-3">
          Tags métier du personnel (pour l&apos;IA) : onglet{" "}
          <a href="/requests" className="font-bold text-indigo-700 underline">
            Demandes → Tags équipe
          </a>
          .
        </p>
      </section>

      <section className="bg-white rounded-2xl border p-6 space-y-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Catalogue de tâches</h2>
            <p className="text-sm text-slate-500 mt-1">
              Chaque tâche décrit un type de demande. Une tâche inactive disparaît du routage IA mais reste
              enregistrée. Vous pouvez créer plusieurs affectations pour la même tâche (personnes différentes).
            </p>
          </div>
          <button
            type="button"
            onClick={addTask}
            className="shrink-0 px-4 py-2 rounded-xl bg-indigo-600 text-white text-sm font-bold"
          >
            + Tâche
          </button>
        </div>

        {config.tasks.length === 0 && (
          <p className="text-sm text-slate-400 italic">Aucune tâche — ajoutez-en une pour commencer.</p>
        )}

        {config.tasks.map((task, idx) => (
          <div
            key={task.id}
            className={`rounded-xl border p-4 space-y-3 ${task.active ? "border-slate-200" : "border-amber-200 bg-amber-50/40"}`}
          >
            <div className="flex flex-wrap items-center gap-3">
              <label className="flex items-center gap-2 text-sm font-bold text-slate-700">
                <input
                  type="checkbox"
                  checked={task.active}
                  onChange={(e) => updateTask(idx, { active: e.target.checked })}
                />
                Active (disponible au routage)
              </label>
              <span className="text-xs text-slate-400 font-mono">{task.id}</span>
              <button
                type="button"
                onClick={() => addAssignment(task.id)}
                className="ml-auto text-xs font-bold text-indigo-600 underline"
              >
                + Affecter quelqu&apos;un
              </button>
              <button
                type="button"
                onClick={() => removeTask(idx)}
                className="text-xs font-bold text-red-600 underline"
              >
                Supprimer
              </button>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <label className="block text-xs font-bold text-slate-500 mb-1">Libellé</label>
                <input
                  className="w-full border rounded-lg p-2 text-sm"
                  value={task.label}
                  onChange={(e) => updateTask(idx, { label: e.target.value })}
                />
              </div>
              <div>
                <label className="block text-xs font-bold text-slate-500 mb-1">Indice pour l&apos;IA</label>
                <input
                  className="w-full border rounded-lg p-2 text-sm"
                  value={task.hint}
                  onChange={(e) => updateTask(idx, { hint: e.target.value })}
                  placeholder="Ex. : pannes, fournitures, locaux…"
                />
              </div>
            </div>
            <div>
              <label className="block text-xs font-bold text-slate-500 mb-1">
                Mots-clés (séparés par des virgules)
              </label>
              <input
                className="w-full border rounded-lg p-2 text-sm"
                value={task.keywords.join(", ")}
                onChange={(e) =>
                  updateTask(idx, {
                    keywords: e.target.value
                      .split(",")
                      .map((s) => s.trim())
                      .filter(Boolean),
                  })
                }
              />
            </div>
          </div>
        ))}
      </section>

      <section className="bg-white rounded-2xl border p-6 space-y-4">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-bold text-slate-900">Affectations (tâche → personne)</h2>
            <p className="text-sm text-slate-500 mt-1">
              Mistral choisit une affectation dans ce catalogue. Désactivez une affectation pour la retirer du
              select IA sans supprimer la tâche.
            </p>
          </div>
          <button
            type="button"
            onClick={() => addAssignment()}
            className="shrink-0 px-4 py-2 rounded-xl bg-slate-800 text-white text-sm font-bold"
          >
            + Affectation
          </button>
        </div>

        {membersLoading && <p className="text-xs text-slate-400">Chargement des membres Clerk…</p>}

        {config.assignments.map((asg, idx) => {
          const task = config.tasks.find((t) => t.id === asg.taskId);
          const taskInactive = !task?.active;
          const hidden = !asg.active || !activeTaskIds.has(asg.taskId);
          return (
            <div
              key={asg.id}
              className={`rounded-xl border p-4 space-y-3 ${hidden ? "border-slate-200 bg-slate-50 opacity-75" : "border-emerald-200"}`}
            >
              <div className="flex flex-wrap items-center gap-3">
                <label className="flex items-center gap-2 text-sm font-bold text-slate-700">
                  <input
                    type="checkbox"
                    checked={asg.active}
                    onChange={(e) => updateAssignment(idx, { active: e.target.checked })}
                  />
                  Affectation active
                </label>
                {taskInactive && (
                  <span className="text-xs text-amber-700 font-bold">Tâche parente inactive</span>
                )}
                <button
                  type="button"
                  onClick={() => removeAssignment(idx)}
                  className="ml-auto text-xs font-bold text-red-600 underline"
                >
                  Supprimer
                </button>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <div>
                  <label className="block text-xs font-bold text-slate-500 mb-1">Tâche</label>
                  <select
                    className="w-full border rounded-lg p-2 text-sm"
                    value={asg.taskId}
                    onChange={(e) => updateAssignment(idx, { taskId: e.target.value })}
                  >
                    {config.tasks.map((t) => (
                      <option key={t.id} value={t.id}>
                        {t.active ? "" : "⏸ "}
                        {t.label} ({t.id})
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-xs font-bold text-slate-500 mb-1">Service</label>
                  <select
                    className="w-full border rounded-lg p-2 text-sm"
                    value={asg.serviceId}
                    onChange={(e) => updateAssignment(idx, { serviceId: e.target.value })}
                  >
                    {config.services.map((s) => (
                      <option key={s.id} value={s.id}>
                        {s.label}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-xs font-bold text-slate-500 mb-1">Personne (email)</label>
                  <select
                    className="w-full border rounded-lg p-2 text-sm"
                    value={asg.email}
                    onChange={(e) => {
                      const email = e.target.value;
                      const member = members.find((m) => m.email === email);
                      const personName = member
                        ? [member.firstName, member.lastName].filter(Boolean).join(" ") || email.split("@")[0]
                        : asg.personName;
                      updateAssignment(idx, { email, personName: personName || "" });
                    }}
                  >
                    <option value="">— Choisir —</option>
                    {memberEmails.map((email) => (
                      <option key={email} value={email}>
                        {email}
                      </option>
                    ))}
                  </select>
                </div>
                <div>
                  <label className="block text-xs font-bold text-slate-500 mb-1">Nom affiché</label>
                  <input
                    className="w-full border rounded-lg p-2 text-sm"
                    value={asg.personName}
                    onChange={(e) => updateAssignment(idx, { personName: e.target.value })}
                  />
                </div>
              </div>
            </div>
          );
        })}
      </section>

      <section className="bg-white rounded-2xl border p-6 space-y-4">
        <h2 className="text-lg font-bold text-slate-900">Files direction (transfert manuel uniquement)</h2>
        <p className="text-sm text-slate-500">
          Ces files ne reçoivent jamais de demande automatiquement — elles servent d&apos;indicateur et de
          cible pour « Transmettre direction ».
        </p>
        {config.directionQueues.map((q, idx) => (
          <div key={q.id} className="flex flex-wrap items-center gap-3 rounded-xl border p-3">
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={q.active}
                onChange={(e) => {
                  const directionQueues = [...config.directionQueues];
                  directionQueues[idx] = { ...q, active: e.target.checked };
                  onChange({ ...config, directionQueues });
                }}
              />
              Active
            </label>
            <span className="text-sm font-bold text-slate-700">{q.label}</span>
            <input
              className="flex-1 min-w-[200px] border rounded-lg p-2 text-sm"
              value={q.email}
              onChange={(e) => {
                const directionQueues = [...config.directionQueues];
                directionQueues[idx] = { ...q, email: e.target.value };
                onChange({ ...config, directionQueues });
              }}
            />
          </div>
        ))}
      </section>
    </div>
  );
}
