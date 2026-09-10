"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";
import QRCode from "qrcode";
import PasswordInput from "@/app/components/auth/PasswordInput";
import SwitchAccountLink from "@/app/components/auth/SwitchAccountLink";
import { authClient } from "@/app/lib/auth-client";
import { resolvePasskeyRegisterHints } from "@/app/lib/passkey-client-hints";
import { roleRequiresTwoFactor } from "@/app/lib/two-factor-policy";

type Mode = "choose" | "passkey" | "totp-password" | "totp-verify" | "done";

async function prepareTwoFactorSetup(): Promise<void> {
  try {
    await fetch("/api/account/two-factor/prepare", {
      method: "POST",
      credentials: "include",
    });
  } catch {
    /* best-effort */
  }
}

function Setup2faForm() {
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const [mode, setMode] = useState<Mode>("choose");
  const [password, setPassword] = useState("");
  const [totpUri, setTotpUri] = useState<string | null>(null);
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [backupCodes, setBackupCodes] = useState<string[]>([]);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [canSkipMfa, setCanSkipMfa] = useState(false);
  const [passkeySupported, setPasskeySupported] = useState(true);
  const [passkeyHelp, setPasskeyHelp] = useState(
    "Sur PC, priorité au téléphone (QR). Sur mobile, Face ID / empreinte.",
  );

  useEffect(() => {
    void prepareTwoFactorSetup();
    setPasskeySupported(
      typeof window !== "undefined" &&
        typeof window.PublicKeyCredential !== "undefined",
    );
    setPasskeyHelp(resolvePasskeyRegisterHints().helpText);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const res = await fetch("/api/auth/me", { credentials: "include" });
        const data = (await res.json()) as {
          user?: {
            roles?: string[];
            orgAdmin?: boolean;
            platformAdmin?: boolean;
            mfaSatisfied?: boolean;
            hasPasskey?: boolean;
            twoFactorEnabled?: boolean;
          } | null;
        };
        const u = data.user;
        if (!u || cancelled) return;
        if (u.mfaSatisfied || u.hasPasskey || u.twoFactorEnabled) {
          window.location.assign(redirectTo);
          return;
        }
        setCanSkipMfa(
          !roleRequiresTwoFactor({
            platformAdmin: Boolean(u.platformAdmin),
            orgAdmin: Boolean(u.orgAdmin),
            roles: Array.isArray(u.roles) ? u.roles : [],
          }),
        );
      } catch {
        /* si /me échoue, on n’affiche pas « Passer » */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [redirectTo]);

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

  async function goToApp() {
    // Navigation hard : le proxy doit revoir hasPasskey sans cache RSC.
    window.location.assign(redirectTo);
  }

  async function skipSetup() {
    setBusy(true);
    setError(null);
    try {
      await prepareTwoFactorSetup();
      goToApp();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Impossible de continuer sans MFA.");
      setBusy(false);
    }
  }

  async function registerPasskey() {
    setBusy(true);
    setError(null);
    setMode("passkey");
    const hints = resolvePasskeyRegisterHints();
    try {
      const { data, error: regError } = await authClient.passkey.addPasskey({
        name: hints.name,
        authenticatorAttachment: hints.authenticatorAttachment,
      });
      if (regError) {
        throw new Error(
          regError.message ||
            (hints.authenticatorAttachment === "platform"
              ? "Enregistrement annulé. Validez Face ID / empreinte, ou utilisez l’appli OTP."
              : "Enregistrement annulé. Choisissez téléphone / QR dans la fenêtre du navigateur, ou utilisez l’appli OTP."),
        );
      }
      if (!data) throw new Error("Aucune passkey enregistrée.");
      await fetch("/api/account/security-event", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ action: "passkey_registered" }),
      });
      setMode("done");
      window.setTimeout(() => {
        goToApp();
      }, 600);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur passkey");
      setMode("choose");
      setBusy(false);
    }
  }

  async function enableTotp(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
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
      setMode("totp-verify");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
  }

  async function verifyTotp(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const { error: verifyError } = await authClient.twoFactor.verifyTotp({
        code: code.trim(),
      });
      if (verifyError) throw new Error(verifyError.message || "Code invalide.");
      const completeRes = await fetch("/api/account/security-event", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ action: "two_factor_enabled" }),
      });
      if (!completeRes.ok) {
        throw new Error("Code accepté mais activation MFA incomplète. Réessayez.");
      }
      setMode("done");
      window.setTimeout(() => {
        goToApp();
      }, 600);
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
          <h1 className="text-xl font-semibold text-amber-950">Sécurité renforcée</h1>
          <p className="mt-2 text-sm text-amber-900/80">
            {canSkipMfa
              ? "Recommandé pour sécuriser votre compte. Facultatif pour les professeurs, surveillants et CPE."
              : "Obligatoire pour la direction et le personnel administratif."}
          </p>
        </div>

        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}

        {mode === "choose" ? (
          <div className="space-y-3">
            <div className="rounded-xl border border-emerald-100 bg-emerald-50/60 p-4 text-sm text-emerald-950">
              <p className="font-semibold">Passkey</p>
              <p className="mt-1 text-emerald-900/80">{passkeyHelp}</p>
            </div>
            <button
              type="button"
              disabled={busy || !passkeySupported}
              onClick={() => void registerPasskey()}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
            >
              {busy ? "Enregistrement…" : "Enregistrer une passkey"}
            </button>
            {!passkeySupported ? (
              <p className="text-xs text-amber-800">
                Ce navigateur ne gère pas les passkeys. Utilisez Chrome / Edge / Safari récents, ou
                l’option appli OTP ci-dessous.
              </p>
            ) : null}
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setError(null);
                setMode("totp-password");
              }}
              className="w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-semibold text-slate-700 disabled:opacity-60"
            >
              Préférer une appli OTP (Google Authenticator…)
            </button>
            {canSkipMfa ? (
              <button
                type="button"
                disabled={busy}
                onClick={() => void skipSetup()}
                className="w-full rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-sm font-semibold text-slate-600 disabled:opacity-60"
              >
                Passer cette étape
              </button>
            ) : null}
            <SwitchAccountLink />
          </div>
        ) : null}

        {mode === "passkey" ? (
          <div className="space-y-3">
            <p className="text-sm text-slate-700">{passkeyHelp}</p>
            <p className="text-xs text-slate-500">Ne fermez pas cette page pendant la validation.</p>
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setBusy(false);
                setMode("choose");
              }}
              className="w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-semibold text-slate-700"
            >
              Annuler
            </button>
          </div>
        ) : null}

        {mode === "totp-password" ? (
          <form onSubmit={enableTotp} className="space-y-3">
            <p className="text-sm text-slate-600">
              Secours si vous ne pouvez pas utiliser de passkey. Vous aurez un QR à scanner dans
              une appli d’authentification.
            </p>
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
              {busy ? "Génération…" : "Générer le QR OTP"}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setError(null);
                setMode("choose");
              }}
              className="w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-semibold text-slate-700"
            >
              Retour
            </button>
            <SwitchAccountLink />
          </form>
        ) : null}

        {mode === "totp-verify" ? (
          <form onSubmit={verifyTotp} className="space-y-4">
            {qrDataUrl ? (
              <div className="flex justify-center">
                {/* eslint-disable-next-line @next/next/no-img-element */}
                <img src={qrDataUrl} alt="QR code OTP" className="rounded-lg border border-slate-200" />
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
              {busy ? "Vérification…" : "Activer l’OTP"}
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={() => {
                setTotpUri(null);
                setBackupCodes([]);
                setCode("");
                setMode("choose");
              }}
              className="w-full rounded-xl border border-slate-200 bg-slate-50 px-4 py-2.5 text-sm font-semibold text-slate-700"
            >
              Choisir une autre méthode
            </button>
            <SwitchAccountLink />
          </form>
        ) : null}

        {mode === "done" ? (
          <div className="space-y-3">
            <p className="rounded-lg bg-emerald-50 px-3 py-2 text-sm text-emerald-800">
              Sécurité activée. Redirection vers l’intranet…
            </p>
            <button
              type="button"
              onClick={() => goToApp()}
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
