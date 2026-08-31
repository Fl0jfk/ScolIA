"use client";

import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useEffect, useState } from "react";
import PasswordRequirementsChecklist from "@/app/components/auth/PasswordRequirementsChecklist";
import { authClient } from "@/app/lib/auth-client";
import { validatePasswordPolicy } from "@/app/lib/password-policy";

function ResetPasswordForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const token = searchParams.get("token") ?? "";
  const errorParam = searchParams.get("error");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    if (errorParam === "INVALID_TOKEN") {
      setError("Ce lien n’est plus valide ou a expiré. Demandez un nouveau lien.");
    }
  }, [errorParam]);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!token) {
      setError("Lien invalide. Demandez un nouveau lien d’activation.");
      return;
    }
    const policy = validatePasswordPolicy(newPassword);
    if (!policy.ok) {
      setError(policy.error);
      return;
    }
    if (newPassword !== confirmPassword) {
      setError("La confirmation ne correspond pas.");
      return;
    }
    setBusy(true);
    const { error: resetError } = await authClient.resetPassword({
      newPassword,
      token,
    });
    if (resetError) {
      setError(resetError.message || "Impossible de définir le mot de passe. Demandez un nouveau lien.");
      setBusy(false);
      return;
    }
    setSuccess(true);
    window.setTimeout(() => {
      router.replace("/auth/sign-in");
      router.refresh();
    }, 2000);
  }

  if (!token && !errorParam) {
    return (
      <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
        <div className="w-full max-w-md space-y-4 rounded-2xl border border-red-200 bg-white p-8 text-center shadow-xl">
          <h1 className="text-xl font-semibold text-red-900">Lien invalide</h1>
          <p className="text-sm text-red-800/80">
            Ce lien d’activation est incomplet. Utilisez le lien reçu par e-mail ou demandez-en un
            nouveau.
          </p>
          <Link
            href="/auth/forgot-password"
            className="inline-block text-sm font-medium text-[#2F6B4A] underline underline-offset-2"
          >
            Demander un nouveau lien
          </Link>
        </div>
      </div>
    );
  }

  if (success) {
    return (
      <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
        <div
          className="w-full max-w-md space-y-3 rounded-2xl border border-emerald-200 bg-white p-8 text-center shadow-xl"
          role="status"
        >
          <p className="text-lg font-semibold text-emerald-900">Mot de passe créé</p>
          <p className="text-sm text-emerald-800/80">
            Vous pouvez vous connecter avec votre e-mail et votre mot de passe.
            La double authentification n’est obligatoire que pour la direction et le personnel
            administratif. Si vous l’aviez déjà activée, vous continuerez à saisir votre code
            comme avant.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-md space-y-4 rounded-2xl border border-emerald-100 bg-white p-8 shadow-xl shadow-emerald-900/10"
      >
        <div>
          <h1 className="text-xl font-semibold text-emerald-950">Créer votre mot de passe</h1>
          <p className="mt-2 text-sm text-emerald-800/70">
            Choisissez un mot de passe personnel. Ensuite, connectez-vous avec votre e-mail et
            ce mot de passe. La double authentification n’est obligatoire que pour la direction
            et le personnel administratif ; les professeurs, surveillants et CPE peuvent s’en
            passer.
          </p>
        </div>
        <PasswordRequirementsChecklist password={newPassword} />
        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-emerald-950">Nouveau mot de passe</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={12}
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-emerald-950">Confirmer</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={12}
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={busy}
          className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-medium text-white hover:brightness-110 disabled:opacity-60"
        >
          {busy ? "Enregistrement…" : "Enregistrer et se connecter"}
        </button>
        <p className="text-center text-sm">
          <Link
            href="/auth/forgot-password"
            className="font-medium text-[#2F6B4A] underline underline-offset-2"
          >
            Demander un nouveau lien
          </Link>
        </p>
      </form>
    </div>
  );
}

export default function ResetPasswordPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-sm">Chargement…</p>}>
      <ResetPasswordForm />
    </Suspense>
  );
}
