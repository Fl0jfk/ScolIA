"use client";

import { Suspense, useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import QRCode from "qrcode";
import PasswordInput from "@/app/components/auth/PasswordInput";
import SwitchAccountLink from "@/app/components/auth/SwitchAccountLink";
import { authClient } from "@/app/lib/auth-client";
import { roleRequiresTwoFactor } from "@/app/lib/two-factor-policy";

type Step = "password" | "verify" | "done";

async function prepareTwoFactorSetup(): Promise<void> {
  try {
    await fetch("/api/account/two-factor/prepare", {
      method: "POST",
      credentials: "include",
    });
  } catch {
    /* best-effort : enable() pourra encore échouer proprement */
  }
}

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
  const [canSkipMfa, setCanSkipMfa] = useState(false);

  useEffect(() => {
    void prepareTwoFactorSetup();
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/auth/me", { credentials: "include" });
        const data = (await res.json()) as {
          user?: { roles?: string[]; orgAdmin?: boolean; platformAdmin?: boolean } | null;
        };
        const u = data.user;
        if (!u || cancelled) return;
        setCanSkipMfa(
          !roleRequiresTwoFactor({
            platformAdmin: Boolean(u.platformAdmin),
            orgAdmin: Boolean(u.orgAdmin),
            roles: Array.isArray(u.roles) ? u.roles : [],
          }),
        );
      } catch {
        /* si /me échoue, on n’affiche pas « Passer » (sécurité) */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

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

  function resetToPasswordStep() {
    setStep("password");
    setTotpUri(null);
    setQrDataUrl(null);
    setBackupCodes([]);
    setCode("");
    setError(null);
  }

  async function restartSetup() {
    setBusy(true);
    setError(null);
    try {
      await prepareTwoFactorSetup();
      resetToPasswordStep();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Impossible de recommencer.");
    } finally {
      setBusy(false);
    }
  }

  async function skipSetup() {
    setBusy(true);
    setError(null);
    try {
      await prepareTwoFactorSetup();
      router.replace(redirectTo);
      router.refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Impossible de continuer sans MFA.");
      setBusy(false);
    }
  }

  async function enable(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      // Purge un éventuel secret orphelin (abandon précédent) avant de régénérer.
      await prepareTwoFactorSetup();
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
      // Filet : force twoFactorEnabled si Better-Auth a accepté le code sans promouvoir le flag.
      const completeRes = await fetch("/api/account/security-event", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ action: "two_factor_enabled" }),
      });
      if (!completeRes.ok) {
        throw new Error("Code accepté mais activation MFA incomplète. Réessayez ou recommencez.");
      }
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
            {canSkipMfa
              ? "La double authentification est recommandée, mais facultative pour les professeurs, surveillants et CPE. Direction et personnel administratif doivent l’activer. Si vous l’activez, elle restera demandée à chaque connexion."
              : "Les comptes direction et personnel administratif doivent activer une application d’authentification (Google Authenticator, Authy, etc.) pour sécuriser l’accès à l’intranet."}
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
              <PasswordInput
                autoComplete="current-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
              />
            </label>
            <button
              type="submit"
              disabled={busy}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
            >
              {busy ? "Génération…" : "Générer le QR code"}
            </button>
            {canSkipMfa ? (
              <button
                type="button"
                disabled={busy}
                onClick={() => void skipSetup()}
                className="w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-semibold text-slate-700 disabled:opacity-60"
              >
                Passer cette étape
              </button>
            ) : null}
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
            <button
              type="button"
              disabled={busy}
              onClick={() => void restartSetup()}
              className="w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-semibold text-slate-700 disabled:opacity-60"
            >
              Recommencer (nouveau QR)
            </button>
            {canSkipMfa ? (
              <button
                type="button"
                disabled={busy}
                onClick={() => void skipSetup()}
                className="w-full rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-sm font-semibold text-slate-600 disabled:opacity-60"
              >
                Passer sans activer
              </button>
            ) : null}
            <p className="text-xs text-slate-500">
              Si vous aviez déjà scanné un ancien QR, utilisez « Recommencer » puis scannez uniquement
              le nouveau code.
            </p>
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

        {step !== "done" ? <SwitchAccountLink /> : null}
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
