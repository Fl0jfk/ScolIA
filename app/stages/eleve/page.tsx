"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import type { StageConvention } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";

function EleveContent() {
  const searchParams = useSearchParams();
  const token = searchParams.get("token") || "";
  const [convention, setConvention] = useState<StageConvention | null>(null);
  const [readOnly, setReadOnly] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);

  const load = useCallback(async () => {
    if (!token) {
      setError("Lien incomplet.");
      return;
    }
    setError(null);
    const res = await fetch(`/api/stages/public/student?token=${encodeURIComponent(token)}`, {
      cache: "no-store",
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Lien invalide");
    setConvention(data.convention);
    setReadOnly(data.readOnly === true);
  }, [token]);

  useEffect(() => {
    void load().catch((e: unknown) => setError(e instanceof Error ? e.message : "Erreur"));
  }, [load]);

  async function save(action: "save" | "submit") {
    if (!convention) return;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/student", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ token, action, convention }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");
      setConvention(data.convention);
      if (action === "submit") setDone(true);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  if (!convention && !error) {
    return <main className="min-h-screen flex items-center justify-center p-6">Chargement…</main>;
  }

  if (error && !convention) {
    return (
      <main className="min-h-screen flex items-center justify-center p-6">
        <p className="text-rose-700">{error}</p>
      </main>
    );
  }

  if (!convention) return null;

  const c = convention;

  return (
    <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
      <div className="mx-auto max-w-2xl rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h1 className="text-2xl font-black text-[#1F3D2B]">Préconvention de stage</h1>
        <p className="mt-2 text-sm text-stone-600">
          Statut : {STAGE_CONVENTION_STATUS_LABELS[c.status]}
        </p>

        {done && (
          <p className="mt-4 rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800">
            Préconvention envoyée à l&apos;administratif pour validation. Vous serez notifié une fois
            la convention prête à signer.
          </p>
        )}
        {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}

        {!readOnly && !done && (
          <div className="mt-6">
            <StagePreconventionForm
              convention={c}
              onChange={setConvention}
              onSave={() => void save("save")}
              onSubmit={() => void save("submit")}
              busy={busy}
            />
          </div>
        )}

        {readOnly && (
          <div className="mt-6 text-sm text-stone-600 space-y-2">
            <p>
              <strong>
                {c.student.firstName} {c.student.lastName}
              </strong>{" "}
              — {c.student.className} ({c.student.level})
            </p>
            <p>
              <strong>Entreprise :</strong> {c.company.name}
            </p>
            <p>
              <strong>Période :</strong> {c.schedule.periodStart} → {c.schedule.periodEnd}
            </p>
            <p>
              <strong>Horaires :</strong> {scheduleSummary(c.schedule)}
            </p>
          </div>
        )}
      </div>
    </main>
  );
}

export default function StageElevePage() {
  return (
    <Suspense fallback={<main className="p-8">Chargement…</main>}>
      <EleveContent />
    </Suspense>
  );
}
