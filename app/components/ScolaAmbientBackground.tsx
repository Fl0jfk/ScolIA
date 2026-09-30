import type { ReactNode } from "react";

/** Fond intranet — gris doux + halo lime discret (langage Permitly). */
export default function ScolaAmbientBackground({ children }: { children: ReactNode }) {
  return (
    <div
      className="relative min-h-screen overflow-x-hidden text-[var(--dash-ink)] antialiased selection:bg-[color:var(--dash-lime)]/70 selection:text-[var(--dash-ink)]"
      style={{ backgroundColor: "var(--dash-surface, #F4F5F3)" }}
    >
      <div aria-hidden className="pointer-events-none fixed inset-0 -z-10 overflow-hidden">
        <div
          className="absolute -left-[8%] -top-[12%] h-[48vh] w-[48vh] rounded-full opacity-50 blur-[110px]"
          style={{
            background: "radial-gradient(circle, color-mix(in srgb, var(--dash-lime) 55%, transparent), transparent 70%)",
          }}
        />
        <div
          className="absolute -right-[6%] top-[30%] h-[40vh] w-[40vh] rounded-full opacity-35 blur-[100px]"
          style={{
            background: "radial-gradient(circle, color-mix(in srgb, var(--dash-soft) 80%, transparent), transparent 70%)",
          }}
        />
        <div
          className="absolute bottom-[-5%] left-[20%] h-[36vh] w-[44vh] rounded-full opacity-30 blur-[90px]"
          style={{
            background: "radial-gradient(circle, rgba(20,20,20,0.08), transparent 70%)",
          }}
        />
      </div>
      {children}
    </div>
  );
}
