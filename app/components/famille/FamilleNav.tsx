"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const LINKS = [
  { href: "/famille", label: "Accueil" },
  { href: "/famille/bulletins", label: "Bulletins" },
  { href: "/famille/absences", label: "Absences" },
  { href: "/famille/carnet", label: "Carnet" },
  { href: "/famille/finances", label: "Finances" },
] as const;

export default function FamilleNav() {
  const pathname = usePathname();
  return (
    <nav className="flex flex-wrap gap-2 mt-3">
      {LINKS.map((l) => {
        const active = pathname === l.href || (l.href !== "/famille" && pathname.startsWith(l.href));
        return (
          <Link
            key={l.href}
            href={l.href}
            className={`rounded-lg px-3 py-1.5 text-xs font-bold ${
              active
                ? "bg-indigo-600 text-white"
                : "bg-white text-slate-700 border border-slate-200 hover:bg-slate-50"
            }`}
          >
            {l.label}
          </Link>
        );
      })}
    </nav>
  );
}
