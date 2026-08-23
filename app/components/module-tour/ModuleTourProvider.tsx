"use client";

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { usePathname } from "next/navigation";
import { useSessionUser } from "@/app/hooks/useAppUser";
import { useAdminBootstrap } from "@/app/contexts/admin-bootstrap";
import { intranetRolesFromMetadata } from "@/app/lib/intranet-roles";
import { resolveModuleIdFromPath } from "@/app/lib/intranet-modules";
import {
  getModuleTour,
  tourVisibleForRoles,
  type ModuleTourDefinition,
} from "@/app/lib/module-tours";
import {
  markModuleTourCompleted,
  readCompletedModuleTours,
  resetModuleTour,
} from "@/app/lib/module-tour-storage";
import ModuleTourOverlay from "@/app/components/module-tour/ModuleTourOverlay";

type ModuleTourContextValue = {
  requestReplay: (moduleId: string) => void;
};

const ModuleTourContext = createContext<ModuleTourContextValue | null>(null);

export function useModuleTour() {
  const ctx = useContext(ModuleTourContext);
  if (!ctx) {
    return { requestReplay: () => {} };
  }
  return ctx;
}

export default function ModuleTourProvider({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const { user, isLoaded } = useSessionUser();
  const bootstrap = useAdminBootstrap();
  const userId = user?.id ?? "";

  const roles = useMemo(
    () => bootstrap.appContext?.session?.intranetRoles ?? intranetRolesFromMetadata(user?.publicMetadata),
    [bootstrap.appContext?.session?.intranetRoles, user?.publicMetadata],
  );

  const [activeTour, setActiveTour] = useState<ModuleTourDefinition | null>(null);
  const [stepIndex, setStepIndex] = useState(0);
  const [forcedModuleId, setForcedModuleId] = useState<string | null>(null);

  const closeTour = useCallback(
    (completed: boolean) => {
      if (completed && activeTour && userId) {
        markModuleTourCompleted(userId, activeTour.moduleId);
      }
      setActiveTour(null);
      setStepIndex(0);
      setForcedModuleId(null);
    },
    [activeTour, userId],
  );

  const requestReplay = useCallback(
    (moduleId: string) => {
      if (!userId) return;
      resetModuleTour(userId, moduleId);
      const tour = getModuleTour(moduleId);
      if (!tour) return;
      if (!tourVisibleForRoles(tour, roles)) return;
      setForcedModuleId(moduleId);
      setActiveTour(tour);
      setStepIndex(0);
    },
    [userId, roles],
  );

  useEffect(() => {
    if (!isLoaded || !userId || activeTour) return;
    if (forcedModuleId) return;

    const moduleId = resolveModuleIdFromPath(pathname);
    if (!moduleId) return;

    const tour = getModuleTour(moduleId);
    if (!tour) return;
    if (!tourVisibleForRoles(tour, roles)) return;

    const completed = readCompletedModuleTours(userId);
    if (completed.has(moduleId)) return;

    const timer = window.setTimeout(() => {
      setActiveTour(tour);
      setStepIndex(0);
    }, 600);

    return () => window.clearTimeout(timer);
  }, [pathname, userId, isLoaded, roles, activeTour, forcedModuleId]);

  useEffect(() => {
    if (!forcedModuleId) return;
    const tour = getModuleTour(forcedModuleId);
    if (tour) {
      setActiveTour(tour);
      setStepIndex(0);
    }
  }, [forcedModuleId]);

  const ctx = useMemo(() => ({ requestReplay }), [requestReplay]);

  return (
    <ModuleTourContext.Provider value={ctx}>
      {children}
      {activeTour && (
        <ModuleTourOverlay
          tour={activeTour}
          stepIndex={stepIndex}
          onNext={() => setStepIndex((i) => Math.min(i + 1, activeTour.steps.length - 1))}
          onPrev={() => setStepIndex((i) => Math.max(i - 1, 0))}
          onSkip={() => closeTour(true)}
          onFinish={() => closeTour(true)}
        />
      )}
    </ModuleTourContext.Provider>
  );
}
