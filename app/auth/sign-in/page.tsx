"use client";

import { useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { authClient } from "@/app/lib/auth-client";

export default function BetterAuthSignInPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    const { error: signInError } = await authClient.signIn.email({
      email: email.trim(),
      password,
      callbackURL: redirectTo,
    });
    setLoading(false);
    if (signInError) {
      const msg = signInError.message || "Connexion impossible.";
      if (/not found|invalid|incorrect|credential/i.test(msg)) {
        setError(
          "Identifiants incorrects. Si ton compte est déjà provisionné, choisis d’abord un mot de passe via « Activer mon compte ».",
        );
      } else {
        setError(msg);
      }
      return;
    }
    router.push(redirectTo);
    router.refresh();
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-md space-y-4 rounded-2xl border border-emerald-100 bg-white p-8 shadow-xl shadow-emerald-900/10"
      >
        <div className="flex items-start gap-3">
          <div
            className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-[#2F6B4A] to-[#1E4A32] text-lg font-black text-white shadow-md"
            aria-hidden
          >
            IA
          </div>
          <div>
            <h1 className="text-xl font-semibold text-emerald-950">Connexion intranet</h1>
            <p className="mt-1 text-sm text-emerald-800/70">
              Identifiants ScolIA (e-mail et mot de passe)
            </p>
          </div>
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
        <label className="block space-y-1 text-sm">
          <span className="font-medium text-emerald-950">Mot de passe</span>
          <input
            type="password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
          />
        </label>
        <button
          type="submit"
          disabled={loading}
          className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-medium text-white hover:brightness-110 disabled:opacity-60"
        >
          {loading ? "Connexion…" : "Se connecter"}
        </button>
        <p className="text-center text-sm text-emerald-800/70">
          Compte déjà provisionné ?{" "}
          <a
            href="/auth/sign-up"
            className="font-medium text-emerald-700 underline-offset-2 hover:underline"
          >
            Activer mon compte
          </a>
        </p>
      </form>
    </div>
  );
}
