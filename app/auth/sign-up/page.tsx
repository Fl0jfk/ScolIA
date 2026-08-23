"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { authClient } from "@/app/lib/auth-client";

export default function BetterAuthSignUpPage() {
  const router = useRouter();
  const [firstName, setFirstName] = useState("");
  const [lastName, setLastName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function claimMigratedAccount(): Promise<{ ok: boolean; error?: string }> {
    const res = await fetch("/api/auth/claim-migrated", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        email: email.trim(),
        password,
        firstName: firstName.trim() || undefined,
        lastName: lastName.trim() || undefined,
      }),
    });
    const data = (await res.json().catch(() => ({}))) as { error?: string };
    if (!res.ok) {
      return { ok: false, error: data.error || "Activation impossible." };
    }
    return { ok: true };
  }

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);

    const claim = await claimMigratedAccount();
    if (claim.ok) {
      const { error: signInError } = await authClient.signIn.email({
        email: email.trim(),
        password,
        callbackURL: "/dashboard",
      });
      setLoading(false);
      if (signInError) {
        setError(
          "Mot de passe enregistré, mais la connexion a échoué. Réessaie via « Se connecter ».",
        );
        return;
      }
      router.push("/dashboard");
      router.refresh();
      return;
    }

    if (claim.error && /mot de passe existe déjà/i.test(claim.error)) {
      setLoading(false);
      setError(claim.error);
      return;
    }

    if (claim.error && !/Compte inconnu/i.test(claim.error ?? "")) {
      setLoading(false);
      setError(claim.error ?? "Activation impossible.");
      return;
    }

    const { error: signUpError } = await authClient.signUp.email({
      email: email.trim(),
      password,
      name: `${firstName} ${lastName}`.trim() || email.trim(),
      callbackURL: "/dashboard",
    });
    setLoading(false);
    if (signUpError) {
      setError(signUpError.message || "Inscription impossible.");
      return;
    }
    router.push("/dashboard");
    router.refresh();
  }

  return (
    <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
      <form
        onSubmit={onSubmit}
        className="w-full max-w-md space-y-4 rounded-2xl border border-emerald-100 bg-white p-8 shadow-xl shadow-emerald-900/10"
      >
        <div>
          <h1 className="text-xl font-semibold text-emerald-950">Activer mon compte</h1>
          <p className="mt-1 text-sm text-emerald-800/70">
            Compte déjà déjà provisionné : choisis un nouveau mot de passe. Sinon, création
            d’un nouveau compte.
          </p>
        </div>
        {error ? (
          <p className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}
        <div className="grid grid-cols-2 gap-3">
          <label className="block space-y-1 text-sm">
            <span className="font-medium text-emerald-950">Prénom</span>
            <input
              value={firstName}
              onChange={(e) => setFirstName(e.target.value)}
              className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
            />
          </label>
          <label className="block space-y-1 text-sm">
            <span className="font-medium text-emerald-950">Nom</span>
            <input
              value={lastName}
              onChange={(e) => setLastName(e.target.value)}
              className="w-full rounded-xl border border-emerald-100 px-3 py-2 outline-none ring-emerald-200 focus:ring-2"
            />
          </label>
        </div>
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
          <span className="font-medium text-emerald-950">Nouveau mot de passe</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={8}
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
          {loading ? "Activation…" : "Activer et se connecter"}
        </button>
        <p className="text-center text-sm text-emerald-800/70">
          Déjà un mot de passe ?{" "}
          <a href="/auth/sign-in" className="font-medium text-emerald-700 underline-offset-2 hover:underline">
            Se connecter
          </a>
        </p>
      </form>
    </div>
  );
}
