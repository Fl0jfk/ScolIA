import { Suspense } from "react";
import ServicesHubClient from "@/app/(admin)/services/ServicesHubClient";

export default function ServicesHubPage() {
  return (
    <Suspense fallback={<p className="p-10 text-center text-slate-500">Chargement du module Services…</p>}>
      <ServicesHubClient />
    </Suspense>
  );
}
