"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

const LINKS = [
  { href: "/famille", label: "Accueil" },
  { href: "/famille/edt", label: "EDT" },
  { href: "/famille/notes", label: "Notes" },
  { href: "/famille/bulletins", label: "Bulletins" },
  { href: "/famille/absences", label: "Absences" },
  { href: "/famille/carnet", label: "Carnet" },
  { href: "/famille/messages", label: "Messages" },
  { href: "/famille/finances", label: "Finances" },
] as const;

type Props = {
  enfantId?: string | null;
  /** Badge notifs messages non lus (in-app). */
  unreadMessages?: number;
};

function hrefWithEnfant(base: string, enfantId: string | null | undefined): string {
  if (!enfantId) return base;
  return `${base}?enfant=${encodeURIComponent(enfantId)}`;
}

export default function FamilleNav({ enfantId, unreadMessages = 0 }: Props) {
  const pathname = usePathname();
  return (
    <nav className="flex flex-wrap gap-2 mt-3 items-center">
      {LINKS.map((l) => {
        const href = hrefWithEnfant(l.href, enfantId);
        const active = pathname === l.href || (l.href !== "/famille" && pathname.startsWith(l.href));
        const showBadge = l.href === "/famille/messages" && unreadMessages > 0;
        return (
          <Link
            key={l.href}
            href={href}
            className={`rounded-lg px-3 py-1.5 text-xs font-bold ${
              active
                ? "bg-indigo-600 text-white"
                : "bg-white text-slate-700 border border-slate-200 hover:bg-slate-50"
            }`}
          >
            {l.label}
            {showBadge ? (
              <span
                className={`ml-1 inline-flex min-w-[1.25rem] justify-center rounded-full px-1 text-[10px] ${
                  active ? "bg-white/25 text-white" : "bg-rose-600 text-white"
                }`}
              >
                {unreadMessages > 9 ? "9+" : unreadMessages}
              </span>
            ) : null}
          </Link>
        );
      })}
      <Link
        href="/sign-out?dev_tenant=default"
        className="rounded-lg px-3 py-1.5 text-xs font-bold text-slate-500 border border-slate-200 bg-white hover:bg-slate-50 ml-auto"
      >
        Se déconnecter
      </Link>
    </nav>
  );
}
