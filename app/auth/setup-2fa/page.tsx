"use client";

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import QRCode from "qrcode";
import { authClient } from "@/app/lib/auth-client";

type Step = "password" | "verify" | "done";

function Setup2faForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const [step, setStep] = useState<Step>("password");
  const [password, setPassword] = useState("");
  const [totpUri, setTotpUri] = useState<string | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [backupCodes, setBackupCodes] = useState<string[]>([]);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!totpUri) {
      setQrDataUrl(null);
      return;
    }
    let cancelled = false;
    void QRCode.toDataURL(totpUri, { width: 220, margin: 1 }).then((url) => {
      if (!cancelled) setQrDataUrl(url);
    });
    return () => {
      cancelled = true;
    };
  }, [totpUri]);

  async function enable(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const { data, error: enableError } = await authClient.twoFactor.enable({
        password,
      });
      if (enableError) throw new Error(enableError.message || "Activation impossible.");
      const uri =
        data && "totpURI" in data && typeof data.totpURI === "string" ? data.totpURI : null;
      const codes =
        data && "backupCodes" in data && Array.isArray(data.backupCodes)
          ? data.backupCodes.map(String)
          : [];
      if (!uri) throw new Error("URI TOTP manquante.");
      setTotpUri(uri);
      setBackupCodes(codes);
      setStep("verify");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function verify(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const { error: verifyError } = await authClient.twoFactor.verifyTotp({
        code: code.trim(),
      });
      if (verifyError) throw new Error(verifyError.message || "Code invalide.");
      void fetch("/api/account/security-event", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ action: "two_factor_enabled" }),
      });
      setStep("done");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <div className="w-full max-w-lg space-y-4 rounded-2xl border border-amber-200 bg-white p-8 shadow-xl">
        <div>
          <h1 className="text-xl font-semibold text-amber-950">Sécurité renforcée (2FA)</h1>
          <p className="mt-2 text-sm text-amber-900/80">
            Tous les comptes du personnel doivent activer une application d’authentification
            (Google Authenticator, Authy, etc.) pour sécuriser l’accès à l’intranet.
          </p>
        </div>

        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}

        {step === "password" ? (
          <form onSubmit={enable} className="space-y-3">
            <label className="block space-y-1 text-sm">
              <span className="font-medium text-slate-800">Mot de passe actuel</span>
              <input
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
              />
            </label>
            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
            >
              {busy ? "Génération…" : "Générer le QR code"}
            </button>
          </form>
        ) : null}

        {step === "verify" ? (
          <form onSubmit={verify} className="space-y-4">
            {qrDataUrl ? (
              <div className="flex justify-center">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={qrDataUrl} alt="QR code 2FA" className="rounded-lg border border-slate-200" />
              </div>
            ) : null}
            {backupCodes.length > 0 ? (
              <div className="rounded-xl border border-amber-100 bg-amber-50/60 p-3">
                <p className="text-xs font-semibold uppercase tracking-wide text-amber-900">
                  Codes de secours (à conserver)
                </p>
                <ul className="mt-2 grid grid-cols-2 gap-1 font-mono text-xs text-amber-950">
                  {backupCodes.map((c) => (
                    <li key={c}>{c}</li>
                  ))}
                </ul>
              </div>
            ) : null}
            <label className="block space-y-1 text-sm">
              <span className="font-medium text-slate-800">Code à 6 chiffres</span>
              <input
                type="text"
                inputMode="numeric"
                autoComplete="one-time-code"
                required
                value={code}
                onChange={(e) => setCode(e.target.value)}
                className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
              />
            </label>
            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
            >
              {busy ? "Vérification…" : "Activer la 2FA"}
            </button>
          </form>
        ) : null}

        {step === "done" ? (
          <div className="space-y-3">
            <p className="rounded-lg bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
              Double authentification activée. Conservez vos codes de secours hors ligne.
            </p>
            <button
              type="button"
              onClick={() => {
                router.replace(redirectTo);
                router.refresh();
              }}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white"
            >
              Continuer vers l’intranet
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}

export default function Setup2faPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-sm">Chargement…</p>}>
      <Setup2faForm />
    </Suspense>
  );
}
