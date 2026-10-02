import { Suspense } from "react";
import MessageriePageClient from "@/app/components/messaging/MessageriePageClient";
import DashboardThemeRoot from "@/app/components/Dashboard/DashboardThemeRoot";

export default function MessageriePage() {
  return (
    <Suspense
      fallback={
        <DashboardThemeRoot>
          <p className="p-8 text-sm text-neutral-500">Chargement…</p>
        </DashboardThemeRoot>
      }
    >
      <MessageriePageClient />
    </Suspense>
  );
}
