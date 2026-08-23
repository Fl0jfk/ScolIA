"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { useSignOutWithPortalReset } from "@/app/hooks/useSignOutWithPortalReset";
import { platformAdminSignInUrl } from "@/app/lib/platform-portal-url";

/** Alerte si une session administration Scola est encore active sur le portail connexion. */
export default function ConnexionPlatformSessionBanner() {
  const { isLoaded, isSignedIn } = useSessionUser();
  const signOut = useSignOutWithPortalReset();

  if (!isLoaded || !isSignedIn) return null;

  return (
    <div className="mt-8 rounded-2xl border border-violet-200 bg-violet-50 px-5 py-4 text-sm text-violet-950">
      <p className="font-bold">Session administration Scola active sur docslapro.com</p>
      <p className="mt-1 text-violet-900/90">
        Pour accéder à l&apos;intranet de votre établissement, choisissez-le dans la liste
        ci-dessous — c&apos;est une connexion séparée.
      </p>
      <div className="mt-3 flex flex-wrap gap-3">
        <button
          type="button"
          onClick={() => signOut(window.location.href)}
          className="rounded-full bg-white px-4 py-2 text-xs font-bold text-violet-800 ring-1 ring-violet-200 hover:bg-violet-100"
        >
          Se déconnecter de l&apos;administration
        </button>
        <a
          href={platformAdminSignInUrl()}
          className="rounded-full bg-violet-700 px-4 py-2 text-xs font-bold text-white hover:bg-violet-800"
        >
          Retour administration
        </a>
      </div>
    </div>
  );
}
