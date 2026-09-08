"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import dynamic from "next/dynamic";
import Link from "next/link";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import ModuleTabFallback from "@/app/components/module-chrome/ModuleTabFallback";
import ModuleTabNav from "@/app/components/module-chrome/ModuleTabNav";
import type { Establishment } from "@/app/lib/app-config-schemas";
import { EVENEMENTS_TOOLS_META, type EvenementToolId } from "@/app/lib/evenements-tools";
import {
  generatePortesOuvertesSlots,
  type PortesOuvertesDepartureIntervalMinutes,
  type PortesOuvertesVisitDurationMinutes,
} from "@/app/lib/portes-ouvertes-slots";
import {
  PORTES_OUVERTES_CYCLE_LABELS,
  type PortesOuvertesCycle,
} from "@/app/lib/portes-ouvertes-types";
import type { PortesOuvertesSlot, ToolboxConfig } from "@/app/lib/toolbox-types";

const RentreeEditor = dynamic(() => import("@/app/components/toolbox/RentreeEditor"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});
const FournituresEditor = dynamic(() => import("@/app/components/toolbox/FournituresEditor"), {
  ssr: false,
  loading: () => <ModuleTabFallback />,
});

type Tab = "overview" | EvenementToolId;

type PoAdminPayload = {
  enabled: boolean;
  title: string;
  intro: string;
  address: string;
  mapsUrl?: string;
  notifyEmail?: string;
  preinscriptionUrl?: string;
  followUpDelayMinutes: number;
  consentLabel: string;
  slots: PortesOuvertesSlot[];
  stats: Record<string, number>;
  registrationsCount: number;
  error?: string;
};

type CycleGridForm = {
  day: string;
  startTime: string;
  endTime: string;
  /** Rythme des départs (toutes les X minutes). */
  departureInterval: PortesOuvertesDepartureIntervalMinutes;
  /** Durée réelle de la visite (souvent ~1 h). */
  visitDuration: PortesOuvertesVisitDurationMinutes;
  maxPlaces: number;
};

function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <label className="flex items-center gap-3 cursor-pointer">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} className="h-4 w-4" />
      <span className="text-sm font-semibold text-slate-800">{label}</span>
    </label>
  );
}

function defaultGridDay(): string {
  const d = new Date();
  d.setDate(d.getDate() + 14);
  return d.toISOString().slice(0, 10);
}

function emptyCycleGrid(): CycleGridForm {
  return {
    day: defaultGridDay(),
    startTime: "08:30",
    endTime: "12:00",
    departureInterval: 15,
    visitDuration: 60,
    maxPlaces: 20,
  };
}

function applyPoResponse(j: PoAdminPayload): PoAdminPayload {
  return {
    enabled: Boolean(j.enabled),
    title: j.title || "Portes ouvertes",
    intro: j.intro || "",
    address: j.address || "",
    mapsUrl: j.mapsUrl || "",
    notifyEmail: j.notifyEmail || "",
    preinscriptionUrl: j.preinscriptionUrl || "",
    followUpDelayMinutes:
      typeof j.followUpDelayMinutes === "number" && j.followUpDelayMinutes > 0
        ? j.followUpDelayMinutes
        : 60,
    consentLabel: j.consentLabel || "",
    slots: Array.isArray(j.slots) ? j.slots : [],
    stats: j.stats && typeof j.stats === "object" ? j.stats : {},
    registrationsCount: typeof j.registrationsCount === "number" ? j.registrationsCount : 0,
  };
}

function toDatetimeLocalValue(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export default function EvenementsHubClient() {
  const [tab, setTab] = useState<Tab>("overview");
  const [config, setConfig] = useState<ToolboxConfig | null>(null);
  const [establishments, setEstablishments] = useState<Establishment[]>([]);
  const [publicOrigin, setPublicOrigin] = useState("");
  const [po, setPo] = useState<PoAdminPayload | null>(null);
  const [loading, setLoading] = useState(true);
  const [poLoading, setPoLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [cycleGrids, setCycleGrids] = useState<Partial<Record<PortesOuvertesCycle, CycleGridForm>>>(
    {},
  );

  const loadPo = useCallback(async () => {
    setPoLoading(true);
    try {
      const res = await fetch("/api/toolbox/portes-ouvertes", { cache: "no-store" });
      const j = (await res.json()) as PoAdminPayload;
      if (!res.ok) throw new Error(j.error || "Erreur portes ouvertes");
      setPo(applyPoResponse(j));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur portes ouvertes");
    } finally {
      setPoLoading(false);
    }
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/toolbox/config", { cache: "no-store" });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setConfig(j.config);
      setEstablishments(j.establishments || []);
      setPublicOrigin(typeof j.publicOrigin === "string" ? j.publicOrigin.replace(/\/$/, "") : "");
      await loadPo();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, [loadPo]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const t = new URLSearchParams(window.location.search).get("tab");
    if (t === "portes-ouvertes" || t === "rentree" || t === "secret-santa") {
      setTab(t);
    }
  }, []);

  useEffect(() => {
    if (tab === "portes-ouvertes" && !po && !poLoading) {
      void loadPo();
    }
  }, [tab, po, poLoading, loadPo]);

  const activeCycles = useMemo(() => {
    const found = new Set<PortesOuvertesCycle>();
    for (const e of establishments) {
      if (e.active === false) continue;
      if (e.kind === "ecole" || e.kind === "college" || e.kind === "lycee") {
        found.add(e.kind);
      }
    }
    const ordered = (["ecole", "college", "lycee"] as const).filter((c) => found.has(c));
    return ordered.length > 0 ? [...ordered] : (["ecole", "college", "lycee"] as PortesOuvertesCycle[]);
  }, [establishments]);

  const cycleLabel = useCallback(
    (cycle: PortesOuvertesCycle) => {
      const est = establishments.find((e) => e.kind === cycle && e.active !== false);
      return est?.label || PORTES_OUVERTES_CYCLE_LABELS[cycle];
    },
    [establishments],
  );

  function gridFor(cycle: PortesOuvertesCycle): CycleGridForm {
    return cycleGrids[cycle] || emptyCycleGrid();
  }

  function patchGrid(cycle: PortesOuvertesCycle, patch: Partial<CycleGridForm>) {
    setCycleGrids((prev) => ({
      ...prev,
      [cycle]: { ...(prev[cycle] || emptyCycleGrid()), ...patch },
    }));
  }

  async function putPo(body: Record<string, unknown>, successMsg?: string) {
    setSaving(true);
    setMsg(null);
    setError(null);
    try {
      const res = await fetch("/api/toolbox/portes-ouvertes", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      const j = (await res.json()) as PoAdminPayload & { success?: boolean };
      if (!res.ok) throw new Error(j.error || "Erreur");
      setPo(applyPoResponse(j));
      if (config) {
        setConfig({
          ...config,
          tools: {
            ...config.tools,
            "portes-ouvertes": {
              ...config.tools["portes-ouvertes"],
              enabled: Boolean(j.enabled),
              slots: [],
            },
          },
        });
      }
      if (successMsg) setMsg(successMsg);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  }

  async function saveToolbox() {
    if (!config) return;
    setSaving(true);
    setMsg(null);
    setError(null);
    try {
      const res = await fetch("/api/toolbox/config", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(config),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setConfig(j.config);
      setMsg("Configuration enregistrée.");
      await load();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  }

  async function savePortesOuvertesMeta() {
    if (!po) return;
    await putPo(
      {
        enabled: po.enabled,
        title: po.title,
        intro: po.intro,
        address: po.address,
        mapsUrl: po.mapsUrl || null,
        notifyEmail: po.notifyEmail || null,
        preinscriptionUrl: po.preinscriptionUrl || null,
        followUpDelayMinutes: po.followUpDelayMinutes,
        consentLabel: po.consentLabel,
      },
      "Portes ouvertes enregistrées (SQL).",
    );
  }

  async function save() {
    if (tab === "portes-ouvertes") {
      await savePortesOuvertesMeta();
      return;
    }
    await saveToolbox();
  }

  function patchTool<K extends keyof ToolboxConfig["tools"]>(
    key: K,
    patch: Partial<ToolboxConfig["tools"][K]>,
  ) {
    if (!config) return;
    setConfig({
      ...config,
      tools: {
        ...config.tools,
        [key]: { ...config.tools[key], ...patch },
      },
    });
  }

  function setFournituresActive(v: boolean) {
    if (!config) return;
    setConfig({
      ...config,
      tools: {
        ...config.tools,
        "simulateur-fournitures": { ...config.tools["simulateur-fournitures"], enabled: v },
        rentree: { ...config.tools.rentree, showSimulateurFournitures: v },
      },
    });
  }

  function patchPoLocal(patch: Partial<PoAdminPayload>) {
    setPo((prev) => (prev ? { ...prev, ...patch } : prev));
  }

  async function generateSlots(cycle: PortesOuvertesCycle, mode: "append" | "replace") {
    const g = gridFor(cycle);
    const generated = generatePortesOuvertesSlots({
      date: g.day,
      startTime: g.startTime,
      endTime: g.endTime,
      departureIntervalMinutes: g.departureInterval,
      visitDurationMinutes: g.visitDuration,
      maxPlaces: g.maxPlaces > 0 ? g.maxPlaces : undefined,
      cycle,
    });
    if (generated.length === 0) {
      setError("Impossible de générer des créneaux (vérifiez jour / horaires).");
      return;
    }
    const slotsWithCycle = generated.map((s) => ({ ...s, cycle }));
    if (mode === "append") {
      await putPo(
        { slotsAppend: slotsWithCycle },
        `${generated.length} créneau(x) ajouté(s) pour ${cycleLabel(cycle)}.`,
      );
    } else {
      await putPo(
        { slotsReplaceCycle: { cycle, slots: slotsWithCycle } },
        `${generated.length} créneau(x) — grille ${cycleLabel(cycle)} remplacée.`,
      );
    }
  }

  async function upsertSlot(slot: PortesOuvertesSlot & { cycle: PortesOuvertesCycle }) {
    await putPo({ slotUpsert: slot }, "Créneau mis à jour.");
  }

  async function deleteSlot(id: string) {
    await putPo({ slotDeleteId: id }, "Créneau supprimé.");
  }

  const slotsByCycle = useMemo(() => {
    const groups: Record<PortesOuvertesCycle, PortesOuvertesSlot[]> = {
      ecole: [],
      college: [],
      lycee: [],
    };
    for (const s of po?.slots || []) {
      if (s.cycle === "ecole" || s.cycle === "college" || s.cycle === "lycee") {
        groups[s.cycle].push(s);
      } else {
        // Créneaux legacy sans cycle : affichés sous le premier cycle actif
        groups[activeCycles[0] || "college"].push(s);
      }
    }
    for (const c of Object.keys(groups) as PortesOuvertesCycle[]) {
      groups[c].sort((a, b) => a.startAt.localeCompare(b.startAt));
    }
    return groups;
  }, [po?.slots, activeCycles]);

  if (loading || !config) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1280px]">
        <p className="text-slate-500">{loading ? "Chargement…" : "Configuration indisponible."}</p>
      </ModulePageShell>
    );
  }

  const poEnabled = po?.enabled ?? config.tools["portes-ouvertes"].enabled;

  return (
    <RequireOrgAdmin>
      <ModulePageShell maxWidthClass="max-w-[1280px]" className="space-y-6">
        <ModulePageHeader
          eyebrow="Établissement"
          title="Événements"
          description="Portes ouvertes, rentrée digitale et Secret Santa — configuration ici, plus dans la boîte à outils."
          actions={
            tab !== "overview" ? (
              <ModuleButton onClick={() => void save()} disabled={saving || (tab === "portes-ouvertes" && !po)}>
                {saving ? "Enregistrement…" : "Enregistrer"}
              </ModuleButton>
            ) : null
          }
        />

        {error ? (
          <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        ) : null}
        {msg ? (
          <p className="rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">
            {msg}
          </p>
        ) : null}

        <ModuleTabNav
          tabs={[
            { id: "overview", label: "Vue d’ensemble" },
            { id: "portes-ouvertes", label: "Portes ouvertes" },
            { id: "rentree", label: "Rentrée" },
            { id: "secret-santa", label: "Secret Santa" },
          ]}
          active={tab}
          onChange={(id) => {
            setTab(id);
            const url = new URL(window.location.href);
            if (id === "overview") url.searchParams.delete("tab");
            else url.searchParams.set("tab", id);
            window.history.replaceState({}, "", url.pathname + url.search);
          }}
        />

        {tab === "overview" ? (
          <div className="grid gap-4 sm:grid-cols-1 md:grid-cols-3">
            {EVENEMENTS_TOOLS_META.map((ev) => {
              const tool = config.tools[ev.id];
              const enabled =
                ev.id === "portes-ouvertes"
                  ? poEnabled
                  : "enabled" in tool
                    ? tool.enabled
                    : false;
              return (
                <article
                  key={ev.id}
                  className={`rounded-2xl border p-5 flex flex-col gap-3 ${ev.accent}`}
                >
                  <div className="flex items-start justify-between gap-2">
                    <h2 className="text-lg font-black leading-tight">{ev.title}</h2>
                    <span className="shrink-0 text-[10px] font-bold uppercase tracking-wide opacity-70">
                      {ev.season}
                    </span>
                  </div>
                  <p className="text-sm opacity-90 flex-1">{ev.description}</p>
                  <p className="text-[11px] font-bold opacity-70">
                    {enabled ? "Activé" : "Désactivé"}
                  </p>
                  <div className="flex flex-wrap gap-2 pt-1">
                    <button
                      type="button"
                      onClick={() => setTab(ev.id)}
                      className="inline-flex rounded-xl bg-slate-900 px-3 py-2 text-xs font-bold text-white hover:bg-slate-800"
                    >
                      Configurer
                    </button>
                    {ev.publicHref && enabled ? (
                      <Link
                        href={ev.publicHref}
                        className="inline-flex rounded-xl border border-current/20 bg-white/70 px-3 py-2 text-xs font-bold hover:bg-white"
                      >
                        Page publique
                      </Link>
                    ) : null}
                    {ev.id === "secret-santa" && enabled ? (
                      <Link
                        href="/etablissement/evenements/secret-santa"
                        className="inline-flex rounded-xl border border-current/20 bg-white/70 px-3 py-2 text-xs font-bold hover:bg-white"
                      >
                        Lancer le tirage
                      </Link>
                    ) : null}
                  </div>
                </article>
              );
            })}
          </div>
        ) : null}

        {tab === "rentree" ? (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-6">
            <Toggle
              checked={config.tools.rentree.enabled}
              onChange={(v) => patchTool("rentree", { enabled: v })}
              label="Publier la page publique /rentree (peut rester désactivée pendant la préparation dès juin)"
            />
            <Toggle
              checked={config.tools.rentree.showSimulateurTarifs}
              onChange={(v) => patchTool("rentree", { showSimulateurTarifs: v })}
              label="Afficher le lien simulateur tarifs sur la page rentrée"
            />
            <p className="text-xs text-slate-500 -mt-3">
              Barème et publication :{" "}
              <Link
                href="/etablissement/communication"
                className="font-semibold text-slate-700 underline"
              >
                Établissement → Communication
              </Link>
              {!config.tools["simulateur-tarifs"].enabled
                ? " (simulateur actuellement désactivé)."
                : "."}
            </p>

            <div className="border-t border-slate-100 pt-5 space-y-4">
              <h3 className="text-sm font-black uppercase tracking-wide text-emerald-800">
                Fournitures scolaires
              </h3>
              <p className="text-xs text-slate-500">
                Intégré à la rentrée digitale — listes par classe et page publique
                /simulateurFournitures.
              </p>
              <Toggle
                checked={
                  config.tools["simulateur-fournitures"].enabled &&
                  config.tools.rentree.showSimulateurFournitures
                }
                onChange={(v) => setFournituresActive(v)}
                label="Activer les fournitures (lien sur /rentree + page publique)"
              />
              <FournituresEditor
                config={config.tools["simulateur-fournitures"]}
                onChange={(patch) => patchTool("simulateur-fournitures", patch)}
              />
              {config.tools["simulateur-fournitures"].enabled ? (
                <a
                  href={`${publicOrigin}/simulateurFournitures`}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm font-bold text-emerald-700 underline break-all"
                >
                  Voir la page publique → {publicOrigin}/simulateurFournitures
                </a>
              ) : null}
            </div>

            <RentreeEditor
              rentree={config.tools.rentree}
              establishments={establishments}
              onChange={(patch) => patchTool("rentree", patch)}
              onPagesChange={(pages) => patchTool("rentree", { pages })}
            />
            <p className="text-xs text-slate-500">
              Page publique :{" "}
              <a href="/rentree" target="_blank" rel="noopener noreferrer" className="underline font-semibold">
                {publicOrigin || ""}/rentree
              </a>
            </p>
          </section>
        ) : null}

        {tab === "portes-ouvertes" ? (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-5">
            {poLoading && !po ? (
              <p className="text-sm text-slate-500">Chargement des créneaux SQL…</p>
            ) : null}
            {po ? (
              <>
                <Toggle
                  checked={po.enabled}
                  onChange={(v) => patchPoLocal({ enabled: v })}
                  label="Activer la page publique /portes-ouvertes"
                />
                <div className="grid gap-4 sm:grid-cols-2">
                  <label className="block">
                    <span className="text-xs font-bold text-slate-500 uppercase">Titre</span>
                    <input
                      className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                      value={po.title}
                      onChange={(e) => patchPoLocal({ title: e.target.value })}
                    />
                  </label>
                  <label className="block">
                    <span className="text-xs font-bold text-slate-500 uppercase">
                      E-mail notifications
                    </span>
                    <input
                      className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                      value={po.notifyEmail || ""}
                      onChange={(e) => patchPoLocal({ notifyEmail: e.target.value })}
                    />
                  </label>
                </div>
                <label className="block">
                  <span className="text-xs font-bold text-slate-500 uppercase">Introduction</span>
                  <textarea
                    className="mt-1 w-full rounded-xl border px-3 py-2 text-sm min-h-[80px]"
                    value={po.intro}
                    onChange={(e) => patchPoLocal({ intro: e.target.value })}
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold text-slate-500 uppercase">Adresse</span>
                  <input
                    className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                    value={po.address}
                    onChange={(e) => patchPoLocal({ address: e.target.value })}
                    placeholder="12 rue …, 75000 Paris"
                  />
                </label>
                <label className="block">
                  <span className="text-xs font-bold text-slate-500 uppercase">
                    Lien Google Maps (optionnel)
                  </span>
                  <input
                    className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                    value={po.mapsUrl || ""}
                    onChange={(e) => patchPoLocal({ mapsUrl: e.target.value })}
                  />
                </label>
                <div className="grid gap-4 sm:grid-cols-2">
                  <label className="block">
                    <span className="text-xs font-bold text-slate-500 uppercase">
                      Lien préinscription (mail de suivi)
                    </span>
                    <input
                      className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                      value={po.preinscriptionUrl || ""}
                      onChange={(e) => patchPoLocal({ preinscriptionUrl: e.target.value })}
                      placeholder="https://…"
                    />
                  </label>
                  <label className="block">
                    <span className="text-xs font-bold text-slate-500 uppercase">
                      Délai mail de suivi (min)
                    </span>
                    <input
                      type="number"
                      min={5}
                      max={1440}
                      className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                      value={po.followUpDelayMinutes}
                      onChange={(e) =>
                        patchPoLocal({
                          followUpDelayMinutes: Math.max(5, Number(e.target.value) || 60),
                        })
                      }
                    />
                  </label>
                </div>
                <label className="block">
                  <span className="text-xs font-bold text-slate-500 uppercase">
                    Libellé consentement
                  </span>
                  <textarea
                    className="mt-1 w-full rounded-xl border px-3 py-2 text-sm min-h-[60px]"
                    value={po.consentLabel}
                    onChange={(e) => patchPoLocal({ consentLabel: e.target.value })}
                  />
                </label>

                <div className="rounded-xl border border-violet-100 bg-violet-50/60 p-4 space-y-2">
                  <p className="text-sm text-violet-950">
                    Les créneaux sont stockés en SQL (par établissement / cycle). L’enregistrement
                    meta ne pousse plus les slots dans le JSON toolbox.
                  </p>
                  <a
                    href="/accueil/portes-ouvertes"
                    className="inline-block text-xs font-bold text-violet-800 underline"
                  >
                    Ouvrir la saisie Accueil →
                  </a>
                </div>

                {activeCycles.map((cycle) => {
                  const g = gridFor(cycle);
                  const cycleSlots = slotsByCycle[cycle];
                  return (
                    <div
                      key={cycle}
                      className="rounded-2xl border border-violet-100 bg-violet-50/40 p-4 space-y-4"
                    >
                      <div className="flex flex-wrap items-center justify-between gap-2">
                        <h3 className="font-bold text-violet-950">{cycleLabel(cycle)}</h3>
                        <span className="text-xs font-semibold text-violet-800">
                          {cycleSlots.length} créneau(x)
                        </span>
                      </div>
                      <p className="text-xs text-violet-900">
                        Départs toutes les 15 / 30 / 60 min, indépendamment de la durée de la visite
                        (souvent ~1 h). « Fin » = fermeture (fin de la dernière visite).
                      </p>
                      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-6">
                        <label className="block">
                          <span className="text-[11px] font-bold uppercase text-violet-800">
                            Jour
                          </span>
                          <input
                            type="date"
                            className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                            value={g.day}
                            onChange={(e) => patchGrid(cycle, { day: e.target.value })}
                          />
                        </label>
                        <label className="block">
                          <span className="text-[11px] font-bold uppercase text-violet-800">
                            Premier départ
                          </span>
                          <input
                            type="time"
                            className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                            value={g.startTime}
                            onChange={(e) => patchGrid(cycle, { startTime: e.target.value })}
                          />
                        </label>
                        <label className="block">
                          <span className="text-[11px] font-bold uppercase text-violet-800">
                            Fin (fermeture)
                          </span>
                          <input
                            type="time"
                            className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                            value={g.endTime}
                            onChange={(e) => patchGrid(cycle, { endTime: e.target.value })}
                          />
                        </label>
                        <label className="block">
                          <span className="text-[11px] font-bold uppercase text-violet-800">
                            Départ toutes les
                          </span>
                          <select
                            className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm font-semibold"
                            value={g.departureInterval}
                            onChange={(e) =>
                              patchGrid(cycle, {
                                departureInterval: Number(
                                  e.target.value,
                                ) as PortesOuvertesDepartureIntervalMinutes,
                              })
                            }
                          >
                            <option value={15}>15 min</option>
                            <option value={30}>30 min</option>
                            <option value={60}>1 h</option>
                          </select>
                        </label>
                        <label className="block">
                          <span className="text-[11px] font-bold uppercase text-violet-800">
                            Durée de la visite
                          </span>
                          <select
                            className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm font-semibold"
                            value={g.visitDuration}
                            onChange={(e) =>
                              patchGrid(cycle, {
                                visitDuration: Number(
                                  e.target.value,
                                ) as PortesOuvertesVisitDurationMinutes,
                              })
                            }
                          >
                            <option value={30}>30 min</option>
                            <option value={45}>45 min</option>
                            <option value={60}>1 h</option>
                            <option value={75}>1 h 15</option>
                            <option value={90}>1 h 30</option>
                          </select>
                        </label>
                        <label className="block">
                          <span className="text-[11px] font-bold uppercase text-violet-800">
                            Places / départ
                          </span>
                          <input
                            type="number"
                            min={1}
                            className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                            value={g.maxPlaces}
                            onChange={(e) =>
                              patchGrid(cycle, { maxPlaces: Number(e.target.value) || 0 })
                            }
                          />
                        </label>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        <button
                          type="button"
                          disabled={saving}
                          className="rounded-lg bg-violet-700 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
                          onClick={() => void generateSlots(cycle, "append")}
                        >
                          Ajouter la grille
                        </button>
                        <button
                          type="button"
                          disabled={saving}
                          className="rounded-lg border border-violet-300 bg-white px-3 py-1.5 text-xs font-bold text-violet-900 disabled:opacity-50"
                          onClick={() => void generateSlots(cycle, "replace")}
                        >
                          Remplacer les créneaux de ce cycle
                        </button>
                      </div>

                      {cycleSlots.length === 0 ? (
                        <p className="text-sm text-slate-500">Aucun créneau pour ce cycle.</p>
                      ) : (
                        <div className="space-y-3">
                          {cycleSlots.map((slot, idx) => (
                            <SlotEditorRow
                              key={slot.id}
                              slot={slot}
                              cycle={cycle}
                              index={idx}
                              registered={po.stats[slot.id] || 0}
                              saving={saving}
                              onDelete={() => void deleteSlot(slot.id)}
                              onSave={(next) => void upsertSlot({ ...next, cycle })}
                            />
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}

                <p className="text-sm text-slate-600">
                  Total inscriptions : <strong>{po.registrationsCount}</strong>
                </p>
                <a
                  href="/portes-ouvertes"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-sm font-bold text-violet-700 underline"
                >
                  Page publique →
                </a>
              </>
            ) : null}
          </section>
        ) : null}

        {tab === "secret-santa" ? (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-4">
            <Toggle
              checked={config.tools["secret-santa"].enabled}
              onChange={(v) => patchTool("secret-santa", { enabled: v })}
              label="Activer Secret Santa"
            />
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Titre</span>
              <input
                className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                value={config.tools["secret-santa"].title}
                onChange={(e) => patchTool("secret-santa", { title: e.target.value })}
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Budget indicatif</span>
              <input
                className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                value={config.tools["secret-santa"].budgetHint}
                onChange={(e) => patchTool("secret-santa", { budgetHint: e.target.value })}
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">
                Participants (un nom par ligne)
              </span>
              <textarea
                className="mt-1 w-full rounded-xl border px-3 py-2 text-sm min-h-[140px] font-mono"
                value={config.tools["secret-santa"].participantNames.join("\n")}
                onChange={(e) =>
                  patchTool("secret-santa", {
                    participantNames: e.target.value
                      .split(/\r?\n/)
                      .map((s) => s.trim())
                      .filter(Boolean),
                  })
                }
              />
            </label>
            <Link
              href="/etablissement/evenements/secret-santa"
              className="inline-block rounded-xl bg-red-600 px-4 py-2 text-sm font-bold text-white"
            >
              Lancer le tirage →
            </Link>
          </section>
        ) : null}

        <p className="text-xs text-slate-500">
          QR code et répartition de classes restent dans la{" "}
          <Link href="/toolbox" className="font-semibold text-slate-700 underline-offset-2 hover:underline">
            boîte à outils
          </Link>
          . Simulateur de tarifs :{" "}
          <Link
            href="/etablissement/communication"
            className="font-semibold text-slate-700 underline-offset-2 hover:underline"
          >
            Communication
          </Link>
          .
        </p>
      </ModulePageShell>
    </RequireOrgAdmin>
  );
}

function SlotEditorRow({
  slot,
  cycle,
  index,
  registered,
  saving,
  onDelete,
  onSave,
}: {
  slot: PortesOuvertesSlot;
  cycle: PortesOuvertesCycle;
  index: number;
  registered: number;
  saving: boolean;
  onDelete: () => void;
  onSave: (slot: PortesOuvertesSlot & { cycle: PortesOuvertesCycle }) => void;
}) {
  const [label, setLabel] = useState(slot.label);
  const [maxPlaces, setMaxPlaces] = useState(slot.maxPlaces ?? 0);
  const [startLocal, setStartLocal] = useState(toDatetimeLocalValue(slot.startAt));
  const [endLocal, setEndLocal] = useState(toDatetimeLocalValue(slot.endAt));

  useEffect(() => {
    setLabel(slot.label);
    setMaxPlaces(slot.maxPlaces ?? 0);
    setStartLocal(toDatetimeLocalValue(slot.startAt));
    setEndLocal(toDatetimeLocalValue(slot.endAt));
  }, [slot.id, slot.label, slot.maxPlaces, slot.startAt, slot.endAt]);

  return (
    <div className="rounded-xl border border-slate-100 bg-white p-4 space-y-3">
      <div className="flex justify-between items-center">
        <span className="text-xs font-black uppercase text-slate-500">Créneau {index + 1}</span>
        <button
          type="button"
          disabled={saving}
          onClick={onDelete}
          className="text-xs text-rose-600 font-bold disabled:opacity-50"
        >
          Supprimer
        </button>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <input
          className="rounded-lg border px-3 py-2 text-sm"
          placeholder="Libellé"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
        />
        <input
          type="number"
          className="rounded-lg border px-3 py-2 text-sm"
          placeholder="Places max"
          value={maxPlaces || ""}
          onChange={(e) => setMaxPlaces(Number(e.target.value) || 0)}
        />
        <input
          type="datetime-local"
          className="rounded-lg border px-3 py-2 text-sm"
          value={startLocal}
          onChange={(e) => setStartLocal(e.target.value)}
        />
        <input
          type="datetime-local"
          className="rounded-lg border px-3 py-2 text-sm"
          value={endLocal}
          onChange={(e) => setEndLocal(e.target.value)}
        />
      </div>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-xs text-slate-500">
          Inscrits : {registered}
          {maxPlaces > 0 ? ` — plafond ${maxPlaces}` : ""}
        </p>
        <button
          type="button"
          disabled={saving}
          className="rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
          onClick={() => {
            const startAt = new Date(startLocal).toISOString();
            const endAt = new Date(endLocal).toISOString();
            onSave({
              id: slot.id,
              label: label.trim() || slot.label,
              startAt,
              endAt,
              maxPlaces: maxPlaces > 0 ? maxPlaces : undefined,
              cycle,
            });
          }}
        >
          Enregistrer ce créneau
        </button>
      </div>
    </div>
  );
}
