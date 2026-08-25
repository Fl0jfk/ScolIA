"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAppUser } from "@/app/hooks/useAppUser";
import { useIsOrgAdmin } from "@/app/hooks/useIsOrgAdmin";

/** Redirige vers le dashboard si l'utilisateur n'est pas admin intranet. */
export default function RequireOrgAdmin({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const { isLoaded } = useAppUser();
  const isOrgAdmin = useIsOrgAdmin();

  useEffect(() => {
    // Attendre la session complète : ne pas rediriger tant que isLoaded est false.
    if (!isLoaded) return;
    if (!isOrgAdmin) {
      router.replace("/dashboard");
    }
  }, [isLoaded, isOrgAdmin, router]);

  if (!isLoaded) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Chargement…</p>
      </div>
    );
  }

  if (!isOrgAdmin) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Redirection…</p>
      </div>
    );
  }

  return <>{children}</>;
}
