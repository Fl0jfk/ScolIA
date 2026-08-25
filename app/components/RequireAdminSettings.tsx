"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useAppUser } from "@/app/hooks/useAppUser";
import { useCanAccessAdminSettings } from "@/app/hooks/useCanAccessAdminSettings";

/** Paramètres : org-admin ou direction (école / collège / lycée). */
export default function RequireAdminSettings({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const { isLoaded } = useAppUser();
  const canAccess = useCanAccessAdminSettings();

  useEffect(() => {
    if (!isLoaded) return;
    if (!canAccess) {
      router.replace("/dashboard");
    }
  }, [isLoaded, canAccess, router]);

  if (!isLoaded) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Chargement…</p>
      </div>
    );
  }

  if (!canAccess) {
    return (
      <div className="flex items-center justify-center min-h-[40vh]">
        <p className="text-sm text-slate-400">Redirection…</p>
      </div>
    );
  }

  return <>{children}</>;
}
