import { Suspense } from "react";
import VsPresenceClient from "@/app/components/vie-scolaire/VsPresenceClient";

export default function VsPresencePage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement…</p>}>
      <VsPresenceClient />
    </Suspense>
  );
}
