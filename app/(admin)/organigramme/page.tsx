"use client";

import { Suspense } from "react";
import OrganigrammePageClient from "./OrganigrammePageClient";

export default function Page() {
  return (
    <Suspense
      fallback={
        <main className="min-h-screen flex items-center justify-center text-slate-500 text-sm">
          Chargement de l&apos;organigramme…
        </main>
      }
    >
      <OrganigrammePageClient />
    </Suspense>
  );
}
