import { Suspense } from "react";
import FoyersListClient from "./FoyersListClient";

export default function FoyersPage() {
  return (
    <Suspense fallback={<div className="p-8 text-sm text-slate-500">Chargement des foyers…</div>}>
      <FoyersListClient />
    </Suspense>
  );
}
