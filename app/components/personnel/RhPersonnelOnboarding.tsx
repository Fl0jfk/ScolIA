"use client";

import { useState } from "react";
import RhStaffProfileFields from "@/app/components/personnel/RhStaffProfileFields";
import RhSelfDepositPanel from "@/app/components/personnel/RhSelfDepositPanel";
import type { PersonnelProfile } from "@/app/lib/personnel-profile";
import type { PersonnelRecord } from "@/app/lib/personnel-types";

type Props = {
  record: PersonnelRecord | null;
  identityComplete: boolean;
  onRefresh: () => Promise<void>;
};

export default function RhPersonnelOnboarding({ record, identityComplete, onRefresh }: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [step, setStep] = useState<"profile" | "documents">("profile");

  const initIfNeeded = async (): Promise<boolean> => {
    if (record) return true;
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/espace", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "init" }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Initialisation impossible");
      await onRefresh();
      return true;
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
      return false;
    } finally {
      setBusy(false);
    }
  };

  const saveProfile = async (fd: FormData) => {
    setBusy(true);
    setError(null);
    try {
      const ok = await initIfNeeded();
      if (!ok) return;

      const profile: PersonnelProfile = {
        birthDate: String(fd.get("birthDate") || "").trim() || null,
        birthPlace: String(fd.get("birthPlace") || "").trim() || null,
        birthName: String(fd.get("birthName") || "").trim() || null,
        nationality: String(fd.get("nationality") || "").trim() || null,
        gender: (String(fd.get("gender") || "") || null) as PersonnelProfile["gender"],
        socialSecurityNumber: String(fd.get("socialSecurityNumber") || "").trim() || null,
        addressLine1: String(fd.get("addressLine1") || "").trim() || null,
        addressLine2: String(fd.get("addressLine2") || "").trim() || null,
        postalCode: String(fd.get("postalCode") || "").trim() || null,
        city: String(fd.get("city") || "").trim() || null,
        country: String(fd.get("country") || "").trim() || null,
        phone: String(fd.get("phone") || "").trim() || null,
        phoneMobile: String(fd.get("phoneMobile") || "").trim() || null,
      };

      const res = await fetch("/api/rh/espace", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "update-profile", profile }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Enregistrement impossible");
      await onRefresh();
      setStep("documents");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  const submitForValidation = async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/rh/espace", {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ action: "submit-onboarding" }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Envoi impossible");
      await onRefresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-indigo-200 bg-gradient-to-br from-indigo-50 via-white to-violet-50/40 p-6 sm:p-8">
        <p className="text-[11px] font-black uppercase tracking-widest text-indigo-600">Bienvenue</p>
        <h2 className="text-2xl font-black text-slate-900 mt-1">Activez votre espace RH</h2>
        <p className="text-sm text-slate-600 mt-2 max-w-2xl">
          Avant d&apos;accéder à vos demandes d&apos;absence, HSE et documents, complétez votre dossier.
          Une fois validé par la RH ou l&apos;administratif de votre établissement, votre tableau de bord sera
          disponible.
        </p>
        <div className="mt-4 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={() => setStep("profile")}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold ${
              step === "profile" ? "bg-indigo-600 text-white" : "bg-white border border-slate-200 text-slate-600"
            }`}
          >
            1. Identité
          </button>
          <button
            type="button"
            onClick={() => setStep("documents")}
            className={`px-3 py-1.5 rounded-lg text-xs font-bold ${
              step === "documents" ? "bg-indigo-600 text-white" : "bg-white border border-slate-200 text-slate-600"
            }`}
          >
            2. Documents
          </button>
        </div>
      </section>

      {step === "profile" ? (
        <form
          className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            void saveProfile(new FormData(e.currentTarget));
          }}
        >
          <h3 className="font-black text-slate-900">Vos informations</h3>
          <RhStaffProfileFields profile={record?.profile} showContract={false} showBank={false} />
          {error && (
            <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2">{error}</p>
          )}
          <button
            type="submit"
            disabled={busy}
            className="px-5 py-2.5 rounded-xl bg-indigo-600 text-white text-sm font-bold disabled:opacity-60"
          >
            {busy ? "Enregistrement…" : "Enregistrer et continuer"}
          </button>
        </form>
      ) : (
        <div className="space-y-4">
          <RhSelfDepositPanel />
          <section className="rounded-2xl border border-slate-200 bg-white p-6 shadow-sm">
            <h3 className="font-black text-slate-900 mb-2">Soumettre à la RH</h3>
            <p className="text-sm text-slate-600 mb-4">
              Déposez au minimum votre pièce d&apos;identité et votre RIB si disponibles, puis envoyez votre dossier
              pour validation.
            </p>
            {!identityComplete && (
              <p className="text-sm text-amber-800 bg-amber-50 border border-amber-100 rounded-xl px-3 py-2 mb-4">
                Complétez d&apos;abord l&apos;étape identité (date de naissance, adresse, téléphone).
              </p>
            )}
            {error && (
              <p className="text-sm text-rose-700 bg-rose-50 border border-rose-100 rounded-xl px-3 py-2 mb-4">
                {error}
              </p>
            )}
            <button
              type="button"
              disabled={busy || !identityComplete}
              onClick={() => void submitForValidation()}
              className="px-5 py-2.5 rounded-xl bg-emerald-600 text-white text-sm font-bold disabled:opacity-60"
            >
              {busy ? "Envoi…" : "Envoyer mon dossier à la RH"}
            </button>
          </section>
        </div>
      )}
    </div>
  );
}
