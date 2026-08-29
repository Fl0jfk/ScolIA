import { Suspense } from "react";
import StagePublicSignerClient from "@/app/components/stages/StagePublicSignerClient";

export default function StageSignerPage() {
  return (
    <Suspense fallback={<main className="p-8">Chargement…</main>}>
      <StagePublicSignerClient />
    </Suspense>
  );
}
