"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useIsPlatformMaster } from "@/app/hooks/useIsPlatformMaster";

/** Redirige si l'utilisateur n'est pas Master plateforme. */
export default function RequirePlatformMaster({
  children,
  redirectTo = "/dashboard",
}: {
  children: React.ReactNode;
  redirectTo?: string;
}) {
  const router = useRouter();
  const { isLoaded } = useSessionUser();
  const isPlatformMaster = useIsPlatformMaster();

  useEffect(() => {
    if (isLoaded && !isPlatformMaster) {
      router.replace(redirectTo);
    }
  }, [isLoaded, isPlatformMaster, router, redirectTo]);

  if (!isLoaded || !isPlatformMaster) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Chargement…</p>
      </div>
    );
  }

  return <>{children}</>;
}
