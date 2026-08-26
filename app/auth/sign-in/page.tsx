"use client";

import { useEffect, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { authClient, rememberMfaEmailHint } from "@/app/lib/auth-client";

export default function BetterAuthSignInPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const redirectTo = searchParams.get("redirect_url") || "/dashboard";
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [checkingSession, setCheckingSession] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const { data } = await authClient.getSession();
        if (cancelled) return;
        if (data?.session) {
          const u = data.user as { mustChangePassword?: boolean; twoFactorEnabled?: boolean } | undefined;
          // Session encore « provisoire » : ne pas renvoyer vers le dashboard (boucle),
          // laisser le formulaire pour un autre compte après déconnexion côté autre page.
          if (u?.mustChangePassword) {
            router.replace(
              `/auth/change-password-required?redirect_url=${encodeURIComponent(redirectTo)}`,
            );
            return;
          }
          router.replace(redirectTo);
          return;
        }
      } catch {
        /* afficher le formulaire */
      } finally {
        if (!cancelled) setCheckingSession(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [redirectTo, router]);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    rememberMfaEmailHint(email);
    const { data, error: signInError } = await authClient.signIn.email({
      email: email.trim(),
      password,
      callbackURL: redirectTo,
    });
    setLoading(false);
    if (signInError) {
      const msg = signInError.message || "Connexion impossible.";
      if (/verif/i.test(msg) && /email/i.test(msg)) {
        setError("E-mail non vérifié. Consultez votre boîte mail pour le lien d’activation.");
      } else if (/not found|invalid|incorrect|credential/i.test(msg)) {
        setError("Identifiants incorrects. Vérifiez l’e-mail (celui du registre) et le mot de passe.");
      } else {
        setError(msg);
      }
      return;
    }
    if (data && "twoFactorRedirect" in data && data.twoFactorRedirect) {
      const emailQs = email.trim()
        ? `&email=${encodeURIComponent(email.trim().toLowerCase())}`
        : "";
      router.push(
        `/auth/two-factor?redirect_url=${encodeURIComponent(redirectTo)}${emailQs}`,
      );
      return;
    }
    router.push(redirectTo);
    router.refresh();
  }

  if (checkingSession) {
    return (
      <div className="flex min-h-[calc(100vh-3.5rem)] items-center justify-center px-4 py-10">
        <p className="text-sm text-emerald-800/80">Vérification de la session…</p>
      </div>
    );
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
        <p className="text-center text-sm">
          <a
            href="/auth/forgot-password"
            className="font-medium text-[#2F6B4A] underline underline-offset-2"
          >
            Mot de passe oublié ou premier accès
          </a>
        </p>
      </form>
    </div>
  );
}
