"use client";

import { useSessionUser } from "@/app/hooks/useAppUser";
import { useSignOutWithPortalReset } from "@/app/hooks/useSignOutWithPortalReset";
import { useIsPlatformMaster } from "@/app/hooks/useIsPlatformMaster";
import { platformAppOrigin } from "@/app/lib/platform-portal-url";

/**
 * Alerte uniquement pour une session Master plateforme sur le portail connexion.
 * Les comptes établissement partagent déjà le cookie — pas besoin de se reconnecter.
 */
export default function ConnexionPlatformSessionBanner() {
  const { isLoaded, isSignedIn } = useSessionUser();
  const isMaster = useIsPlatformMaster();
  const signOut = useSignOutWithPortalReset();

  if (!isLoaded || !isSignedIn || !isMaster) return null;

  return (
    <div className="mt-8 rounded-2xl border border-violet-200 bg-violet-50 px-5 py-4 text-sm text-violet-950">
      <p className="font-bold">Session administration Scola active</p>
      <p className="mt-1 text-violet-900/90">
        Vous êtes connecté en Master. Choisissez un établissement ci-dessous pour ouvrir son
        intranet, ou retournez à la console plateforme.
      </p>
      <div className="mt-3 flex flex-wrap gap-3">
        <button
          type="button"
          onClick={() => signOut(window.location.href)}
          className="rounded-full bg-white px-4 py-2 text-xs font-bold text-violet-800 ring-1 ring-violet-200 hover:bg-violet-100"
        >
          Se déconnecter
        </button>
        <a
          href={`${platformAppOrigin()}/plateforme`}
          className="rounded-full bg-violet-700 px-4 py-2 text-xs font-bold text-white hover:bg-violet-800"
        >
          Retour administration
        </a>
      </div>
    </div>
  );
}
