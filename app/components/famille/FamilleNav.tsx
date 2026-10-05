"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  QUOTIDIEN_BASE,
  QUOTIDIEN_NAV_LINKS,
  quotidienHref,
} from "@/app/lib/quotidien-portal";

type Props = {
  enfantId?: string | null;
  /** Badge notifs messages non lus (in-app). */
  unreadMessages?: number;
};

export default function FamilleNav({ enfantId, unreadMessages = 0 }: Props) {
  const pathname = usePathname();
  return (
    <nav className="flex flex-wrap gap-2 mt-3 items-center">
      {QUOTIDIEN_NAV_LINKS.map((l) => {
        const href = quotidienHref(l.path, enfantId);
        const fullPath = `${QUOTIDIEN_BASE}${l.path}` || QUOTIDIEN_BASE;
        const active =
          pathname === fullPath ||
          (l.path !== "" && pathname.startsWith(fullPath));
        const showBadge = l.path === "/messages" && unreadMessages > 0;
        return (
          <Link
            key={l.path || "home"}
            href={href}
            className={`rounded-lg px-3 py-1.5 text-xs font-bold ${
              active
                ? "bg-teal-700 text-white"
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
