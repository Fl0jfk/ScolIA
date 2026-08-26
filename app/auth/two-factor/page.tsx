"use client";

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { authClient } from "@/app/lib/auth-client";

type TrustPolicy = {
  allowTrust: boolean;
  maxAgeSeconds: number;
  label: string;
  hint: string;
};

function TwoFactorForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const emailHint = searchParams.get("email")?.trim() || "";
  const [code, setCode] = useState("");
  const [useBackup, setUseBackup] = useState(false);
  const [trustDevice, setTrustDevice] = useState(true);
  const [policy, setPolicy] = useState<TrustPolicy | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const qs = emailHint ? `?email=${encodeURIComponent(emailHint)}` : "";
        const res = await fetch(`/api/auth/mfa-trust-policy${qs}`, {
          credentials: "include",
          cache: "no-store",
        });
        if (!res.ok) return;
        const j = (await res.json()) as TrustPolicy;
        if (cancelled) return;
        setPolicy(j);
        setTrustDevice(j.allowTrust);
      } catch {
        /* garde le défaut 30 j côté UI ; le serveur applique la vraie politique */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [emailHint]);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const trust = Boolean(policy?.allowTrust && trustDevice);
      if (useBackup) {
        const { error: verifyError } = await authClient.twoFactor.verifyBackupCode({
          code: code.trim(),
          trustDevice: trust,
        });
        if (verifyError) throw new Error(verifyError.message || "Code de secours invalide.");
      } else {
        const { error: verifyError } = await authClient.twoFactor.verifyTotp({
          code: code.trim(),
          trustDevice: trust,
        });
        if (verifyError) throw new Error(verifyError.message || "Code invalide.");
      }
      router.replace(redirectTo);
      router.refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  const allowTrust = policy?.allowTrust !== false;
  const trustLabel = policy?.label || "Se souvenir de cet appareil 30 jours";
  const trustHint =
    policy?.hint ||
    "Sur cet appareil, la MFA ne sera pas redemandée pendant 30 jours.";

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-md space-y-4 rounded-2xl border border-emerald-100 bg-white p-8 shadow-xl"
      >
        <div>
          <h1 className="text-xl font-semibold text-emerald-950">Double authentification</h1>
          <p className="mt-2 text-sm text-emerald-800/80">
            {useBackup
              ? "Saisissez un code de secours à usage unique."
              : "Saisissez le code à 6 chiffres de votre application d’authentification."}
          </p>
        </div>
        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-emerald-950">
            {useBackup ? "Code de secours" : "Code TOTP"}
          </span>
          <input
            type="text"
            inputMode={useBackup ? "text" : "numeric"}
            autoComplete="one-time-code"
            required
            value={code}
            onChange={(e) => setCode(e.target.value)}
            className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
          />
        </label>
        {allowTrust ? (
          <label className="flex items-start gap-2 text-sm text-slate-700">
            <input
              type="checkbox"
              checked={trustDevice}
              onChange={(e) => setTrustDevice(e.target.checked)}
              className="mt-0.5 rounded border-slate-300"
            />
            <span>
              <span className="font-medium text-slate-800">{trustLabel}</span>
              <span className="mt-0.5 block text-xs text-slate-500">{trustHint}</span>
            </span>
          </label>
        ) : (
          <p className="rounded-xl border border-amber-100 bg-amber-50 px-3 py-2 text-xs text-amber-950">
            {trustHint}
          </p>
        )}
        <button
          type="submit"
          disabled={busy}
          className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
        >
          {busy ? "Vérification…" : "Valider"}
        </button>
        <button
          type="button"
          onClick={() => {
            setUseBackup((v) => !v);
            setCode("");
            setError(null);
          }}
          className="w-full text-center text-sm font-medium text-emerald-700 underline-offset-2 hover:underline"
        >
          {useBackup ? "Utiliser le code de l’application" : "Utiliser un code de secours"}
        </button>
      </form>
    </div>
  );
}

export default function TwoFactorPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-sm">Chargement…</p>}>
      <TwoFactorForm />
    </Suspense>
  );
}
