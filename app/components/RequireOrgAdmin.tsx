"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";

/** Redirige vers le dashboard si l'utilisateur n'est pas admin intranet. */
export default function RequireOrgAdmin({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const { isLoaded } = useSessionUser();
  const isOrgAdmin = useIsOrgAdmin();

  useEffect(() => {
    if (isLoaded && !isOrgAdmin) {
      router.replace("/dashboard");
    }
  }, [isLoaded, isOrgAdmin, router]);

  if (!isLoaded || !isOrgAdmin) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Chargement…</p>
      </div>
    );
  }

  return <>{children}</>;
}
