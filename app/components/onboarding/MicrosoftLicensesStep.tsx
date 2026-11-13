"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import Link from "@/app/components/ui/AppLink";
import { useRouter } from "next/navigation";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import {
  MAX_A3_LICENSES,
  MICROSOFT_LICENSE_ONBOARDING,
  type MicrosoftLicensePerson,
  type MicrosoftLicenseRequest,
  type MicrosoftLicenseType,
} from "@/app/lib/microsoft-license-types";

type PersonDraft = Omit<MicrosoftLicensePerson, "id">;

const inputClass =
  "w-full rounded-xl border border-stone-200 bg-white px-3 py-2.5 text-sm outline-none focus:border-[#2F6B4A] focus:ring-2 focus:ring-emerald-100";

function emptyPerson(licenseType: MicrosoftLicenseType = "A1"): PersonDraft {
  return {
    firstName: "",
    lastName: "",
    email: "",
    jobRole: "",
    licenseType,
  };
}

export default function MicrosoftLicensesStep() {
  const router = useRouter();
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [submitted, setSubmitted] = useState<MicrosoftLicenseRequest | null>(null);
  const [notes, setNotes] = useState("");
  const [people, setPeople] = useState<PersonDraft[]>([emptyPerson("A3"), emptyPerson("A1")]);

  const a3Count = useMemo(() => people.filter((p) => p.licenseType === "A3").length, [people]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch("/api/onboarding/microsoft-licenses", { cache: "no-store" });
      const j = await res.json();
      if (res.ok && j.request) {
        setSubmitted(j.request);
        setPeople(
          j.request.people.map((p: MicrosoftLicensePerson) => ({
            firstName: p.firstName,
            lastName: p.lastName,
            email: p.email,
            jobRole: p.jobRole,
            licenseType: p.licenseType,
          })),
        );
        setNotes(j.request.notes || "");
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const updatePerson = (idx: number, patch: Partial<PersonDraft>) => {
    setPeople((prev) => prev.map((p, i) => (i === idx ? { ...p, ...patch } : p)));
  };

  const addPerson = (licenseType: MicrosoftLicenseType) => {
    if (licenseType === "A3" && a3Count >= MAX_A3_LICENSES) return;
    setPeople((prev) => [...prev, emptyPerson(licenseType)]);
  };

  const removePerson = (idx: number) => {
    setPeople((prev) => prev.filter((_, i) => i !== idx));
  };

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await fetch("/api/onboarding/microsoft-licenses", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ people, notes }),
      });
      const j = await res.json();
      if (!res.ok) throw new Error(j.error || "Envoi impossible");
      setSubmitted(j.request);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Erreur");
    } finally {
      setBusy(false);
    }
  };

  if (loading) {
    return <div className="p-12 text-center text-stone-500">Chargement…</div>;
  }

  return (
    <div className="min-h-screen bg-gradient-to-b from-[#F7F4EF] via-white to-[#EEF5F0] py-10 px-4">
      <div className="max-w-2xl mx-auto">
        <div className="mb-8">
          <p className="text-xs font-semibold uppercase tracking-wider text-[#2F6B4A] mb-1">
            Onboarding · Licences Microsoft
          </p>
          <h1 className="text-2xl font-black text-[#14231A]">
            <span className={SCOLA_GRADIENT_TEXT}>{MICROSOFT_LICENSE_ONBOARDING.title}</span>
          </h1>
          <p className="mt-3 text-sm text-stone-600 leading-relaxed">{MICROSOFT_LICENSE_ONBOARDING.intro}</p>
          <ul className="mt-4 space-y-2 text-sm text-stone-700">
            {MICROSOFT_LICENSE_ONBOARDING.bullets.map((b) => (
              <li key={b} className="flex gap-2">
                <span className="text-[#2F6B4A] font-bold">✓</span>
                <span>{b}</span>
              </li>
            ))}
          </ul>
        </div>

        <div className="bg-white/90 rounded-2xl border border-stone-200 shadow-sm p-6 md:p-8">
          {submitted && (
            <div className="mb-5 text-sm text-emerald-900 bg-emerald-50 border border-emerald-200 rounded-xl p-4">
              Demande envoyée le {new Date(submitted.submittedAt).toLocaleString("fr-FR")}. Notre équipe et le
              revendeur CSP traiteront l&apos;attribution sous quelques jours ouvrés. Vous pouvez modifier et
              renvoyer la liste si besoin.
            </div>
          )}

          {error && (
            <div className="mb-4 text-sm text-red-700 bg-red-50 border border-red-200 rounded-xl p-3">{error}</div>
          )}

          <div className="flex flex-wrap gap-2 mb-6">
            <button
              type="button"
              className="text-xs font-semibold rounded-lg border border-[#2F6B4A]/30 text-[#2F6B4A] px-3 py-1.5 hover:bg-emerald-50 disabled:opacity-40"
              disabled={a3Count >= MAX_A3_LICENSES}
              onClick={() => addPerson("A3")}
            >
              + Licence A3 ({a3Count}/{MAX_A3_LICENSES})
            </button>
            <button
              type="button"
              className="text-xs font-semibold rounded-lg border border-stone-300 text-stone-700 px-3 py-1.5 hover:bg-stone-50"
              onClick={() => addPerson("A1")}
            >
              + Enseignant A1
            </button>
          </div>

          <div className="space-y-4">
            {people.map((person, idx) => (
              <div key={idx} className="rounded-xl border border-stone-200 bg-stone-50/60 p-4 space-y-3">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-xs font-bold text-stone-500">
                    Personne {idx + 1} · {person.licenseType === "A3" ? "A3 (référent)" : "A1 (enseignant)"}
                  </span>
                  {people.length > 1 && (
                    <button type="button" className="text-xs text-red-600" onClick={() => removePerson(idx)}>
                      Retirer
                    </button>
                  )}
                </div>
                <div className="grid sm:grid-cols-2 gap-3">
                  <label className="block text-sm">
                    <span className="text-stone-600">Prénom</span>
                    <input
                      className={`${inputClass} mt-1`}
                      value={person.firstName}
                      onChange={(e) => updatePerson(idx, { firstName: e.target.value })}
                    />
                  </label>
                  <label className="block text-sm">
                    <span className="text-stone-600">Nom</span>
                    <input
                      className={`${inputClass} mt-1`}
                      value={person.lastName}
                      onChange={(e) => updatePerson(idx, { lastName: e.target.value })}
                    />
                  </label>
                </div>
                <label className="block text-sm">
                  <span className="text-stone-600">E-mail professionnel</span>
                  <input
                    className={`${inputClass} mt-1`}
                    type="email"
                    value={person.email}
                    onChange={(e) => updatePerson(idx, { email: e.target.value })}
                  />
                </label>
                <label className="block text-sm">
                  <span className="text-stone-600">Fonction / rôle</span>
                  <input
                    className={`${inputClass} mt-1`}
                    placeholder="ex. Directeur, Comptable, Professeur de maths"
                    value={person.jobRole}
                    onChange={(e) => updatePerson(idx, { jobRole: e.target.value })}
                  />
                </label>
                <label className="block text-sm">
                  <span className="text-stone-600">Type de licence</span>
                  <select
                    className={`${inputClass} mt-1`}
                    value={person.licenseType}
                    onChange={(e) => updatePerson(idx, { licenseType: e.target.value as MicrosoftLicenseType })}
                  >
                    <option value="A3">A3 — référent / admin / compta (max. {MAX_A3_LICENSES})</option>
                    <option value="A1">A1 — enseignant</option>
                  </select>
                </label>
              </div>
            ))}
          </div>

          <label className="block mt-6 text-sm">
            <span className="text-stone-600">Notes pour l&apos;équipe Scola (optionnel)</span>
            <textarea
              className={`${inputClass} mt-1 min-h-[80px]`}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder="Ex. remplacement d'un référent partant, domaine e-mail @ac-…"
            />
          </label>

          <div className="flex flex-wrap justify-between gap-3 mt-8 pt-6 border-t border-stone-100">
            <Link href="/dashboard" className="px-4 py-2 text-sm text-stone-600 hover:text-stone-900">
              Passer pour l&apos;instant
            </Link>
            <button
              type="button"
              className="px-5 py-2.5 rounded-xl bg-[#2F6B4A] text-white text-sm font-semibold disabled:opacity-50 hover:bg-[#255A3D]"
              disabled={busy}
              onClick={submit}
            >
              {busy ? "Envoi…" : submitted ? "Mettre à jour la demande" : "Envoyer la liste"}
            </button>
          </div>

          {!submitted && (
            <button
              type="button"
              className="mt-3 w-full text-center text-sm text-stone-500 hover:text-stone-700"
              onClick={() => router.push("/dashboard")}
            >
              Configurer plus tard depuis le tableau de bord
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
