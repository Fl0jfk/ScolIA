import { Suspense } from "react";
import GestionInternatClient from "./GestionInternatClient";

export default function GestionInternatPage() {
  return (
    <Suspense
      fallback={
        <div className="max-w-[1400px] mx-auto px-4 py-10">
          <p className="text-slate-500 text-sm">Chargement du module internat…</p>
        </div>
      }
    >
      <GestionInternatClient />
    </Suspense>
  );
}
