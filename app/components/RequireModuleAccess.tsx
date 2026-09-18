"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useCanAccessModule } from "@/app/hooks/useCanAccessModule";

/** Redirige vers le dashboard si l’utilisateur n’a pas accès au module. */
export default function RequireModuleAccess({
  moduleId,
  children,
}: {
  moduleId: string;
  children: React.ReactNode;
}) {
  const router = useRouter();
  const { isLoaded, canAccess } = useCanAccessModule(moduleId);

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
