"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import RequireOrgAdmin from "@/app/components/RequireOrgAdmin";
import CommunicationDocumentsPanel from "@/app/components/communication/CommunicationDocumentsPanel";
import CommunicationSitePostsPanel from "@/app/components/communication/CommunicationSitePostsPanel";
import type { ToolboxConfig, TarifsNiveau } from "@/app/lib/toolbox-types";

const NIVEAUX: TarifsNiveau[] = ["maternelle", "elementaire", "college", "lycee"];

type Tab = "documents" | "tarifs" | "actus";

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

export default function CommunicationHubClient() {
  const [tab, setTab] = useState<Tab>("documents");
  const [config, setConfig] = useState<ToolboxConfig | null>(null);
  const [publicOrigin, setPublicOrigin] = useState("");
  const [customWebsiteEnabled, setCustomWebsiteEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [tbRes, setRes] = await Promise.all([
        fetch("/api/toolbox/config", { cache: "no-store" }),
        fetch("/api/settings", { cache: "no-store" }),
      ]);
      const tj = await tbRes.json();
      if (!tbRes.ok) throw new Error(tj.error || "Erreur toolbox");
      setConfig(tj.config);
      setPublicOrigin(typeof tj.publicOrigin === "string" ? tj.publicOrigin.replace(/\/$/, "") : "");

      if (setRes.ok) {
        const sj = await setRes.json();
        setCustomWebsiteEnabled(Boolean(sj.config?.identity?.customWebsite?.enabled));
      }
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

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

  function patchTarifs(patch: Partial<ToolboxConfig["tools"]["simulateur-tarifs"]>) {
    if (!config) return;
    setConfig({
      ...config,
      tools: {
        ...config.tools,
        "simulateur-tarifs": { ...config.tools["simulateur-tarifs"], ...patch },
      },
    });
  }

  if (loading) {
    return (
      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-10">
        <p className="text-slate-500">Chargement…</p>
      </main>
    );
  }

  const tarifs = config?.tools["simulateur-tarifs"];

  return (
    <RequireOrgAdmin>
      <main className="max-w-5xl mx-auto px-4 sm:px-6 py-8 space-y-6">
        <header className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <p className="text-[10px] font-black uppercase tracking-widest text-sky-700">
              Établissement
            </p>
            <h1 className="text-3xl font-black text-slate-900 tracking-tight mt-1">
              Communication
            </h1>
            <p className="text-sm text-slate-500 mt-2 max-w-2xl">
              Documents familles (PDF zéro papier), simulateur de tarifs, et actus site si vitrine
              Scola activée.
            </p>
          </div>
          {tab === "tarifs" ? (
            <button
              type="button"
              onClick={() => void save()}
              disabled={saving || !config}
              className="rounded-xl bg-slate-900 px-4 py-2 text-sm font-bold text-white disabled:opacity-50"
            >
              {saving ? "Enregistrement…" : "Enregistrer"}
            </button>
          ) : null}
        </header>

        <div className="flex flex-wrap gap-2">
          {(
            [
              ["documents", "Documents"],
              ["tarifs", "Tarifs"],
              ["actus", "Actus site"],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              onClick={() => setTab(id)}
              className={`rounded-xl px-4 py-2 text-sm font-bold ${
                tab === id ? "bg-sky-700 text-white" : "bg-slate-100 text-slate-700"
              }`}
            >
              {label}
              {id === "actus" && !customWebsiteEnabled ? (
                <span className="ml-1 text-[10px] font-semibold opacity-80">option</span>
              ) : null}
            </button>
          ))}
        </div>

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

        {tab === "documents" ? <CommunicationDocumentsPanel /> : null}

        {tab === "actus" ? (
          <CommunicationSitePostsPanel enabled={customWebsiteEnabled} onRefreshFlag={() => void load()} />
        ) : null}

        {tab === "tarifs" && tarifs ? (
          <section className="rounded-2xl border border-sky-200 bg-sky-50/40 p-6 space-y-6">
            <div>
              <h2 className="text-lg font-black text-sky-950">Simulateur de tarifs</h2>
              <p className="text-sm text-sky-900/80 mt-1">
                Page publique partageable pour que les familles estiment le coût de scolarité.
              </p>
            </div>

            <Toggle
              checked={tarifs.enabled}
              onChange={(v) => patchTarifs({ enabled: v })}
              label="Publier le simulateur (/simulateurTarifs)"
            />

            <label className="block max-w-xs">
              <span className="text-xs font-bold text-slate-500 uppercase">Année affichée</span>
              <input
                className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                value={tarifs.schoolYear}
                onChange={(e) => patchTarifs({ schoolYear: e.target.value })}
              />
            </label>

            {NIVEAUX.map((niveau) => (
              <div key={niveau}>
                <p className="text-sm font-bold text-slate-800 capitalize mb-2">
                  Enseignement — {niveau}
                </p>
                <p className="text-xs text-slate-500 mb-2">5 tranches QF (du plus élevé au plus bas)</p>
                <div className="flex flex-wrap gap-2">
                  {tarifs.enseignement[niveau].map((val, i) => (
                    <input
                      key={i}
                      type="number"
                      className="w-20 rounded-lg border border-slate-200 bg-white px-2 py-1 text-sm"
                      value={val}
                      onChange={(e) => {
                        const next = [...tarifs.enseignement[niveau]];
                        next[i] = Number(e.target.value);
                        patchTarifs({
                          enseignement: {
                            ...tarifs.enseignement,
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
                className="mt-1 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm"
                value={tarifs.pensionAnnuel}
                onChange={(e) => patchTarifs({ pensionAnnuel: Number(e.target.value) })}
              />
            </label>

            {tarifs.enabled ? (
              <a
                href={`${publicOrigin}/simulateurTarifs`}
                target="_blank"
                rel="noopener noreferrer"
                className="inline-block text-sm font-bold text-sky-800 underline break-all"
              >
                Voir la page publique → {publicOrigin}/simulateurTarifs
              </a>
            ) : null}

            <p className="text-xs text-slate-500">
              Optionnel : lien depuis la{" "}
              <Link
                href="/etablissement/evenements?tab=rentree"
                className="font-semibold underline"
              >
                rentrée digitale
              </Link>
              .
            </p>
          </section>
        ) : null}

        {tab === "tarifs" && !tarifs ? (
          <p className="text-sm text-slate-500">Configuration tarifs indisponible.</p>
        ) : null}
      </main>
    </RequireOrgAdmin>
  );
}
