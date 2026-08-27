"use client";

import Link from "next/link";
import { useState } from "react";
import { authClient } from "@/app/lib/auth-client";

export default function ForgotPasswordPage() {
  const [email, setEmail] = useState("");
  const [loading, setLoading] = useState(false);
  const [sent, setSent] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    const { error: resetError } = await authClient.requestPasswordReset({
      email: email.trim(),
      redirectTo: `${window.location.origin}/auth/reset-password`,
    });
    setLoading(false);
    if (resetError) {
      setError(resetError.message || "Envoi impossible. Réessayez dans quelques minutes.");
      return;
    }
    setSent(true);
  }

  if (sent) {
    return (
      <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
        <div
          className="w-full max-w-md space-y-4 rounded-2xl border border-emerald-200 bg-white p-8 text-center shadow-xl"
          role="status"
        >
          <h1 className="text-xl font-semibold text-emerald-950">E-mail envoyé</h1>
          <p className="text-sm text-emerald-800/80">
            Si un compte existe pour <strong>{email.trim()}</strong>, vous recevrez un lien pour
            créer un nouveau mot de passe (valable 24 heures). Pensez à vérifier les spams.
          </p>
          <Link
            href="/auth/sign-in"
            className="inline-block text-sm font-medium text-[#2F6B4A] underline underline-offset-2"
          >
            Retour à la connexion
          </Link>
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
          <h1 className="text-xl font-semibold text-emerald-950">Mot de passe oublié</h1>
          <p className="mt-2 text-sm text-emerald-800/70">
            Saisissez l’e-mail de votre compte ScolIA. Vous recevrez un lien pour créer un nouveau
            mot de passe et activer la double authentification.
          </p>
        </div>
        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-emerald-950">E-mail</span>
          <input
            type="email"
            autoComplete="email"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-medium text-white hover:brightness-110 disabled:opacity-60"
        >
          {loading ? "Envoi…" : "Recevoir le lien d’activation"}
        </button>
        <p className="text-center text-sm">
          <Link
            href="/auth/sign-in"
            className="font-medium text-[#2F6B4A] underline underline-offset-2"
          >
            Retour à la connexion
          </Link>
        </p>
      </form>
    </div>
  );
}
