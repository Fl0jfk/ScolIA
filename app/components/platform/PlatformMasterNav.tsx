"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useSignOutWithPortalReset } from "@/app/hooks/useSignOutWithPortalReset";
import { useIsPlatformMaster } from "@/app/hooks/useIsPlatformMaster";
import { platformConnexionUrl } from "@/app/lib/platform-portal-url";
import { resolveEstablishmentPortalOrigin } from "@/app/lib/tenant-portal-client";

const connectClassName =
  "rounded-full bg-gradient-to-r from-[#2F6B4A] via-[#25633F] to-[#1E4A32] px-4 py-2 text-sm font-bold text-white shadow-lg shadow-emerald-900/30 transition hover:scale-[1.02] hover:brightness-110";

/** Bouton connexion / espace Master dans le header marketing. */
export default function PlatformMasterNav() {
  const { isLoaded, isSignedIn } = useSessionUser();
  const isMaster = useIsPlatformMaster();
  const signOut = useSignOutWithPortalReset();
  const [intranetHref, setIntranetHref] = useState(platformConnexionUrl());

  useEffect(() => {
    if (!isSignedIn) return;
    let cancelled = false;
    void resolveEstablishmentPortalOrigin().then((origin) => {
      if (cancelled || !origin) return;
      setIntranetHref(`${origin}/dashboard`);
    });
    return () => {
      cancelled = true;
    };
  }, [isSignedIn]);

  if (!isLoaded || !isSignedIn) {
    return (
      <Link href="/connexion" className={connectClassName}>
        Se connecter
      </Link>
    );
  }

  return (
    <div className="flex items-center gap-2">
      {isMaster && (
        <Link
          href="/plateforme"
          className="hidden rounded-full border-2 border-violet-300 bg-violet-50 px-4 py-2 text-sm font-bold text-violet-800 sm:inline-flex"
        >
          Espace plateforme
        </Link>
      )}
      <a href={intranetHref} className={connectClassName}>
        Mon intranet
      </a>
      <button
        type="button"
        onClick={() => signOut("/")}
        className="rounded-full border border-slate-200 bg-white px-3 py-2 text-sm font-semibold text-slate-700 hover:bg-slate-50"
      >
        Déconnexion
      </button>
    </div>
  );
}
