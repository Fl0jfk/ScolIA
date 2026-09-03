"use client";

import { useCallback, useEffect, useState } from "react";
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
  type PortesOuvertesSlotIntervalMinutes,
} from "@/app/lib/portes-ouvertes-slots";
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

function emptySlot(): PortesOuvertesSlot {
  const d = new Date();
  d.setDate(d.getDate() + 14);
  d.setHours(10, 0, 0, 0);
  const end = new Date(d);
  end.setHours(12, 0, 0, 0);
  return {
    id: `slot-${Date.now()}`,
    label: "Matin",
    startAt: d.toISOString(),
    endAt: end.toISOString(),
    maxPlaces: 30,
  };
}

export default function EvenementsHubClient() {
  const [tab, setTab] = useState<Tab>("overview");
  const [config, setConfig] = useState<ToolboxConfig | null>(null);
  const [establishments, setEstablishments] = useState<Establishment[]>([]);
  const [publicOrigin, setPublicOrigin] = useState("");
  const [stats, setStats] = useState<Record<string, number>>({});
  const [regCount, setRegCount] = useState(0);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [slotDay, setSlotDay] = useState(() => {
    const d = new Date();
    d.setDate(d.getDate() + 14);
    return d.toISOString().slice(0, 10);
  });
  const [slotStartTime, setSlotStartTime] = useState("08:30");
  const [slotEndTime, setSlotEndTime] = useState("12:00");
  const [slotInterval, setSlotInterval] = useState<PortesOuvertesSlotIntervalMinutes>(30);
  const [slotMaxPlaces, setSlotMaxPlaces] = useState(20);

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
      setStats(j.portesOuvertesStats || {});
      setRegCount(j.registrationsCount || 0);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    const t = new URLSearchParams(window.location.search).get("tab");
    if (t === "portes-ouvertes" || t === "rentree" || t === "secret-santa") {
      setTab(t);
    }
  }, []);

  async function save() {
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

  if (loading || !config) {
    return (
      <ModulePageShell maxWidthClass="max-w-[1280px]">
        <p className="text-slate-500">{loading ? "Chargement…" : "Configuration indisponible."}</p>
      </ModulePageShell>
    );
  }

  const po = config.tools["portes-ouvertes"];

  return (
    <RequireOrgAdmin>
      <ModulePageShell maxWidthClass="max-w-[1280px]" className="space-y-6">
        <ModulePageHeader
          eyebrow="Établissement"
          title="Événements"
          description="Portes ouvertes, rentrée digitale et Secret Santa — configuration ici, plus dans la boîte à outils."
          actions={
            tab !== "overview" ? (
              <ModuleButton onClick={() => void save()} disabled={saving}>
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
              const enabled = "enabled" in tool ? tool.enabled : false;
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
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-4">
            <Toggle
              checked={po.enabled}
              onChange={(v) => patchTool("portes-ouvertes", { enabled: v })}
              label="Activer la page publique /portes-ouvertes"
            />
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">Titre</span>
                <input
                  className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                  value={po.title}
                  onChange={(e) => patchTool("portes-ouvertes", { title: e.target.value })}
                />
              </label>
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">E-mail notifications</span>
                <input
                  className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                  value={po.notifyEmail || ""}
                  onChange={(e) => patchTool("portes-ouvertes", { notifyEmail: e.target.value })}
                />
              </label>
            </div>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Introduction</span>
              <textarea
                className="mt-1 w-full rounded-xl border px-3 py-2 text-sm min-h-[80px]"
                value={po.intro}
                onChange={(e) => patchTool("portes-ouvertes", { intro: e.target.value })}
              />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Adresse</span>
              <input
                className="mt-1 w-full rounded-xl border px-3 py-2 text-sm"
                value={po.address}
                onChange={(e) => patchTool("portes-ouvertes", { address: e.target.value })}
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
                onChange={(e) => patchTool("portes-ouvertes", { mapsUrl: e.target.value })}
              />
            </label>

            <div className="flex items-center justify-between">
              <h3 className="font-bold text-slate-900">Créneaux</h3>
              <button
                type="button"
                onClick={() => patchTool("portes-ouvertes", { slots: [...po.slots, emptySlot()] })}
                className="rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-bold text-white"
              >
                + Créneau manuel
              </button>
            </div>

            <div className="rounded-xl border border-violet-100 bg-violet-50/60 p-4 space-y-3">
              <p className="text-sm font-semibold text-violet-950">
                Générer une journée (quart d&apos;heure / demi-heure / heure)
              </p>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-5">
                <label className="block">
                  <span className="text-[11px] font-bold uppercase text-violet-800">Jour</span>
                  <input
                    type="date"
                    className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                    value={slotDay}
                    onChange={(e) => setSlotDay(e.target.value)}
                  />
                </label>
                <label className="block">
                  <span className="text-[11px] font-bold uppercase text-violet-800">Début</span>
                  <input
                    type="time"
                    className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                    value={slotStartTime}
                    onChange={(e) => setSlotStartTime(e.target.value)}
                  />
                </label>
                <label className="block">
                  <span className="text-[11px] font-bold uppercase text-violet-800">Fin</span>
                  <input
                    type="time"
                    className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                    value={slotEndTime}
                    onChange={(e) => setSlotEndTime(e.target.value)}
                  />
                </label>
                <label className="block">
                  <span className="text-[11px] font-bold uppercase text-violet-800">Pas</span>
                  <select
                    className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm font-semibold"
                    value={slotInterval}
                    onChange={(e) =>
                      setSlotInterval(Number(e.target.value) as PortesOuvertesSlotIntervalMinutes)
                    }
                  >
                    <option value={15}>15 min</option>
                    <option value={30}>30 min</option>
                    <option value={60}>1 h</option>
                  </select>
                </label>
                <label className="block">
                  <span className="text-[11px] font-bold uppercase text-violet-800">Places / créneau</span>
                  <input
                    type="number"
                    min={1}
                    className="mt-1 w-full rounded-lg border border-violet-200 bg-white px-3 py-2 text-sm"
                    value={slotMaxPlaces}
                    onChange={(e) => setSlotMaxPlaces(Number(e.target.value) || 0)}
                  />
                </label>
              </div>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className="rounded-lg bg-violet-700 px-3 py-1.5 text-xs font-bold text-white"
                  onClick={() => {
                    const generated = generatePortesOuvertesSlots({
                      date: slotDay,
                      startTime: slotStartTime,
                      endTime: slotEndTime,
                      intervalMinutes: slotInterval,
                      maxPlaces: slotMaxPlaces > 0 ? slotMaxPlaces : undefined,
                    });
                    if (generated.length === 0) {
                      setError("Impossible de générer des créneaux (vérifiez jour / horaires).");
                      return;
                    }
                    setError(null);
                    patchTool("portes-ouvertes", { slots: [...po.slots, ...generated] });
                    setMsg(`${generated.length} créneau(x) ajouté(s). Pensez à Enregistrer.`);
                  }}
                >
                  Ajouter la grille
                </button>
                <button
                  type="button"
                  className="rounded-lg border border-violet-300 bg-white px-3 py-1.5 text-xs font-bold text-violet-900"
                  onClick={() => {
                    const generated = generatePortesOuvertesSlots({
                      date: slotDay,
                      startTime: slotStartTime,
                      endTime: slotEndTime,
                      intervalMinutes: slotInterval,
                      maxPlaces: slotMaxPlaces > 0 ? slotMaxPlaces : undefined,
                    });
                    if (generated.length === 0) {
                      setError("Impossible de générer des créneaux (vérifiez jour / horaires).");
                      return;
                    }
                    setError(null);
                    patchTool("portes-ouvertes", { slots: generated });
                    setMsg(`${generated.length} créneau(x) — grille remplacée. Pensez à Enregistrer.`);
                  }}
                >
                  Remplacer tous les créneaux
                </button>
              </div>
              <p className="text-xs text-violet-800">
                Exemple : 8 h 30 → 12 h par 30 min crée 8 h 30–9 h, 9 h–9 h 30, etc. La page Accueil
                consomme ces créneaux (même si la page publique est désactivée).
              </p>
              <a
                href="/accueil/portes-ouvertes"
                className="inline-block text-xs font-bold text-violet-800 underline"
              >
                Ouvrir la saisie Accueil →
              </a>
            </div>

            {po.slots.length === 0 ? (
              <p className="text-sm text-slate-500">
                Ajoutez au moins un créneau pour ouvrir les inscriptions.
              </p>
            ) : null}
            {po.slots.map((slot, idx) => (
              <div key={slot.id} className="rounded-xl border border-slate-100 bg-slate-50 p-4 space-y-3">
                <div className="flex justify-between items-center">
                  <span className="text-xs font-black uppercase text-slate-500">Créneau {idx + 1}</span>
                  <button
                    type="button"
                    onClick={() =>
                      patchTool("portes-ouvertes", {
                        slots: po.slots.filter((s) => s.id !== slot.id),
                      })
                    }
                    className="text-xs text-rose-600 font-bold"
                  >
                    Supprimer
                  </button>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <input
                    className="rounded-lg border px-3 py-2 text-sm"
                    placeholder="Libellé"
                    value={slot.label}
                    onChange={(e) => {
                      const slots = po.slots.map((s) =>
                        s.id === slot.id ? { ...s, label: e.target.value } : s,
                      );
                      patchTool("portes-ouvertes", { slots });
                    }}
                  />
                  <input
                    type="number"
                    className="rounded-lg border px-3 py-2 text-sm"
                    placeholder="Places max"
                    value={slot.maxPlaces || ""}
                    onChange={(e) => {
                      const slots = po.slots.map((s) =>
                        s.id === slot.id
                          ? { ...s, maxPlaces: Number(e.target.value) || undefined }
                          : s,
                      );
                      patchTool("portes-ouvertes", { slots });
                    }}
                  />
                  <input
                    type="datetime-local"
                    className="rounded-lg border px-3 py-2 text-sm"
                    value={slot.startAt.slice(0, 16)}
                    onChange={(e) => {
                      const slots = po.slots.map((s) =>
                        s.id === slot.id
                          ? { ...s, startAt: new Date(e.target.value).toISOString() }
                          : s,
                      );
                      patchTool("portes-ouvertes", { slots });
                    }}
                  />
                  <input
                    type="datetime-local"
                    className="rounded-lg border px-3 py-2 text-sm"
                    value={slot.endAt.slice(0, 16)}
                    onChange={(e) => {
                      const slots = po.slots.map((s) =>
                        s.id === slot.id
                          ? { ...s, endAt: new Date(e.target.value).toISOString() }
                          : s,
                      );
                      patchTool("portes-ouvertes", { slots });
                    }}
                  />
                </div>
                <p className="text-xs text-slate-500">
                  Inscrits : {stats[slot.id] || 0}
                  {slot.maxPlaces ? ` / ${slot.maxPlaces}` : ""}
                </p>
              </div>
            ))}
            <p className="text-sm text-slate-600">
              Total inscriptions : <strong>{regCount}</strong>
            </p>
            <a
              href="/portes-ouvertes"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm font-bold text-violet-700 underline"
            >
              Page publique →
            </a>
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
