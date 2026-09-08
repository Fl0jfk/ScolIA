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
  formatParisHm,
  getParisParts,
  parisDateKey,
  parseParisDateTime,
} from "@/app/lib/paris-time";
import {
  generatePortesOuvertesSlots,
  type PortesOuvertesDepartureIntervalMinutes,
  type PortesOuvertesVisitDurationMinutes,
} from "@/app/lib/portes-ouvertes-slots";
import {
  PORTES_OUVERTES_CYCLE_LABELS,
  PORTES_OUVERTES_MAX_AMBASSADEURS,
  PORTES_OUVERTES_MAX_ENCADRANTS,
  type PortesOuvertesCycle,
  type PortesOuvertesStaffRole,
  type PortesOuvertesStaffRow,
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
  staff: PortesOuvertesStaffRow[];
  stats: Record<string, number>;
  registrationsCount: number;
  /** Snapshots d’inscriptions (pour détecter / restaurer une date écrasée). */
  registrationSlotSnapshots?: Array<{
    slotId: string;
    slotLabel?: string;
    slotStartAt?: string;
    slotEndAt?: string;
    cycle?: PortesOuvertesCycle;
  }>;
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
  // Pas de date inventée (+14 j) : ça a déjà écrasé un vrai jour (21/11 → 22/09).
  return "";
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

function formatDayFr(dayKey: string): string {
  const d = parseParisDateTime(dayKey, "12:00");
  if (!d) return dayKey;
  return d.toLocaleDateString("fr-FR", {
    timeZone: "Europe/Paris",
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}

function toDatetimeLocalValue(iso: string): string {
  const p = getParisParts(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${p.year}-${pad(p.month)}-${pad(p.day)}T${pad(p.hour)}:${pad(p.minute)}`;
}

function datetimeLocalToIso(local: string): string | null {
  const raw = local.trim();
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(raw);
  if (!m) return null;
  const d = parseParisDateTime(m[1], m[2]);
  return d ? d.toISOString() : null;
}

function gridFromSlots(slots: PortesOuvertesSlot[]): CycleGridForm {
  const base = emptyCycleGrid();
  if (!slots.length) return base;
  const sorted = [...slots].sort((a, b) => a.startAt.localeCompare(b.startAt));
  const first = sorted[0];
  const last = sorted[sorted.length - 1];
  return {
    ...base,
    day: parisDateKey(first.startAt),
    startTime: formatParisHm(first.startAt),
    endTime: formatParisHm(last.endAt),
    maxPlaces: first.maxPlaces && first.maxPlaces > 0 ? first.maxPlaces : base.maxPlaces,
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
    staff: Array.isArray(j.staff) ? j.staff : [],
    stats: j.stats && typeof j.stats === "object" ? j.stats : {},
    registrationsCount: typeof j.registrationsCount === "number" ? j.registrationsCount : 0,
    registrationSlotSnapshots: Array.isArray(j.registrationSlotSnapshots)
      ? j.registrationSlotSnapshots
      : [],
  };
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

  /** Préremplit le jour / horaires depuis les créneaux déjà en base (évite une date inventée). */
  useEffect(() => {
    if (!po?.slots?.length) return;
    const trapPlus14 = (() => {
      const d = new Date();
      d.setDate(d.getDate() + 14);
      return parisDateKey(d);
    })();
    setCycleGrids((prev) => {
      const next = { ...prev };
      let changed = false;
      for (const cycle of ["ecole", "college", "lycee"] as const) {
        const cycleSlots = po.slots.filter((s) => s.cycle === cycle);
        if (!cycleSlots.length) continue;
        const fromSlots = gridFromSlots(cycleSlots);
        const current = prev[cycle];
        if (
          current &&
          current.day === fromSlots.day &&
          current.startTime === fromSlots.startTime &&
          current.endTime === fromSlots.endTime
        ) {
          continue;
        }
        // Sync depuis la BDD sauf si l’utilisateur a déjà saisi un autre jour volontairement.
        const currentDay = current?.day || "";
        const userTypedOtherDay =
          Boolean(currentDay) &&
          currentDay !== fromSlots.day &&
          currentDay !== trapPlus14;
        if (userTypedOtherDay) continue;
        next[cycle] = {
          ...(current || emptyCycleGrid()),
          day: fromSlots.day,
          startTime: fromSlots.startTime,
          endTime: fromSlots.endTime,
          maxPlaces: fromSlots.maxPlaces,
        };
        changed = true;
      }
      return changed ? next : prev;
    });
  }, [po?.slots]);

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
        address: po.address ?? "",
        // Envoyer la chaîne telle quelle (pas `|| null`) pour ne pas effacer à tort.
        mapsUrl: (po.mapsUrl ?? "").trim(),
        notifyEmail: (po.notifyEmail ?? "").trim(),
        preinscriptionUrl: (po.preinscriptionUrl ?? "").trim(),
        followUpDelayMinutes: po.followUpDelayMinutes,
        consentLabel: po.consentLabel,
      },
      "Portes ouvertes enregistrées.",
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
    if (!/^\d{4}-\d{2}-\d{2}$/.test(g.day)) {
      setError("Indiquez d’abord le jour de la grille (ex. 21/11/2026) avant d’ajouter ou remplacer.");
      return;
    }
    const dayLabel = formatDayFr(g.day);
    const existing = (po?.slots || []).filter((s) => s.cycle === cycle);
    const existingDays = [
      ...new Set(existing.map((s) => parisDateKey(s.startAt))),
    ].sort();

    if (mode === "replace" && existing.length > 0) {
      const ok = window.confirm(
        `ATTENTION — Remplacer TOUS les créneaux « ${cycleLabel(cycle)} » ?\n\n` +
          `Jour de la nouvelle grille : ${dayLabel} (${g.day})\n` +
          (existingDays.length
            ? `Jours actuellement en base : ${existingDays.map(formatDayFr).join(", ")}\n\n`
            : "\n") +
          "Les inscriptions liées aux anciens créneaux restent en historique, " +
          "mais les créneaux eux-mêmes (et leur staffing) seront effacés.\n\n" +
          "Si vous vouliez seulement enregistrer adresse / Maps / délai : utilisez le bouton Enregistrer en haut, PAS celui-ci.",
      );
      if (!ok) return;
    }

    if (mode === "append" && existingDays.length > 0 && !existingDays.includes(g.day)) {
      const ok = window.confirm(
        `Ajouter une grille le ${dayLabel} (${g.day}) ?\n\n` +
          `Des créneaux existent déjà pour : ${existingDays.map(formatDayFr).join(", ")}.\n` +
          "Vérifiez bien le champ « Jour » avant de confirmer.",
      );
      if (!ok) return;
    }

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
        `${generated.length} créneau(x) ajouté(s) le ${dayLabel} (${cycleLabel(cycle)}).`,
      );
    } else {
      await putPo(
        { slotsReplaceCycle: { cycle, slots: slotsWithCycle } },
        `${generated.length} créneau(x) — grille ${cycleLabel(cycle)} remplacée par le ${dayLabel}.`,
      );
    }
  }

  async function restoreSlotsFromSnapshots() {
    if (!po?.registrationSlotSnapshots?.length) return;
    const existingIds = new Set((po.slots || []).map((s) => s.id));
    const existingDays = new Set((po.slots || []).map((s) => parisDateKey(s.startAt)));
    const toRestore: Array<PortesOuvertesSlot & { cycle: PortesOuvertesCycle }> = [];
    const seen = new Set<string>();
    for (const snap of po.registrationSlotSnapshots) {
      if (!snap.slotId || !snap.slotStartAt || !snap.slotEndAt) continue;
      if (existingIds.has(snap.slotId) || seen.has(snap.slotId)) continue;
      const day = parisDateKey(snap.slotStartAt);
      if (existingDays.has(day)) continue;
      const cycle =
        snap.cycle === "ecole" || snap.cycle === "college" || snap.cycle === "lycee"
          ? snap.cycle
          : (activeCycles[0] || "college");
      seen.add(snap.slotId);
      toRestore.push({
        id: snap.slotId,
        label: snap.slotLabel || formatParisHm(snap.slotStartAt),
        startAt: snap.slotStartAt,
        endAt: snap.slotEndAt,
        cycle,
      });
    }
    if (!toRestore.length) {
      setMsg("Aucun créneau manquant à restaurer depuis les inscriptions.");
      return;
    }
    const days = [...new Set(toRestore.map((s) => parisDateKey(s.startAt)))].map(formatDayFr);
    const ok = window.confirm(
      `Restaurer ${toRestore.length} créneau(x) depuis les inscriptions ?\n\nJour(s) : ${days.join(", ")}\n\n` +
        "Les créneaux actuels (ex. 22 septembre) ne sont pas effacés — uniquement les créneaux manquants sont réajoutés.",
    );
    if (!ok) return;
    await putPo(
      { slotsAppend: toRestore },
      `${toRestore.length} créneau(x) restauré(s) (${days.join(", ")}).`,
    );
  }

  async function upsertSlot(slot: PortesOuvertesSlot & { cycle: PortesOuvertesCycle }) {
    await putPo({ slotUpsert: slot }, "Créneau mis à jour.");
  }

  async function deleteSlot(id: string) {
    await putPo({ slotDeleteId: id }, "Créneau supprimé.");
  }

  async function addSlotStaff(input: {
    slotId: string;
    role: PortesOuvertesStaffRole;
    refId: string;
    displayName: string;
    meta?: Record<string, string>;
  }) {
    await putPo({ staffAdd: input }, `${input.displayName} ajouté(e) au créneau.`);
  }

  async function removeSlotStaff(staffId: string) {
    await putPo({ staffDeleteId: staffId }, "Personne retirée du créneau.");
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

  const orphanRegistrationDays = useMemo(() => {
    const slotDays = new Set((po?.slots || []).map((s) => parisDateKey(s.startAt)));
    const orphan = new Set<string>();
    for (const snap of po?.registrationSlotSnapshots || []) {
      if (!snap.slotStartAt) continue;
      const day = parisDateKey(snap.slotStartAt);
      if (!slotDays.has(day)) orphan.add(day);
    }
    return [...orphan].sort();
  }, [po?.slots, po?.registrationSlotSnapshots]);

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
                {orphanRegistrationDays.length > 0 ? (
                  <div className="rounded-xl border border-amber-300 bg-amber-50 px-4 py-3 space-y-2">
                    <p className="text-sm font-semibold text-amber-950">
                      Des inscriptions référencent des jours absents des créneaux actuels :{" "}
                      {orphanRegistrationDays.map(formatDayFr).join(", ")}.
                    </p>
                    <p className="text-xs text-amber-900">
                      Probable écrasement par une régénération (ex. jour par défaut au 22 septembre).
                      Vous pouvez restaurer les créneaux manquants depuis les snapshots d’inscription.
                    </p>
                    <button
                      type="button"
                      disabled={saving}
                      className="rounded-lg bg-amber-800 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
                      onClick={() => void restoreSlotsFromSnapshots()}
                    >
                      Restaurer les créneaux manquants
                    </button>
                  </div>
                ) : null}
                <Toggle
                  checked={po.enabled}
                  onChange={(v) => patchPoLocal({ enabled: v })}
                  label="Activer les portes ouvertes (page publique + planning Accueil pour direction, administratif et accueil)"
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

                <div className="rounded-xl border border-violet-100 bg-violet-50/60 p-4 space-y-2">
                  <p className="text-sm text-violet-950">
                    Les créneaux sont stockés en SQL (par établissement / cycle). L’enregistrement
                    meta ne pousse plus les slots dans le JSON toolbox.
                  </p>
                  <a
                    href="/accueil/portes-ouvertes"
                    className="inline-block text-xs font-bold text-violet-800 underline"
                  >
                    Planning Accueil (tableur du jour) →
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
                            Jour {g.day ? `(${formatDayFr(g.day)})` : "(à choisir)"}
                          </span>
                          <input
                            type="date"
                            required
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
                          Ajouter la grille du {formatDayFr(g.day)}
                        </button>
                        <button
                          type="button"
                          disabled={saving}
                          className="rounded-lg border border-rose-300 bg-white px-3 py-1.5 text-xs font-bold text-rose-800 disabled:opacity-50"
                          onClick={() => void generateSlots(cycle, "replace")}
                        >
                          Remplacer par le {formatDayFr(g.day)} (danger)
                        </button>
                      </div>
                      <p className="text-[11px] text-violet-800">
                        Vérifiez le champ « Jour » ci-dessus avant d’ajouter ou remplacer. L’enregistrement
                        adresse / Maps / délai se fait uniquement via le bouton Enregistrer en haut de page.
                      </p>

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
                              staff={(po.staff || []).filter((x) => x.slotId === slot.id)}
                              saving={saving}
                              onDelete={() => void deleteSlot(slot.id)}
                              onSave={(next) => void upsertSlot({ ...next, cycle })}
                              onAddStaff={(person) => void addSlotStaff(person)}
                              onRemoveStaff={(staffId) => void removeSlotStaff(staffId)}
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
  staff,
  saving,
  onDelete,
  onSave,
  onAddStaff,
  onRemoveStaff,
}: {
  slot: PortesOuvertesSlot;
  cycle: PortesOuvertesCycle;
  index: number;
  registered: number;
  staff: PortesOuvertesStaffRow[];
  saving: boolean;
  onDelete: () => void;
  onSave: (slot: PortesOuvertesSlot & { cycle: PortesOuvertesCycle }) => void;
  onAddStaff: (input: {
    slotId: string;
    role: PortesOuvertesStaffRole;
    refId: string;
    displayName: string;
    meta?: Record<string, string>;
  }) => void;
  onRemoveStaff: (staffId: string) => void;
}) {
  const [label, setLabel] = useState(slot.label);
  const [maxPlaces, setMaxPlaces] = useState(slot.maxPlaces ?? 0);
  const [startLocal, setStartLocal] = useState(toDatetimeLocalValue(slot.startAt));
  const [endLocal, setEndLocal] = useState(toDatetimeLocalValue(slot.endAt));
  const [staffOpen, setStaffOpen] = useState(false);

  useEffect(() => {
    setLabel(slot.label);
    setMaxPlaces(slot.maxPlaces ?? 0);
    setStartLocal(toDatetimeLocalValue(slot.startAt));
    setEndLocal(toDatetimeLocalValue(slot.endAt));
  }, [slot.id, slot.label, slot.maxPlaces, slot.startAt, slot.endAt]);

  const ambassadeurs = staff.filter((s) => s.role === "ambassadeur");
  const encadrants = staff.filter((s) => s.role === "enseignant" || s.role === "personnel");

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
          {" · "}
          Encadrants {encadrants.length}/{PORTES_OUVERTES_MAX_ENCADRANTS}
          {" · "}
          Ambassadeurs {ambassadeurs.length}/{PORTES_OUVERTES_MAX_AMBASSADEURS}
        </p>
        <div className="flex flex-wrap gap-2">
          <button
            type="button"
            className="rounded-lg border border-violet-200 bg-violet-50 px-3 py-1.5 text-xs font-bold text-violet-900"
            onClick={() => setStaffOpen((v) => !v)}
          >
            {staffOpen ? "Masquer l’équipe" : "Équipe du créneau"}
          </button>
          <button
            type="button"
            disabled={saving}
            className="rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-bold text-white disabled:opacity-50"
            onClick={() => {
              const startAt = datetimeLocalToIso(startLocal);
              const endAt = datetimeLocalToIso(endLocal);
              if (!startAt || !endAt) {
                window.alert("Horaires invalides (fuseau Paris).");
                return;
              }
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

      {staffOpen ? (
        <div className="rounded-lg border border-violet-100 bg-violet-50/50 p-3 space-y-3">
          <p className="text-xs text-violet-900">
            Typiquement 1–2 encadrants (professeur ou personnel OGEC) et 2 élèves ambassadeurs.
          </p>
          <div className="grid gap-3 lg:grid-cols-3">
            <AdminStaffSearchPicker
              kind="enseignant"
              label="Professeur"
              disabled={saving || encadrants.length >= PORTES_OUVERTES_MAX_ENCADRANTS}
              onPick={(hit) =>
                onAddStaff({
                  slotId: slot.id,
                  role: "enseignant",
                  refId: hit.refId,
                  displayName: hit.displayName,
                  meta: hit.meta,
                })
              }
            />
            <AdminStaffSearchPicker
              kind="personnel"
              label="Personnel OGEC"
              disabled={saving || encadrants.length >= PORTES_OUVERTES_MAX_ENCADRANTS}
              onPick={(hit) =>
                onAddStaff({
                  slotId: slot.id,
                  role: "personnel",
                  refId: hit.refId,
                  displayName: hit.displayName,
                  meta: hit.meta,
                })
              }
            />
            <AdminStaffSearchPicker
              kind="eleve"
              label="Élève ambassadeur"
              disabled={saving || ambassadeurs.length >= PORTES_OUVERTES_MAX_AMBASSADEURS}
              onPick={(hit) =>
                onAddStaff({
                  slotId: slot.id,
                  role: "ambassadeur",
                  refId: hit.refId,
                  displayName: hit.displayName,
                  meta: hit.meta,
                })
              }
            />
          </div>
          {staff.length === 0 ? (
            <p className="text-sm text-slate-500">Personne assignée pour l’instant.</p>
          ) : (
            <ul className="divide-y divide-violet-100 rounded-lg border border-violet-100 bg-white">
              {staff.map((row) => (
                <li
                  key={row.id}
                  className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm"
                >
                  <div>
                    <span className="font-semibold text-slate-900">{row.displayName}</span>
                    <span className="ml-2 text-xs font-semibold text-violet-700">
                      {row.role === "ambassadeur"
                        ? "Ambassadeur"
                        : row.role === "enseignant"
                          ? "Professeur"
                          : "Personnel OGEC"}
                    </span>
                    {row.meta?.classe ? (
                      <span className="ml-2 text-xs text-slate-500">{row.meta.classe}</span>
                    ) : null}
                  </div>
                  <button
                    type="button"
                    disabled={saving}
                    className="text-xs font-bold text-rose-600 disabled:opacity-50"
                    onClick={() => onRemoveStaff(row.id)}
                  >
                    Retirer
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      ) : null}
    </div>
  );
}

type StaffSearchHit = {
  refId: string;
  displayName: string;
  meta?: Record<string, string>;
};

function AdminStaffSearchPicker({
  kind,
  label,
  disabled,
  onPick,
}: {
  kind: "eleve" | "enseignant" | "personnel";
  label: string;
  disabled: boolean;
  onPick: (hit: StaffSearchHit) => void;
}) {
  const [q, setQ] = useState("");
  const [results, setResults] = useState<StaffSearchHit[]>([]);
  const [searching, setSearching] = useState(false);

  useEffect(() => {
    const needle = q.trim();
    if (needle.length < 2 || disabled) {
      setResults([]);
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      setSearching(true);
      void fetch(
        `/api/toolbox/portes-ouvertes/search?kind=${encodeURIComponent(kind)}&q=${encodeURIComponent(needle)}`,
        { cache: "no-store" },
      )
        .then(async (res) => {
          const data = (await res.json()) as { results?: StaffSearchHit[]; error?: string };
          if (!res.ok) throw new Error(data.error || "Recherche impossible");
          if (!cancelled) setResults(Array.isArray(data.results) ? data.results : []);
        })
        .catch(() => {
          if (!cancelled) setResults([]);
        })
        .finally(() => {
          if (!cancelled) setSearching(false);
        });
    }, 250);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [q, kind, disabled]);

  return (
    <div className="space-y-1.5">
      <label className="block text-[11px] font-bold uppercase text-violet-800">{label}</label>
      <input
        type="search"
        disabled={disabled}
        placeholder={disabled ? "Plafond atteint" : "Rechercher (2 lettres min.)"}
        className="w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm disabled:opacity-50"
        value={q}
        onChange={(e) => setQ(e.target.value)}
      />
      {searching ? <p className="text-[11px] text-slate-500">Recherche…</p> : null}
      {results.length > 0 ? (
        <ul className="max-h-36 overflow-auto rounded-lg border border-violet-100 bg-white text-sm">
          {results.map((hit) => (
            <li key={`${hit.refId}-${hit.displayName}`}>
              <button
                type="button"
                disabled={disabled}
                className="w-full px-3 py-1.5 text-left hover:bg-violet-50 disabled:opacity-50"
                onClick={() => {
                  onPick(hit);
                  setQ("");
                  setResults([]);
                }}
              >
                <span className="font-semibold text-slate-900">{hit.displayName}</span>
                {hit.meta?.classe ? (
                  <span className="ml-2 text-xs text-slate-500">{hit.meta.classe}</span>
                ) : hit.meta?.jobTitle ? (
                  <span className="ml-2 text-xs text-slate-500">{hit.meta.jobTitle}</span>
                ) : null}
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
