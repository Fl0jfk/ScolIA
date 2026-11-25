import { Suspense } from "react";
import FoyersImportClient from "./FoyersImportClient";

export default function FoyersImportPage() {
  return (
    <Suspense fallback={<div className="p-8 text-sm text-slate-500">Chargement…</div>}>
      <FoyersImportClient />
    </Suspense>
  );
}
