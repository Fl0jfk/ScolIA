import { Suspense } from "react";
import InstallationClient from "./InstallationClient";

export default function InternatInstallationPage() {
  return (
    <main className="min-h-[100dvh] bg-[radial-gradient(ellipse_at_top,_#eef2ff_0%,_#f8fafc_50%,_#f1f5f9_100%)]">
      <Suspense
        fallback={
          <div className="mx-auto max-w-lg px-4 py-16 text-center text-sm text-slate-500">
            Chargement…
          </div>
        }
      >
        <InstallationClient />
      </Suspense>
    </main>
  );
}
