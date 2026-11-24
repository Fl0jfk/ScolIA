"use client";

/** Page publique des familles (sans compte) : charge la vue par jeton et affiche la page + le formulaire réel. */

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { FamilyPageView } from "@/app/components/travels/family/FamilyPageView";
import { FamilyLiveForm } from "@/app/components/travels/family/FamilyLiveForm";
import { FamilyAfterSignDocs, FamilyObjectionBox, StudentStateNotice } from "@/app/components/travels/family/FamilyExtras";
import type { PublicFamilyView } from "@/app/lib/travels-v2-registration";

function when(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", { timeZone: "Europe/Paris", dateStyle: "long", timeStyle: "short" });
}

export function FamilyPublicPage({ token }: { token: string }) {
  const [view, setView] = useState<PublicFamilyView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [invalid, setInvalid] = useState(false);

  const load = useCallback(async () => {
    try {
      const res = await fetch(`/api/voyages/famille/${encodeURIComponent(token)}`, { cache: "no-store" });
      const j = await res.json().catch(() => ({}));
      if (res.status === 404) {
        setInvalid(true);
        return;
      }
      if (!res.ok) throw new Error(j.error || "Impossible de charger la page.");
      setView(j as PublicFamilyView);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Impossible de charger la page.");
    }
  }, [token]);

  useEffect(() => {
    void load();
  }, [load]);

  if (invalid) {
    return (
      <main className="mx-auto max-w-xl p-4 sm:p-8">
        <div className="rounded-2xl border-2 border-amber-200 bg-amber-50 p-6 text-sm text-amber-950">
          <h1 className="text-lg font-black">Lien invalide ou expiré</h1>
          <p className="mt-2">Ce lien n&apos;est pas reconnu. Vous pouvez demander un nouveau lien à l&apos;adresse e-mail utilisée par l&apos;établissement.</p>
          <Link href="/voyages/famille" className="mt-4 inline-block rounded-xl bg-indigo-600 px-4 py-2 font-bold text-white">
            Recevoir un nouveau lien
          </Link>
        </div>
      </main>
    );
  }
  if (error) return <main className="mx-auto max-w-xl p-8 text-sm font-semibold text-rose-700">{error}</main>;
  if (!view) return <main className="mx-auto max-w-xl p-8 text-sm text-slate-500">Chargement…</main>;

  const base = `/api/voyages/famille/${encodeURIComponent(token)}`;
  const closed = !view.window.open && !view.locked;

  return (
    <main className="mx-auto max-w-3xl space-y-4 p-4 sm:p-8">
      {view.signed ? (
        <div className="space-y-2 rounded-2xl border-2 border-emerald-300 bg-emerald-50 p-4 text-sm text-emerald-950" role="status">
          <p className="font-black">
            {view.signed.decision === "oui"
              ? `${view.studentName} est inscrit(e) au voyage.`
              : `Réponse enregistrée : ${view.studentName} ne participe pas au voyage.`}
          </p>
          <p>
            Signé par {view.signed.signerName} le {when(view.signed.signedAt)}. Votre réponse est verrouillée.
          </p>
          {view.signed.decision === "oui" ? (
            <p className="flex flex-wrap gap-3">
              <a className="rounded-lg bg-white px-3 py-2 font-bold text-emerald-800 ring-1 ring-emerald-300" href={`${base}/file?kind=pdf`}>
                Télécharger la circulaire signée (PDF)
              </a>
              <a className="rounded-lg bg-white px-3 py-2 font-bold text-emerald-800 ring-1 ring-emerald-300" href={`${base}/file?kind=ics`}>
                Ajouter à mon agenda (.ics)
              </a>
            </p>
          ) : null}
        </div>
      ) : null}

      <StudentStateNotice view={view} />
      <FamilyObjectionBox token={token} view={view} onView={setView} />
      {view.signed?.decision === "oui" ? <FamilyAfterSignDocs token={token} view={view} onView={setView} /> : null}

      {closed ? (
        <div className="rounded-2xl border-2 border-rose-200 bg-rose-50 p-4 text-sm font-semibold text-rose-900" role="alert">
          {view.window.open === false && view.window.reason === "deadline"
            ? "La date limite de réponse est dépassée : la page est fermée. Contactez l'organisateur du voyage si besoin."
            : "Les inscriptions de ce voyage sont closes."}
        </div>
      ) : null}

      <FamilyPageView
        model={view.model}
        mode="live"
        formSlot={
          view.locked || closed ? (
            <section className="rounded-2xl border bg-slate-50 p-4 text-sm text-slate-600">
              {view.locked ? "Votre réponse a été signée et ne peut plus être modifiée." : "Le formulaire est fermé."}
            </section>
          ) : (
            <FamilyLiveForm token={token} view={view} onView={setView} />
          )
        }
      />
    </main>
  );
}
