"use client";

import { Suspense, useCallback, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import type { StageConvention } from "@/app/lib/stage-types";
import { STAGE_CONVENTION_STATUS_LABELS } from "@/app/lib/stage-types";
import { scheduleSummary } from "@/app/lib/stage-schedule";
import StagePreconventionForm from "@/app/components/stages/StagePreconventionForm";

function extractTokenFromStudentLink(studentLink: string): string | null {
  try {
    const url = studentLink.startsWith("http")
      ? new URL(studentLink)
      : new URL(studentLink, "http://local");
    return url.searchParams.get("token");
  } catch {
    return null;
  }
}

function StagePreconventionPublicContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const tokenFromUrl = searchParams.get("token") || "";

  const [token, setToken] = useState(tokenFromUrl);
  const [step, setStep] = useState<"identity" | "form">(tokenFromUrl ? "form" : "identity");
  const [ine, setIne] = useState("");
  const [dateNaissance, setDateNaissance] = useState("");
  const [convention, setConvention] = useState<StageConvention | null>(null);
  const [readOnly, setReadOnly] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState(false);

  const loadConvention = useCallback(async (activeToken: string) => {
    if (!activeToken) return;
    setError(null);
    const res = await fetch(`/api/stages/public/student?token=${encodeURIComponent(activeToken)}`, {
      cache: "no-store",
    });
    const data = await res.json();
    if (!res.ok) throw new Error(data?.error || "Lien invalide");
    setConvention(data.convention);
    setReadOnly(data.readOnly === true);
    setStep("form");
  }, []);

  useEffect(() => {
    if (tokenFromUrl) {
      setToken(tokenFromUrl);
      void loadConvention(tokenFromUrl).catch((e: unknown) =>
        setError(e instanceof Error ? e.message : "Erreur"),
      );
    }
  }, [tokenFromUrl, loadConvention]);

  async function verifyIdentity(e: React.FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/stages/public/preconvention", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ ine, dateNaissance }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data?.error || "Erreur");

      const newToken = extractTokenFromStudentLink(String(data.studentLink ?? ""));
      if (!newToken) throw new Error("Impossible d'ouvrir le formulaire.");

      setToken(newToken);
      router.replace(`/stages/preconvention?token=${encodeURIComponent(newToken)}`);
      await loadConvention(newToken);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function save(action: "save" | "submit") {
    if (!convention || !token) return;
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

  if (step === "form" && !convention && !error) {
    return <main className="min-h-screen flex items-center justify-center p-6">Chargement…</main>;
  }

  return (
    <main className="min-h-screen bg-[#f6f8f5] px-4 py-10">
      <div className="mx-auto max-w-2xl rounded-2xl border border-stone-200 bg-white p-6 shadow-sm">
        <h1 className="text-2xl font-black text-[#1F3D2B]">Préconvention de stage</h1>
        <p className="mt-2 text-sm text-stone-600">
          Formulaire en ligne : entreprise d&apos;accueil, horaires, dates et contacts. Aucun dépôt
          de PDF — vous remplissez tout directement ici.
        </p>

        {error && <p className="mt-4 text-sm text-rose-700">{error}</p>}

        {step === "identity" && !token && (
          <form onSubmit={(e) => void verifyIdentity(e)} className="mt-6 space-y-4 text-sm">
            <p className="rounded-lg border border-stone-200 bg-stone-50 px-3 py-2 text-xs text-stone-700">
              <strong>Étape 1 — Identification</strong> : INE et date de naissance (bulletin /
              Pronote). Ensuite vous accédez au formulaire complet.
            </p>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">
                Identifiant national élève (INE) *
              </span>
              <input
                className="mt-1 w-full rounded-lg border px-3 py-2 font-mono uppercase"
                placeholder="ex. 180123456AB"
                value={ine}
                onChange={(e) => setIne(e.target.value.toUpperCase())}
                autoComplete="off"
                required
              />
            </label>
            <label className="block">
              <span className="text-xs font-semibold text-stone-600">Date de naissance *</span>
              <input
                type="date"
                className="mt-1 w-full rounded-lg border px-3 py-2"
                value={dateNaissance}
                onChange={(e) => setDateNaissance(e.target.value)}
                required
              />
            </label>
            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-lg bg-[#2F6B4A] py-3 text-sm font-bold text-white disabled:opacity-50"
            >
              {busy ? "Vérification…" : "Continuer vers le formulaire →"}
            </button>
          </form>
        )}

        {step === "form" && convention && (
          <>
            <p className="mt-4 text-sm text-stone-600">
              <strong>Étape 2 — Dossier</strong> · {convention.student.firstName}{" "}
              {convention.student.lastName} ({convention.student.className}) · Statut :{" "}
              {STAGE_CONVENTION_STATUS_LABELS[convention.status]}
            </p>

            {done && (
              <p className="mt-4 rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-800">
                Préconvention envoyée à l&apos;administratif pour validation. Vous serez notifié une
                fois la convention prête à signer.
              </p>
            )}

            {!readOnly && !done && (
              <div className="mt-6" data-tour="stages-preconvention-form">
                <StagePreconventionForm
                  convention={convention}
                  onChange={setConvention}
                  onSave={() => void save("save")}
                  onSubmit={() => void save("submit")}
                  busy={busy}
                  identityLocked={Boolean(convention.ocrMeta?.matchedEleveIne)}
                />
              </div>
            )}

            {readOnly && (
              <div className="mt-6 text-sm text-stone-600 space-y-2">
                <p>
                  <strong>Entreprise :</strong> {convention.company.name}
                </p>
                <p>
                  <strong>Période :</strong> {convention.schedule.periodStart} →{" "}
                  {convention.schedule.periodEnd}
                </p>
                <p>
                  <strong>Horaires :</strong> {scheduleSummary(convention.schedule)}
                </p>
              </div>
            )}
          </>
        )}
      </div>
    </main>
  );
}

export default function StagePreconventionPublicClient() {
  return (
    <Suspense fallback={<main className="p-8">Chargement…</main>}>
      <StagePreconventionPublicContent />
    </Suspense>
  );
}
