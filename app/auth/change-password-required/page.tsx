"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import PasswordInput from "@/app/components/auth/PasswordInput";
import PasswordRequirementsChecklist from "@/app/components/auth/PasswordRequirementsChecklist";
import SwitchAccountLink from "@/app/components/auth/SwitchAccountLink";
import { validatePasswordPolicy } from "@/app/lib/password-policy";

function ChangePasswordRequiredForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
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
    try {
      const res = await fetch("/api/account/security", {
        method: "POST",
        credentials: "include",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "password",
          currentPassword,
          newPassword,
        }),
      });
      const j = (await res.json()) as { error?: string };
      if (!res.ok) throw new Error(j.error || "Échec");
      setSuccess(true);
      window.setTimeout(() => {
        router.replace(redirectTo.startsWith("/") ? redirectTo : "/dashboard");
        router.refresh();
      }, 1200);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
      setBusy(false);
    }
  }

  if (success) {
    return (
      <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
        <div
          className="w-full max-w-md space-y-3 rounded-2xl border border-emerald-200 bg-white p-8 text-center shadow-xl"
          role="status"
        >
          <p className="text-lg font-semibold text-emerald-900">
            Mot de passe mis à jour
          </p>
          <p className="text-sm text-emerald-800/80">
            Redirection vers l’intranet…
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-md space-y-4 rounded-2xl border border-amber-200 bg-white p-8 shadow-xl"
      >
        <div>
          <h1 className="text-xl font-semibold text-amber-950">Sécurité du compte</h1>
          <p className="mt-2 text-sm text-amber-900/80">
            Pour protéger les données de l’établissement, vous devez définir un mot de passe
            personnel avant d’accéder à l’intranet.
          </p>
        </div>
        <PasswordRequirementsChecklist password={newPassword} tone="amber" />
        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-slate-800">Mot de passe actuel (provisoire)</span>
          <PasswordInput
            autoComplete="current-password"
            required
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-slate-800">Nouveau mot de passe</span>
          <PasswordInput
            autoComplete="new-password"
            required
            minLength={12}
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-slate-800">Confirmer</span>
          <PasswordInput
            autoComplete="new-password"
            required
            minLength={12}
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            className="rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={busy}
          className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
        >
          {busy ? "Enregistrement…" : "Enregistrer et continuer"}
        </button>
        <SwitchAccountLink />
      </form>
    </div>
  );
}

export default function ChangePasswordRequiredPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-sm">Chargement…</p>}>
      <ChangePasswordRequiredForm />
    </Suspense>
  );
}
