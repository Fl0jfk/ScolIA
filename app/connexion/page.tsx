"use client";

import { useCallback, useEffect, useState } from "react";
import Image from "next/image";
import MarketingShell from "@/app/components/landing/MarketingShell";
import ConnexionPlatformSessionBanner from "@/app/components/ConnexionPlatformSessionBanner";
import { SCOLA_GRADIENT_TEXT } from "@/app/lib/marketing-theme";
import { authClient, rememberMfaEmailHint } from "@/app/lib/auth-client";
import { isBrowserLocalDev } from "@/app/lib/local-dev";
import { platformAdminSignInUrl } from "@/app/lib/platform-portal-url";
import {
  clearLastPortalTenant,
  saveLastPortalTenant,
} from "@/app/lib/tenant-portal-client";

type MembershipDestination = {
  slug: string;
  label: string;
  context: string;
  etablissementId: string;
  dashboardUrl: string;
  kindLabel?: string;
  postalAddressLabel?: string;
  logoUrl?: string | null;
};

type MembershipsPayload = {
  memberships: MembershipDestination[];
  platformAdmin?: boolean;
  needsChoice?: boolean;
  error?: string;
};

function goToMembership(m: MembershipDestination) {
  saveLastPortalTenant({
    slug: m.slug,
    label: m.label,
    signInUrl: m.dashboardUrl.replace(/\/dashboard(?:\?|$)/, "/auth/sign-in$1"),
  });
  window.location.assign(m.dashboardUrl);
}

async function fetchMemberships(): Promise<MembershipsPayload> {
  const res = await fetch("/api/auth/memberships", {
    credentials: "include",
    cache: "no-store",
  });
  const data = (await res.json()) as MembershipsPayload;
  if (!res.ok) {
    throw new Error(data.error || "Impossible de charger vos établissements.");
  }
  return data;
}

export default function ConnexionPage() {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [choices, setChoices] = useState<MembershipDestination[] | null>(null);
  const [isLocalDev, setIsLocalDev] = useState(false);
  const [signedIn, setSignedIn] = useState(false);

  useEffect(() => {
    setIsLocalDev(isBrowserLocalDev());
  }, []);

  const resolveAfterAuth = useCallback(async () => {
    const data = await fetchMemberships();
    setSignedIn(true);

    if (data.platformAdmin && data.memberships.length === 0) {
      window.location.assign("/plateforme");
      return;
    }

    if (data.memberships.length === 0) {
      setError(
        "Aucun établissement n’est rattaché à ce compte. Contactez votre administrateur.",
      );
      setChoices(null);
      return;
    }

    if (data.memberships.length === 1) {
      goToMembership(data.memberships[0]!);
      return;
    }

    // Multi-établissements (rapprochement) : choix uniquement parmi SES rattachements
    setChoices(data.memberships);
    setError(null);
  }, []);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        clearLastPortalTenant();
        const { data } = await authClient.getSession();
        if (cancelled) return;
        if (data?.session) {
          await resolveAfterAuth();
          return;
        }
      } catch {
        /* formulaire */
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [resolveAfterAuth]);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    setChoices(null);
    try {
      rememberMfaEmailHint(email);
      const { data, error: signInError } = await authClient.signIn.email({
        email: email.trim(),
        password,
      });
      if (signInError) {
        const msg = signInError.message || "Connexion impossible.";
        if (/verif/i.test(msg) && /email/i.test(msg)) {
          setError("E-mail non vérifié. Consultez votre boîte mail pour le lien d’activation.");
        } else if (/not found|invalid|incorrect|credential/i.test(msg)) {
          setError("Identifiants incorrects.");
        } else {
          setError(msg);
        }
        return;
      }
      if (data && "twoFactorRedirect" in data && data.twoFactorRedirect) {
        rememberMfaEmailHint(email);
        const emailQs = email.trim()
          ? `&email=${encodeURIComponent(email.trim().toLowerCase())}`
          : "";
        window.location.assign(
          `/auth/two-factor?redirect_url=${encodeURIComponent("/connexion")}${emailQs}`,
        );
        return;
      }
      await resolveAfterAuth();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Erreur de connexion.");
    } finally {
      setSubmitting(false);
    }
  }

  const adminSignInHref = isLocalDev
    ? `/auth/sign-in?redirect_url=${encodeURIComponent("/plateforme")}`
    : platformAdminSignInUrl();

  if (loading) {
    return (
      <MarketingShell>
        <main className="mx-auto max-w-lg px-6 py-24 text-center">
          <p className="text-sm font-medium text-stone-600">Vérification de la session…</p>
        </main>
      </MarketingShell>
    );
  }

  return (
    <MarketingShell>
      <main className="mx-auto max-w-md px-6 py-12">
        <div className="text-center">
          <h1 className="text-3xl font-black text-[#14231A]">
            Connexion <span className={SCOLA_GRADIENT_TEXT}>ScolIA</span>
          </h1>
          <p className="mx-auto mt-3 max-w-sm text-sm text-stone-600">
            {choices
              ? "Plusieurs établissements sont rattachés à votre compte — choisissez où entrer."
              : "E-mail et mot de passe. Votre établissement est déterminé automatiquement."}
          </p>
        </div>

        <ConnexionPlatformSessionBanner />

        {error ? (
          <p className="mt-8 rounded-lg bg-red-50 px-3 py-2 text-center text-sm text-red-700" role="alert">
            {error}
          </p>
        ) : null}

        {choices && choices.length > 1 ? (
          <ul className="mt-10 grid gap-4">
            {choices.map((m) => (
              <li key={m.etablissementId}>
                <button
                  type="button"
                  onClick={() => goToMembership(m)}
                  className="group flex w-full flex-col items-center gap-3 rounded-2xl border border-stone-200/90 bg-white px-5 py-6 text-center shadow-sm transition hover:-translate-y-0.5 hover:border-[#2F6B4A]/40 hover:shadow-md"
                >
                  {m.logoUrl ? (
                    <div className="flex h-20 w-20 items-center justify-center rounded-2xl bg-stone-50 p-2 shadow-inner ring-1 ring-stone-100">
                      <Image
                        src={m.logoUrl}
                        alt=""
                        width={72}
                        height={72}
                        className="h-full w-full object-contain"
                        unoptimized
                      />
                    </div>
                  ) : (
                    <div className="flex h-20 w-20 items-center justify-center rounded-2xl bg-gradient-to-br from-emerald-50 to-emerald-100 text-2xl font-black text-[#2F6B4A]">
                      {m.label.charAt(0)}
                    </div>
                  )}
                  <div>
                    <p className="text-base font-bold text-[#14231A]">{m.label}</p>
                    {m.kindLabel ? (
                      <p className="mt-1 text-xs font-semibold uppercase tracking-wide text-[#2F6B4A]/80">
                        {m.kindLabel}
                      </p>
                    ) : null}
                    {m.postalAddressLabel ? (
                      <p className="mt-2 text-sm text-stone-600">{m.postalAddressLabel}</p>
                    ) : null}
                  </div>
                  <span className="mt-1 rounded-full bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-1.5 text-xs font-bold text-white">
                    Ouvrir l’intranet
                  </span>
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <form
            onSubmit={onSubmit}
            className="mt-10 space-y-4 rounded-2xl border border-stone-200/90 bg-white p-6 shadow-sm"
          >
            <label className="block space-y-1 text-sm">
              <span className="font-medium text-[#14231A]">E-mail</span>
              <input
                type="email"
                autoComplete="email"
                required
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                className="w-full rounded-xl border border-stone-200 px-3 py-2 outline-none ring-[#2F6B4A]/30 focus:ring-2"
              />
            </label>
            <label className="block space-y-1 text-sm">
              <span className="font-medium text-[#14231A]">Mot de passe</span>
              <input
                type="password"
                autoComplete="current-password"
                required
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full rounded-xl border border-stone-200 px-3 py-2 outline-none ring-[#2F6B4A]/30 focus:ring-2"
              />
            </label>
            <button
              type="submit"
              disabled={submitting}
              className="w-full rounded-xl bg-gradient-to-r from-[#2F6B4A] to-[#1E4A32] px-4 py-2.5 text-sm font-bold text-white disabled:opacity-50"
            >
              {submitting ? "Connexion…" : "Se connecter"}
            </button>
            {signedIn ? (
              <p className="text-center text-xs text-stone-500">
                Session active — résolution de votre établissement…
              </p>
            ) : null}
          </form>
        )}

        <p className="mt-12 text-center text-xs text-stone-500">
          Vous gérez la plateforme Scola ?{" "}
          <a href={adminSignInHref} className="font-semibold text-violet-700 hover:underline">
            Administration
          </a>
        </p>
      </main>
    </MarketingShell>
  );
}
