"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import ToolboxModal from "@/app/components/toolbox/ToolboxModal";
import RentreeEditor from "@/app/components/toolbox/RentreeEditor";
import FournituresEditor from "@/app/components/toolbox/FournituresEditor";
import { renderToolboxAdminIcon, renderToolboxIcon } from "@/app/components/toolbox/ToolboxIcons";
import type { Establishment } from "@/app/lib/app-config-schemas";
import type { ToolboxConfig, PortesOuvertesSlot, TarifsNiveau } from "@/app/lib/toolbox-types";
import { TOOLBOX_TOOLS_META, TOOLBOX_ADMIN_LINKS } from "@/app/lib/toolbox-tools";

type Tab = "overview" | "rentree" | "fournitures" | "tarifs" | "portes-ouvertes" | "secret-santa";

const NIVEAUX: TarifsNiveau[] = ["maternelle", "elementaire", "college", "lycee"];

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

export default function ToolboxAdminPage() {
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
  const [previewOpen, setPreviewOpen] = useState(false);

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
    if (t === "rentree" || t === "fournitures" || t === "tarifs" || t === "portes-ouvertes" || t === "secret-santa") {
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

  if (loading || !config) {
    return (
      <main className="mx-auto max-w-5xl px-4 py-10">
        <p className="text-slate-500">{loading ? "Chargement…" : "Configuration indisponible."}</p>
      </main>
    );
  }

  const po = config.tools["portes-ouvertes"];

  return (
    <RequireOrgAdmin>
      <main className="mx-auto max-w-5xl px-4 py-8">
        <header className="mb-8 flex flex-wrap items-start justify-between gap-4">
          <div>
            <p className="text-[10px] font-black uppercase tracking-widest text-slate-400">Administration</p>
            <h1 className="text-3xl font-black text-slate-900">Boîte à outils</h1>
            <p className="mt-2 text-sm text-slate-600 max-w-xl">
              Activez les modules saisonniers par établissement. Les paramètres généraux (dont les utilisateurs) sont dans le module Établissement.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => setPreviewOpen(true)}
              className="rounded-xl border border-slate-200 px-4 py-2 text-sm font-bold text-slate-700 hover:bg-slate-50"
            >
              Aperçu modal
            </button>
            <button
              type="button"
              onClick={() => void save()}
              disabled={saving}
              className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              {saving ? "Enregistrement…" : "Enregistrer"}
            </button>
          </div>
        </header>

        {error && <p className="mb-4 rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">{error}</p>}
        {msg && <p className="mb-4 rounded-xl border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-800">{msg}</p>}

        <nav className="mb-6 flex flex-wrap gap-2">
          {(
            [
              ["overview", "Vue d'ensemble"],
              ["rentree", "Rentrée"],
              ["fournitures", "Fournitures"],
              ["tarifs", "Simulateur tarifs"],
              ["portes-ouvertes", "Portes ouvertes"],
              ["secret-santa", "Secret Santa"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              onClick={() => setTab(id)}
              className={`rounded-xl px-4 py-2 text-sm font-bold transition ${
                tab === id ? "bg-slate-900 text-white" : "bg-slate-100 text-slate-700 hover:bg-slate-200"
              }`}
            >
              {label}
            </button>
          ))}
        </nav>

        {tab === "overview" && (
          <section className="grid gap-4 sm:grid-cols-2">
            {TOOLBOX_TOOLS_META.map((meta) => {
              const tool = config.tools[meta.id];
              const enabled =
                meta.id === "qrcreator"
                  ? config.tools.qrcreator.enabled
                  : "enabled" in tool
                    ? tool.enabled
                    : false;
              return (
                <div key={meta.id} className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm">
                  <div className="flex items-start gap-4">
                    <span className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${meta.bg} ${meta.color}`}>
                      {renderToolboxIcon(meta.id, "w-8 h-8")}
                    </span>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <h2 className="font-bold text-slate-900">{meta.label}</h2>
                        <Toggle
                          checked={enabled}
                          onChange={(v) => {
                            if (meta.id === "qrcreator") patchTool("qrcreator", { enabled: v });
                            else patchTool(meta.id, { enabled: v } as never);
                          }}
                          label=""
                        />
                      </div>
                      <p className="mt-1 text-xs text-slate-500">{meta.description}</p>
                      {meta.publicPath && enabled && (
                        <a
                          href={`${publicOrigin}${meta.publicPath}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="mt-2 inline-block text-xs font-bold text-blue-600 underline break-all"
                        >
                          Page publique → {publicOrigin}
                          {meta.publicPath}
                        </a>
                      )}
                      {meta.id === "rentree" && enabled && (
                        <button
                          type="button"
                          onClick={() => setTab("rentree")}
                          className="mt-2 block text-xs font-bold text-amber-800 underline"
                        >
                          Configurer établissements & documents →
                        </button>
                      )}
                      {meta.id === "simulateur-fournitures" && enabled && (
                        <button
                          type="button"
                          onClick={() => setTab("fournitures")}
                          className="mt-2 block text-xs font-bold text-emerald-800 underline"
                        >
                          Configurer listes par classe →
                        </button>
                      )}
                      {meta.id === "secret-santa" && enabled && (
                        <Link href="/toolbox/secret-santa" className="mt-2 inline-block text-xs font-bold text-blue-600 underline">
                          Lancer le tirage →
                        </Link>
                      )}
                      {meta.id === "repartition-classes" && enabled && (
                        <Link href="/toolbox/repartition-classes" className="mt-2 inline-block text-xs font-bold text-indigo-800 underline">
                          Ouvrir le module →
                        </Link>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
            <div className="sm:col-span-2 rounded-2xl border border-dashed border-slate-200 p-4 text-sm text-slate-500">
              Le QR code reste accessible via <Link href="/qrcreator" className="font-bold text-slate-800 underline">/qrcreator</Link> même s&apos;il n&apos;est pas activé dans la boîte.
            </div>

            <div className="sm:col-span-2">
              <h2 className="mb-3 text-sm font-black uppercase tracking-widest text-slate-500">Administration</h2>
              <div className="grid gap-4 sm:grid-cols-2">
                {TOOLBOX_ADMIN_LINKS.map((link) => (
                  <Link
                    key={link.id}
                    href={link.adminPath}
                    className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm transition hover:border-slate-300 hover:shadow-md"
                  >
                    <div className="flex items-start gap-4">
                      <span className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-2xl ${link.bg} ${link.color}`}>
                        {renderToolboxAdminIcon(link.id, "w-8 h-8")}
                      </span>
                      <div className="min-w-0">
                        <h3 className="font-bold text-slate-900">{link.label}</h3>
                        <p className="mt-1 text-xs text-slate-500">{link.description}</p>
                        <span className="mt-2 inline-block text-xs font-bold text-slate-700 underline">Ouvrir →</span>
                      </div>
                    </div>
                  </Link>
                ))}
              </div>
            </div>
          </section>
        )}

        {tab === "rentree" && (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-6">
            <Toggle checked={config.tools.rentree.enabled} onChange={(v) => patchTool("rentree", { enabled: v })} label="Activer la préparation de rentrée (page publique /rentree)" />
            <Toggle
              checked={config.tools.rentree.showSimulateurTarifs}
              onChange={(v) => patchTool("rentree", { showSimulateurTarifs: v })}
              label="Afficher le lien simulateur tarifs"
            />
            <Toggle
              checked={config.tools.rentree.showSimulateurFournitures}
              onChange={(v) => patchTool("rentree", { showSimulateurFournitures: v })}
              label="Afficher le lien simulateur fournitures"
            />
            <RentreeEditor
              rentree={config.tools.rentree}
              establishments={establishments}
              onChange={(patch) => patchTool("rentree", patch)}
              onPagesChange={(pages) => patchTool("rentree", { pages })}
            />
            <p className="text-xs text-slate-500">
              Page publique :{" "}
              <a href="/rentree" target="_blank" rel="noopener noreferrer" className="underline font-semibold">
                /rentree
              </a>
              {" "}— un onglet par établissement actif (paramètres généraux).
            </p>
          </section>
        )}

        {tab === "fournitures" && (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-6">
            <Toggle
              checked={config.tools["simulateur-fournitures"].enabled}
              onChange={(v) => patchTool("simulateur-fournitures", { enabled: v })}
              label="Publier le simulateur fournitures (/simulateurFournitures)"
            />
            <FournituresEditor
              config={config.tools["simulateur-fournitures"]}
              onChange={(patch) => patchTool("simulateur-fournitures", patch)}
            />
            <a
              href={`${publicOrigin}/simulateurFournitures`}
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm font-bold text-emerald-700 underline break-all"
            >
              Voir la page publique → {publicOrigin}/simulateurFournitures
            </a>
          </section>
        )}

        {tab === "tarifs" && (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-6">
            <Toggle
              checked={config.tools["simulateur-tarifs"].enabled}
              onChange={(v) => patchTool("simulateur-tarifs", { enabled: v })}
              label="Publier le simulateur de tarifs (/simulateurTarifs)"
            />
            <label className="block max-w-xs">
              <span className="text-xs font-bold text-slate-500 uppercase">Année affichée</span>
              <input
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                value={config.tools["simulateur-tarifs"].schoolYear}
                onChange={(e) => patchTool("simulateur-tarifs", { schoolYear: e.target.value })}
              />
            </label>
            {NIVEAUX.map((niveau) => (
              <div key={niveau}>
                <p className="text-sm font-bold text-slate-800 capitalize mb-2">Enseignement — {niveau}</p>
                <p className="text-xs text-slate-500 mb-2">5 tranches QF (du plus élevé au plus bas)</p>
                <div className="flex flex-wrap gap-2">
                  {config.tools["simulateur-tarifs"].enseignement[niveau].map((val, i) => (
                    <input
                      key={i}
                      type="number"
                      className="w-20 rounded-lg border border-slate-200 px-2 py-1 text-sm"
                      value={val}
                      onChange={(e) => {
                        const next = [...config.tools["simulateur-tarifs"].enseignement[niveau]];
                        next[i] = Number(e.target.value);
                        patchTool("simulateur-tarifs", {
                          enseignement: {
                            ...config.tools["simulateur-tarifs"].enseignement,
                            [niveau]: next,
                          },
                        });
                      }}
                    />
                  ))}
                </div>
              </div>
            ))}
            <label className="block max-w-xs">
              <span className="text-xs font-bold text-slate-500 uppercase">Pension annuelle (€)</span>
              <input
                type="number"
                className="mt-1 w-full rounded-xl border border-slate-200 px-3 py-2 text-sm"
                value={config.tools["simulateur-tarifs"].pensionAnnuel}
                onChange={(e) => patchTool("simulateur-tarifs", { pensionAnnuel: Number(e.target.value) })}
              />
            </label>
            <a href="/simulateurTarifs" target="_blank" rel="noopener noreferrer" className="text-sm font-bold text-blue-600 underline">
              Voir la page publique →
            </a>
          </section>
        )}

        {tab === "portes-ouvertes" && (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-4">
            <Toggle
              checked={po.enabled}
              onChange={(v) => patchTool("portes-ouvertes", { enabled: v })}
              label="Activer la page publique /portes-ouvertes"
            />
            <div className="grid gap-4 sm:grid-cols-2">
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">Titre</span>
                <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={po.title} onChange={(e) => patchTool("portes-ouvertes", { title: e.target.value })} />
              </label>
              <label className="block">
                <span className="text-xs font-bold text-slate-500 uppercase">E-mail notifications</span>
                <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={po.notifyEmail || ""} onChange={(e) => patchTool("portes-ouvertes", { notifyEmail: e.target.value })} />
              </label>
            </div>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Introduction</span>
              <textarea className="mt-1 w-full rounded-xl border px-3 py-2 text-sm min-h-[80px]" value={po.intro} onChange={(e) => patchTool("portes-ouvertes", { intro: e.target.value })} />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Adresse</span>
              <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={po.address} onChange={(e) => patchTool("portes-ouvertes", { address: e.target.value })} placeholder="12 rue …, 75000 Paris" />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Lien Google Maps (optionnel)</span>
              <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={po.mapsUrl || ""} onChange={(e) => patchTool("portes-ouvertes", { mapsUrl: e.target.value })} />
            </label>

            <div className="flex items-center justify-between">
              <h3 className="font-bold text-slate-900">Créneaux</h3>
              <button
                type="button"
                onClick={() => patchTool("portes-ouvertes", { slots: [...po.slots, emptySlot()] })}
                className="rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-bold text-white"
              >
                + Créneau
              </button>
            </div>
            {po.slots.length === 0 && <p className="text-sm text-slate-500">Ajoutez au moins un créneau pour ouvrir les inscriptions.</p>}
            {po.slots.map((slot, idx) => (
              <div key={slot.id} className="rounded-xl border border-slate-100 bg-slate-50 p-4 space-y-3">
                <div className="flex justify-between items-center">
                  <span className="text-xs font-black uppercase text-slate-500">Créneau {idx + 1}</span>
                  <button
                    type="button"
                    onClick={() => patchTool("portes-ouvertes", { slots: po.slots.filter((s) => s.id !== slot.id) })}
                    className="text-xs text-rose-600 font-bold"
                  >
                    Supprimer
                  </button>
                </div>
                <div className="grid gap-3 sm:grid-cols-2">
                  <input className="rounded-lg border px-3 py-2 text-sm" placeholder="Libellé" value={slot.label} onChange={(e) => {
                    const slots = po.slots.map((s) => (s.id === slot.id ? { ...s, label: e.target.value } : s));
                    patchTool("portes-ouvertes", { slots });
                  }} />
                  <input type="number" className="rounded-lg border px-3 py-2 text-sm" placeholder="Places max" value={slot.maxPlaces || ""} onChange={(e) => {
                    const slots = po.slots.map((s) => (s.id === slot.id ? { ...s, maxPlaces: Number(e.target.value) || undefined } : s));
                    patchTool("portes-ouvertes", { slots });
                  }} />
                  <input type="datetime-local" className="rounded-lg border px-3 py-2 text-sm" value={slot.startAt.slice(0, 16)} onChange={(e) => {
                    const slots = po.slots.map((s) => (s.id === slot.id ? { ...s, startAt: new Date(e.target.value).toISOString() } : s));
                    patchTool("portes-ouvertes", { slots });
                  }} />
                  <input type="datetime-local" className="rounded-lg border px-3 py-2 text-sm" value={slot.endAt.slice(0, 16)} onChange={(e) => {
                    const slots = po.slots.map((s) => (s.id === slot.id ? { ...s, endAt: new Date(e.target.value).toISOString() } : s));
                    patchTool("portes-ouvertes", { slots });
                  }} />
                </div>
                <p className="text-xs text-slate-500">
                  Inscrits : {stats[slot.id] || 0}
                  {slot.maxPlaces ? ` / ${slot.maxPlaces}` : ""}
                </p>
              </div>
            ))}
            <p className="text-sm text-slate-600">Total inscriptions : <strong>{regCount}</strong></p>
            <a href="/portes-ouvertes" target="_blank" rel="noopener noreferrer" className="text-sm font-bold text-violet-700 underline">
              Page publique →
            </a>
          </section>
        )}

        {tab === "secret-santa" && (
          <section className="rounded-2xl border border-slate-200 bg-white p-6 space-y-4">
            <Toggle checked={config.tools["secret-santa"].enabled} onChange={(v) => patchTool("secret-santa", { enabled: v })} label="Activer Secret Santa" />
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Titre</span>
              <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={config.tools["secret-santa"].title} onChange={(e) => patchTool("secret-santa", { title: e.target.value })} />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Budget indicatif</span>
              <input className="mt-1 w-full rounded-xl border px-3 py-2 text-sm" value={config.tools["secret-santa"].budgetHint} onChange={(e) => patchTool("secret-santa", { budgetHint: e.target.value })} />
            </label>
            <label className="block">
              <span className="text-xs font-bold text-slate-500 uppercase">Participants (un nom par ligne)</span>
              <textarea
                className="mt-1 w-full rounded-xl border px-3 py-2 text-sm min-h-[140px] font-mono"
                value={config.tools["secret-santa"].participantNames.join("\n")}
                onChange={(e) =>
                  patchTool("secret-santa", {
                    participantNames: e.target.value.split(/\r?\n/).map((s) => s.trim()).filter(Boolean),
                  })
                }
              />
            </label>
            <Link href="/toolbox/secret-santa" className="inline-block rounded-xl bg-red-600 px-4 py-2 text-sm font-bold text-white">
              Lancer le tirage →
            </Link>
          </section>
        )}

        <ToolboxModal open={previewOpen} onClose={() => setPreviewOpen(false)} />
      </main>
    </RequireOrgAdmin>
  );
}
