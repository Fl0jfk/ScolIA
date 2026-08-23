import { Suspense } from "react";

export default function AuthLayout({ children }: { children: React.ReactNode }) {
  return <Suspense fallback={<div className="p-10 text-center text-sm text-emerald-800">Chargement…</div>}>{children}</Suspense>;
}
