"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import type { BienEtreSignalement, BienEtreSignalementIndexEntry } from "@/app/lib/bien-etre-types";
import ModuleButton from "@/app/components/module-chrome/ModuleButton";
import ModuleCard from "@/app/components/module-chrome/ModuleCard";
import ModulePageHeader from "@/app/components/module-chrome/ModulePageHeader";
import ModulePageShell from "@/app/components/module-chrome/ModulePageShell";
import { dash } from "@/app/lib/dashboard-brand";

const STATUS_LABELS: Record<string, string> = {
  nouveau: "Nouveau",
  en_cours: "En cours",
  cloture: "Clôturé",
};

function ReferentContent() {
  const searchParams = useSearchParams();
  const selectedId = searchParams.get("id");
  const [items, setItems] = useState<BienEtreSignalementIndexEntry[]>([]);
  const [detail, setDetail] = useState<BienEtreSignalement | null>(null);
  const [loading, setLoading] = useState(true);
  const [note, setNote] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadList = useCallback(async () => {
    const res = await fetch("/api/bien-etre/signalements", { cache: "no-store" });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Erreur");
    setItems(j.items || []);
  }, []);

  const loadDetail = useCallback(async (id: string) => {
    const res = await fetch(`/api/bien-etre/signalements?id=${encodeURIComponent(id)}`, { cache: "no-store" });
    const j = await res.json();
    if (!res.ok) throw new Error(j.error || "Erreur");
    setDetail(j.signalement);
    setNote(j.signalement?.referentNote || "");
  }, []);

  useEffect(() => {
    (async () => {
      setLoading(true);
      setError(null);
      try {
        await loadList();
        if (selectedId) await loadDetail(selectedId);
        else setDetail(null);
      } catch (e: unknown) {
        setError(e instanceof Error ? e.message : "Erreur");
      } finally {
        setLoading(false);
      }
    })();
  }, [selectedId, loadList, loadDetail]);

  const updateStatus = async (status: "nouveau" | "en_cours" | "cloture") => {
    if (!detail) return;
    setSaving(true);
    try {
      const res = await fetch("/api/bien-etre/signalements", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: detail.id, status, referentNote: note }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Erreur");
      setDetail(j.signalement);
      await loadList();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setSaving(false);
    }
  };

  return (
    <ModulePageShell maxWidthClass="max-w-[1280px]">
      <ModulePageHeader
        eyebrow="Élèves"
        title="Signalements bien-être"
        description="Transmis par les élèves au psychologue."
        actions={
          <Link href="/bien-etre/config" className={`text-sm font-semibold underline ${dash.textPrimary}`}>
            Configuration →
          </Link>
        }
      />

      {error ? <p className="mb-4 text-sm text-rose-700">{error}</p> : null}
      {loading ? <p className={`text-sm ${dash.textMid}`}>Chargement…</p> : null}

      <div className="grid gap-4 lg:grid-cols-2">
        <ModuleCard>
          <h2 className={`border-b border-white/60 px-4 py-3 text-lg font-semibold ${dash.ink}`}>Liste</h2>
          <ul className="max-h-[32rem] divide-y divide-white/50 overflow-y-auto">
            {items.length === 0 ? (
              <li className={`p-4 text-sm ${dash.textMid}`}>Aucun signalement.</li>
            ) : (
              items.map((it) => (
                <li key={it.id}>
                  <Link
                    href={`/bien-etre/referent?id=${encodeURIComponent(it.id)}`}
                    className={`block px-4 py-3 ${dash.hoverBgSoft} ${selectedId === it.id ? dash.bgSoft : ""}`}
                  >
                    <p className={`font-semibold ${dash.ink}`}>{it.prenom}</p>
                    <p className={`text-xs ${dash.textMid}`}>
                      {new Date(it.createdAt).toLocaleString("fr-FR")} · {STATUS_LABELS[it.status] || it.status} ·{" "}
                      {it.severity}
                    </p>
                  </Link>
                </li>
              ))
            )}
          </ul>
        </ModuleCard>

        <ModuleCard bodyClassName="p-5">
          {detail ? (
            <div className="space-y-4">
              <h2 className={`text-xl font-semibold ${dash.ink}`}>{detail.prenom}</h2>
              <p className={`text-sm ${dash.textMid}`}>
                {new Date(detail.createdAt).toLocaleString("fr-FR")}
                {detail.classe ? ` · ${detail.classe}` : ""}
              </p>
              <p>
                <span className={dash.fieldLabel}>Gravité</span> — {detail.severity}
              </p>
              <p>
                <span className={dash.fieldLabel}>Catégories</span> — {detail.categories.join(", ") || "—"}
              </p>
              <div>
                <p className={`mb-1 ${dash.fieldLabel}`}>Résumé</p>
                <p className={`whitespace-pre-wrap rounded-xl p-3 text-sm leading-relaxed ${dash.bgSoft50}`}>
                  {detail.summary}
                </p>
              </div>
              {detail.complement ? (
                <div>
                  <p className={`mb-1 ${dash.fieldLabel}`}>Complément élève</p>
                  <p className="whitespace-pre-wrap text-sm">{detail.complement}</p>
                </div>
              ) : null}
              <label className={`block text-sm font-semibold ${dash.ink}`}>
                Note interne
                <textarea
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  rows={3}
                  className={`mt-1 ${dash.field} text-sm`}
                />
              </label>
              <div className="flex flex-wrap gap-2">
                <ModuleButton variant="secondary" disabled={saving} onClick={() => updateStatus("en_cours")}>
                  En cours
                </ModuleButton>
                <ModuleButton disabled={saving} onClick={() => updateStatus("cloture")}>
                  Clôturer
                </ModuleButton>
              </div>
            </div>
          ) : (
            <p className={`text-sm ${dash.textMid}`}>Sélectionnez un signalement dans la liste.</p>
          )}
        </ModuleCard>
      </div>
    </ModulePageShell>
  );
}

export default function BienEtreReferentPage() {
  return (
    <Suspense
      fallback={
        <ModulePageShell maxWidthClass="max-w-[1280px]">
          <p className={`text-sm ${dash.textMid}`}>Chargement…</p>
        </ModulePageShell>
      }
    >
      <ReferentContent />
    </Suspense>
  );
}
