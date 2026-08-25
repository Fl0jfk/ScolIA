"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAppUser } from "@/app/hooks/useAppUser";
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
  const { isLoaded } = useAppUser();
  const isPlatformMaster = useIsPlatformMaster();

  useEffect(() => {
    if (!isLoaded) return;
    if (!isPlatformMaster) {
      router.replace(redirectTo);
    }
  }, [isLoaded, isPlatformMaster, router, redirectTo]);

  if (!isLoaded) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Chargement…</p>
      </div>
    );
  }

  if (!isPlatformMaster) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Redirection…</p>
      </div>
    );
  }

  return <>{children}</>;
}
