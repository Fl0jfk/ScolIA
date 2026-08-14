import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "ScolIA",
  description: "Assistant IA de votre établissement",
};

export default function ScoliaLayout({ children }: { children: React.ReactNode }) {
  return <div className="min-h-[100dvh] bg-slate-50">{children}</div>;
}
