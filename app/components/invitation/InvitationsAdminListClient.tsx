"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import RequireModuleAccess from "@/app/components/RequireModuleAccess";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import type { InvitationPageRecord } from "@/app/lib/invitation-types";

export default function InvitationsAdminListClient() {
  const [pages, setPages] = useState<InvitationPageRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [title, setTitle] = useState("Remise de diplôme");
  const [creating, setCreating] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch("/api/invitation/pages", { cache: "no-store" });
      const data = (await res.json()) as { pages?: InvitationPageRecord[]; error?: string };
      if (!res.ok) throw new Error(data.error || "Chargement impossible.");
      setPages(data.pages || []);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  async function createPage() {
    setCreating(true);
    setError(null);
    try {
      const res = await fetch("/api/invitation/pages", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          title: title.trim() || "Invitation",
          theme: "remise_diplome",
          diplomaMode: "both",
          enabled: false,
          maxTotalPersons: 200,
          maxPersonsPerEleve: 4,
        }),
      });
      const data = (await res.json()) as { page?: InvitationPageRecord; error?: string };
      if (!res.ok) throw new Error(data.error || "Création impossible.");
      setTitle("Remise de diplôme");
      await load();
      if (data.page) {
        window.location.href = `/etablissement/evenements/invitations/${data.page.id}`;
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setCreating(false);
    }
  }

  return (
    <RequireModuleAccess moduleId="evenements">
      <ModulePageShell>
        <ModulePageHeader
          eyebrow="Événements"
          title="Invitations cérémonies"
          description="Pages publiques RSVP (remise bac, brevet…) — une page = un lien familles + un tableau de bord."
          actions={
            <Link
              href="/etablissement/evenements"
              className="rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
            >
              Retour
            </Link>
          }
        />

        <section className="rounded-2xl border border-slate-200 bg-white p-5 space-y-4">
          <h2 className="text-sm font-black text-slate-900">Nouvelle page</h2>
          <div className="flex flex-wrap items-end gap-3">
            <label className="flex flex-col gap-1 text-xs font-semibold text-slate-600">
              Titre
              <input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="rounded-xl border border-slate-200 px-3 py-2 text-sm font-medium text-slate-900 min-w-[220px]"
                placeholder="Remise de diplôme"
              />
            </label>
            <ModuleButton type="button" onClick={() => void createPage()} disabled={creating}>
              {creating ? "Création…" : "Créer"}
            </ModuleButton>
          </div>
        </section>

        {error ? (
          <p className="rounded-xl border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-800">
            {error}
          </p>
        ) : null}

        <section className="space-y-3">
          <h2 className="text-sm font-black text-slate-900">Pages</h2>
          {loading ? (
            <p className="text-sm text-slate-500">Chargement…</p>
          ) : pages.length === 0 ? (
            <p className="text-sm text-slate-500">Aucune page pour l’instant.</p>
          ) : (
            <ul className="grid gap-3 sm:grid-cols-2">
              {pages.map((p) => (
                <li
                  key={p.id}
                  className="rounded-2xl border border-slate-200 bg-white p-4 flex flex-col gap-2"
                >
                  <div className="flex items-start justify-between gap-2">
                    <h3 className="font-black text-slate-900 leading-tight">{p.title}</h3>
                    <span
                      className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-bold uppercase ${
                        p.enabled
                          ? "bg-emerald-100 text-emerald-800"
                          : "bg-slate-100 text-slate-600"
                      }`}
                    >
                      {p.enabled ? "Publiée" : "Brouillon"}
                    </span>
                  </div>
                  <p className="text-xs text-slate-500">
                    /invitation/{p.slug}
                  </p>
                  <div className="flex flex-wrap gap-2 pt-1">
                    <Link
                      href={`/etablissement/evenements/invitations/${p.id}`}
                      className="inline-flex rounded-xl bg-slate-900 px-3 py-2 text-xs font-bold text-white hover:bg-slate-800"
                    >
                      Paramétrer / tableau de bord
                    </Link>
                    {p.enabled ? (
                      <Link
                        href={`/invitation/${p.slug}`}
                        className="inline-flex rounded-xl border border-slate-200 px-3 py-2 text-xs font-bold text-slate-700 hover:bg-slate-50"
                        target="_blank"
                      >
                        Page publique
                      </Link>
                    ) : null}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </ModulePageShell>
    </RequireModuleAccess>
  );
}
