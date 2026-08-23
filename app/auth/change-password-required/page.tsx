"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

function ChangePasswordRequiredForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (newPassword.length < 10) {
      setError("Le nouveau mot de passe doit contenir au moins 10 caractères.");
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
      router.replace(redirectTo);
      router.refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur");
    } finally {
      setBusy(false);
    }
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
        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-slate-800">Mot de passe actuel (provisoire)</span>
          <input
            type="password"
            autoComplete="current-password"
            required
            value={currentPassword}
            onChange={(e) => setCurrentPassword(e.target.value)}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-slate-800">Nouveau mot de passe (min. 10)</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={10}
            value={newPassword}
            onChange={(e) => setNewPassword(e.target.value)}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
          />
        </label>
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-slate-800">Confirmer</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={10}
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            className="w-full rounded-xl border border-slate-200 px-3 py-2 outline-none ring-amber-200 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={busy}
          className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-60"
        >
          {busy ? "Enregistrement…" : "Enregistrer et continuer"}
        </button>
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
