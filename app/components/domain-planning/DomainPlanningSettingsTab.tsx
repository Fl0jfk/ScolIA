"use client";

import { useEffect, useState } from "react";
import {
  DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS,
  DEFAULT_DOMAIN_PLANNING_DOMAINS,
  normalizeDomainPlanningModule,
  TRANSVERSAL_NIVEAU_LABELS,
} from "@/app/lib/domain-planning-defaults";
import type { DomainPlanningSession } from "@/app/lib/domain-planning-types";
import { PROF_ROOM_COLOR_PRESETS } from "@/app/lib/prof-room-subject-colors";
import SubjectColorEditor from "@/app/components/prof-room/SubjectColorEditor";
import DomainCoordinatorPicker, { type DirectoryMemberOption } from "./DomainCoordinatorPicker";

type Domain = {
  id: string;
  name: string;
  description?: string;
  color?: string;
  coordinatorExternalUserIds: string[];
};

type ModuleConfig = {
  classesByPole: Record<string, string[]>;
  activityColors: Record<string, string>;
  bookingHorizonDays: number;
  hoursStart: number;
  hoursEnd: number;
};

function slugifyDomainId(name: string): string {
  const base =
    name
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "") || `domaine-${Date.now()}`;
  return base;
}

function uniqueDomainId(name: string, domains: Domain[], skipIdx?: number): string {
  const base = slugifyDomainId(name);
  const used = new Set(domains.filter((_, i) => i !== skipIdx).map((d) => d.id));
  if (!used.has(base)) return base;
  let n = 2;
  while (used.has(`${base}-${n}`)) n++;
  return `${base}-${n}`;
}

export default function DomainPlanningSettingsTab() {
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [domains, setDomains] = useState<Domain[]>([]);
  const [members, setMembers] = useState<DirectoryMemberOption[]>([]);
  const [config, setConfig] = useState<ModuleConfig>({
    classesByPole: {},
    activityColors: {},
    bookingHorizonDays: 56,
    hoursStart: 8,
    hoursEnd: 17,
  });
  const [newActivity, setNewActivity] = useState("");
  const [newActivityColor, setNewActivityColor] = useState(PROF_ROOM_COLOR_PRESETS[0].value);
  const [newPoleName, setNewPoleName] = useState("");
  const [newClassByPole, setNewClassByPole] = useState<Record<string, string>>({});
  const [sessions, setSessions] = useState<DomainPlanningSession[]>([]);
  useEffect(() => {
    (async () => {
      try {
        const [domainsRes, configRes, usersRes, sessionsRes] = await Promise.all([
          fetch("/api/domain-planning/domains"),
          fetch("/api/domain-planning/module-config"),
          fetch("/api/domain-planning/directory-users"),
          fetch("/api/domain-planning/sessions"),
        ]);
        const domainsJson = await domainsRes.json();
        const configJson = await configRes.json();
        const usersJson = await usersRes.json();
        if (!domainsRes.ok) throw new Error(domainsJson.error || "Domaines introuvables");
        if (!configRes.ok) throw new Error(configJson.error || "Configuration introuvable");
        setDomains(domainsJson.domains || []);
        const loaded = configJson.config || {
          classesByPole: {},
          activityColors: {},
          bookingHorizonDays: 56,
          hoursStart: 8,
          hoursEnd: 17,
        };
        setConfig(
          normalizeDomainPlanningModule({
            ...loaded,
            activityColors: { ...DEFAULT_DOMAIN_PLANNING_ACTIVITY_COLORS, ...loaded.activityColors },
          }),
        );
        if (!usersRes.ok) {
          throw new Error(usersJson.error || "Impossible de charger les utilisateurs du directory.");
        }
        setMembers((usersJson.users || []) as DirectoryMemberOption[]);
        if (sessionsRes.ok) {
          const sessionsJson = await sessionsRes.json();
          setSessions(sessionsJson.sessions || []);
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Erreur de chargement");
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  const saveDomains = async () => {
    setSaving(true);
    setError(null);
    try {
      const normalized = domains
        .filter((d) => d.name.trim())
        .map((d, idx) => {
          const name = d.name.trim();
          const isNew = !d.id || /^domaine-\d+$/.test(d.id);
          return {
            ...d,
            name,
            id: isNew ? uniqueDomainId(name, domains, idx) : d.id.trim(),
            coordinatorExternalUserIds: d.coordinatorExternalUserIds || [],
          };
        });
      const res = await fetch("/api/domain-planning/domains", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ domains: normalized }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement domaines");
      setDomains(normalized);
      alert("Domaines enregistrés.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const saveSessions = async () => {
    setSaving(true);
    setError(null);
    try {
      const res = await fetch("/api/domain-planning/sessions", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ sessions }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement séances");
      if (j.sessions) setSessions(j.sessions);
      alert("Séances enregistrées.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const saveModuleConfig = async (nextConfig: ModuleConfig = config, silent = false) => {
    setSaving(true);
    setError(null);
    try {
      const payload = normalizeDomainPlanningModule(nextConfig);
      const res = await fetch("/api/domain-planning/module-config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Échec enregistrement");
      if (j.config) setConfig(normalizeDomainPlanningModule(j.config));
      if (!silent) alert("Paramètres enregistrés.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  const addActivity = () => {
    const name = newActivity.trim();
    if (!name) return;
    if (config.activityColors[name]) {
      alert("Ce libellé existe déjà.");
      return;
    }
    setConfig({
      ...config,
      activityColors: { ...config.activityColors, [name]: newActivityColor },
    });
    setNewActivity("");
  };

  const removeActivity = (name: string) => {
    if (!confirm(`Supprimer « ${name} » ?`)) return;
    const next = { ...config.activityColors };
    delete next[name];
    setConfig({ ...config, activityColors: next });
  };

  const addPole = () => {
    const name = newPoleName.trim().toUpperCase();
    if (!name) return;
    if (config.classesByPole[name]) {
      alert("Ce pôle existe déjà.");
      return;
    }
    setConfig({ ...config, classesByPole: { ...config.classesByPole, [name]: [] } });
    setNewPoleName("");
  };

  const addClassToPole = (pole: string) => {
    const cls = (newClassByPole[pole] || "").trim().toUpperCase();
    if (!cls) return;
    const list = config.classesByPole[pole] || [];
    if (list.includes(cls)) return;
    setConfig({
      ...config,
      classesByPole: { ...config.classesByPole, [pole]: [...list, cls] },
    });
    setNewClassByPole({ ...newClassByPole, [pole]: "" });
  };

  const removeClassFromPole = (pole: string, cls: string) => {
    setConfig({
      ...config,
      classesByPole: {
        ...config.classesByPole,
        [pole]: (config.classesByPole[pole] || []).filter((c) => c !== cls),
      },
    });
  };

  const removePole = async (pole: string) => {
    if (!confirm(`Supprimer le pôle « ${pole} » et toutes ses classes ?`)) return;
    const nextClasses = { ...config.classesByPole };
    delete nextClasses[pole];
    const nextConfig = { ...config, classesByPole: nextClasses };
    setConfig(nextConfig);
    const nextInputs = { ...newClassByPole };
    delete nextInputs[pole];
    setNewClassByPole(nextInputs);
    await saveModuleConfig(nextConfig, true);
  };

  if (loading) {
    return <p className="p-10 text-center text-slate-500 font-bold">Chargement du paramétrage…</p>;
  }

  return (
    <div className="space-y-8 px-4 pb-8">
      {error && <p className="text-red-600 text-sm font-bold bg-red-50 p-3 rounded-xl">{error}</p>}

      <section className="bg-violet-50 rounded-3xl border border-violet-200 p-6 space-y-4">
        <h2 className="text-lg font-black text-slate-900">Domaines & responsables</h2>
        <p className="text-sm text-slate-600 leading-relaxed">
          Créez un domaine (ex. EVARS), choisissez sa couleur, puis sélectionnez ses <strong>responsables</strong>{" "}
          dans la liste directory. La responsable EVARS peut modifier les séances et gérer les positionnements des
          professeurs.
        </p>
        {domains.map((domain, idx) => {
          const defaultPreset =
            DEFAULT_DOMAIN_PLANNING_DOMAINS.find((d) => d.id === domain.id)?.color ||
            PROF_ROOM_COLOR_PRESETS[idx % PROF_ROOM_COLOR_PRESETS.length].value;
          const domainColor = domain.color || defaultPreset;
          return (
          <div key={domain.id || idx} className="bg-white border rounded-2xl p-4 space-y-3 shadow-sm">
            <div className="flex gap-2">
              <input
                className="flex-1 border rounded-xl p-3 text-sm font-bold"
                placeholder="Nom (ex: EVARS)"
                value={domain.name}
                onChange={(e) => {
                  const next = [...domains];
                  next[idx] = { ...domain, name: e.target.value };
                  setDomains(next);
                }}
              />
              <button
                type="button"
                onClick={() => setDomains(domains.filter((_, i) => i !== idx))}
                className="text-red-600 text-sm font-bold px-3 shrink-0"
              >
                Supprimer
              </button>
            </div>
            <input
              className="w-full border rounded-xl p-3 text-sm"
              placeholder="Description courte (optionnel)"
              value={domain.description || ""}
              onChange={(e) => {
                const next = [...domains];
                next[idx] = { ...domain, description: e.target.value };
                setDomains(next);
              }}
            />
            <SubjectColorEditor
              label={domain.name.trim() || "Aperçu domaine"}
              value={domainColor}
              onChange={(next) => {
                const nextDomains = [...domains];
                nextDomains[idx] = { ...domain, color: next };
                setDomains(nextDomains);
              }}
            />
            <div className="pt-1 border-t border-slate-100">
              <p className="text-[10px] font-black text-slate-500 uppercase tracking-widest mb-2">
                {domain.id === "evars" ? "Responsable(s) EVARS" : "Responsables du domaine"}
              </p>
              {domain.coordinatorExternalUserIds.length > 0 && (
                <p className="text-sm font-bold text-violet-800 mb-2">
                  {members
                    .filter((m) => domain.coordinatorExternalUserIds.includes(m.externalUserId))
                    .map((m) => m.displayName || `${m.firstName ?? ""} ${m.lastName ?? ""}`.trim() || m.email)
                    .join(" · ") || `${domain.coordinatorExternalUserIds.length} responsable(s) sélectionné(s)`}
                </p>
              )}
              <DomainCoordinatorPicker
                domainName={domain.name.trim() || "Domaine"}
                members={members}
                selectedIds={domain.coordinatorExternalUserIds}
                onChange={(ids) => {
                  const next = [...domains];
                  next[idx] = { ...domain, coordinatorExternalUserIds: ids };
                  setDomains(next);
                }}
              />
            </div>
          </div>
        );
        })}
        <div className="flex flex-wrap items-center gap-4 pt-1">
          <button
            type="button"
            className="text-violet-600 font-bold text-sm"
            onClick={() =>
              setDomains([
                ...domains,
                {
                  id: `domaine-${Date.now()}`,
                  name: "",
                  color: PROF_ROOM_COLOR_PRESETS[domains.length % PROF_ROOM_COLOR_PRESETS.length].value,
                  coordinatorExternalUserIds: [],
                },
              ])
            }
          >
            + Ajouter un domaine
          </button>
          <button
            type="button"
            disabled={saving}
            onClick={saveDomains}
            className="bg-violet-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
          >
            Enregistrer les domaines
          </button>
        </div>
      </section>

      <section className="bg-white rounded-3xl border p-6 space-y-4">
        <h2 className="text-lg font-black text-slate-900">Séances EVARS (collège)</h2>
        <p className="text-sm text-slate-600">
          Thèmes et règles d&apos;inscription pour chaque séance. <strong>SVT obligatoire</strong> en séance 1,{" "}
          <strong>intervenant imposé</strong> en séance 2, <strong>choix libre</strong> en séance 3.
        </p>
        <div className="space-y-3 max-h-[32rem] overflow-y-auto pr-1">
          {sessions.map((session, idx) => (
            <div key={session.id} className="border rounded-xl p-3 space-y-2 text-sm">
              <p className="font-black text-slate-800">
                {TRANSVERSAL_NIVEAU_LABELS[session.niveau]} — Séance {session.seanceNumber}
              </p>
              <textarea
                className="w-full border rounded-lg p-2 text-sm"
                rows={2}
                value={session.theme}
                onChange={(e) => {
                  const next = [...sessions];
                  next[idx] = { ...session, theme: e.target.value };
                  setSessions(next);
                }}
              />
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2">
                <input
                  className="border rounded-lg p-2 text-sm font-bold"
                  value={session.intervenantLabel}
                  onChange={(e) => {
                    const next = [...sessions];
                    next[idx] = { ...session, intervenantLabel: e.target.value };
                    setSessions(next);
                  }}
                  placeholder="Intervenant affiché"
                />
                <select
                  className="border rounded-lg p-2 text-sm font-bold"
                  value={session.intervenantConstraint}
                  onChange={(e) => {
                    const next = [...sessions];
                    next[idx] = {
                      ...session,
                      intervenantConstraint: e.target.value as DomainPlanningSession["intervenantConstraint"],
                    };
                    setSessions(next);
                  }}
                >
                  <option value="svt_only">SVT uniquement</option>
                  <option value="fixed_association">Association (verrouillé)</option>
                  <option value="psy_inf">Psychologue / Infirmière</option>
                  <option value="free">Choix libre (validation EVARS)</option>
                </select>
                <label className="flex items-center gap-2 font-bold text-slate-700 px-2">
                  <input
                    type="checkbox"
                    checked={session.mixte}
                    onChange={(e) => {
                      const next = [...sessions];
                      next[idx] = { ...session, mixte: e.target.checked };
                      setSessions(next);
                    }}
                  />
                  Mixte
                </label>
              </div>
            </div>
          ))}
        </div>
        <button
          type="button"
          disabled={saving}
          onClick={() => void saveSessions()}
          className="bg-rose-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
        >
          Enregistrer les séances
        </button>
      </section>

      <section className="bg-white rounded-3xl border p-6 space-y-4">
        <h2 className="text-lg font-black text-slate-900">Libellés d&apos;activité & couleurs</h2>
        <div className="space-y-2">
          {Object.entries(config.activityColors).map(([name, colorValue]) => (
            <SubjectColorEditor
              key={name}
              label={name}
              value={colorValue}
              onChange={(next) =>
                setConfig({
                  ...config,
                  activityColors: { ...config.activityColors, [name]: next },
                })
              }
              onRemove={() => removeActivity(name)}
            />
          ))}
        </div>
        <div className="flex flex-col gap-3 pt-2 border-t">
          <input
            className="w-full border rounded-xl p-3 text-sm font-bold"
            placeholder="Nouveau libellé (ex: Séance 1)"
            value={newActivity}
            onChange={(e) => setNewActivity(e.target.value)}
          />
          <SubjectColorEditor label="Aperçu" value={newActivityColor} onChange={setNewActivityColor} />
          <button
            type="button"
            onClick={addActivity}
            className="bg-slate-800 text-white px-4 py-2 rounded-xl font-bold text-sm self-start"
          >
            Ajouter le libellé
          </button>
        </div>
      </section>

      <section className="bg-white rounded-3xl border p-6 space-y-4">
        <h2 className="text-lg font-black text-slate-900">Classes par pôle</h2>
        <p className="text-sm text-slate-500">
          École, collège, lycée… Vous pouvez supprimer un pôle inutile (ex. maintenance, réservé à la réservation de
          salles).
        </p>
        {Object.entries(config.classesByPole).map(([pole, classes]) => (
          <div key={pole} className="border rounded-2xl p-4 space-y-3">
            <div className="flex items-center justify-between gap-2">
              <p className="font-black text-sm text-slate-700 uppercase">{pole}</p>
              <button
                type="button"
                onClick={() => removePole(pole)}
                className="text-red-600 text-xs font-bold shrink-0 hover:underline"
              >
                Supprimer le pôle
              </button>
            </div>
            <div className="flex flex-wrap gap-2">
              {classes.map((cls) => (
                <span
                  key={cls}
                  className="inline-flex items-center gap-1 bg-slate-100 px-2 py-1 rounded-lg text-xs font-bold"
                >
                  {cls}
                  <button
                    type="button"
                    onClick={() => removeClassFromPole(pole, cls)}
                    className="text-red-500 hover:text-red-700 leading-none"
                    aria-label={`Supprimer ${cls}`}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
            <div className="flex gap-2">
              <input
                className="flex-1 border rounded-lg p-2 text-sm font-bold uppercase"
                placeholder="Nouvelle classe"
                value={newClassByPole[pole] || ""}
                onChange={(e) => setNewClassByPole({ ...newClassByPole, [pole]: e.target.value })}
                onKeyDown={(e) => e.key === "Enter" && addClassToPole(pole)}
              />
              <button type="button" onClick={() => addClassToPole(pole)} className="text-violet-600 font-bold text-sm px-3">
                + Classe
              </button>
            </div>
          </div>
        ))}
        <div className="flex gap-2 pt-2 border-t">
          <input
            className="flex-1 border rounded-xl p-3 text-sm font-bold uppercase"
            placeholder="Nouveau pôle"
            value={newPoleName}
            onChange={(e) => setNewPoleName(e.target.value)}
          />
          <button type="button" onClick={addPole} className="bg-slate-800 text-white px-4 py-2 rounded-xl font-bold text-sm">
            Ajouter pôle
          </button>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 pt-2">
          <div>
            <label className="block text-sm font-bold text-slate-600 mb-1">Heure début</label>
            <input
              type="number"
              min={6}
              max={12}
              className="w-full border rounded-xl p-3 text-sm font-bold"
              value={config.hoursStart}
              onChange={(e) => setConfig({ ...config, hoursStart: parseInt(e.target.value, 10) || 8 })}
            />
          </div>
          <div>
            <label className="block text-sm font-bold text-slate-600 mb-1">Heure fin</label>
            <input
              type="number"
              min={12}
              max={20}
              className="w-full border rounded-xl p-3 text-sm font-bold"
              value={config.hoursEnd}
              onChange={(e) => setConfig({ ...config, hoursEnd: parseInt(e.target.value, 10) || 17 })}
            />
          </div>
          <div>
            <label className="block text-sm font-bold text-slate-600 mb-1">Horizon (jours)</label>
            <input
              type="number"
              min={7}
              max={365}
              className="w-full border rounded-xl p-3 text-sm font-bold"
              value={config.bookingHorizonDays}
              onChange={(e) =>
                setConfig({ ...config, bookingHorizonDays: parseInt(e.target.value, 10) || 56 })
              }
            />
          </div>
        </div>
        <button
          type="button"
          disabled={saving}
          onClick={() => saveModuleConfig()}
          className="bg-violet-600 text-white px-6 py-2.5 rounded-xl font-bold disabled:opacity-50"
        >
          Enregistrer la configuration
        </button>
      </section>
    </div>
  );
}
