"use client";

import { useEffect, useState } from "react";
import { usePathname, useSearchParams } from "next/navigation";

/**
 * Feedback immédiat au clic interne : barre + voile léger.
 * Se coupe dès que la route a changé (soft nav Next).
 */
export default function NavigationProgress() {
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const [pending, setPending] = useState(false);
  const [label, setLabel] = useState("Chargement…");

  useEffect(() => {
    setPending(false);
  }, [pathname, searchParams]);

  useEffect(() => {
    const onClick = (event: MouseEvent) => {
      if (event.defaultPrevented) return;
      if (event.button !== 0) return;
      if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;

      const target = event.target;
      if (!(target instanceof Element)) return;
      const anchor = target.closest("a[href]");
      if (!(anchor instanceof HTMLAnchorElement)) return;
      if (anchor.hasAttribute("download")) return;
      if (anchor.target && anchor.target !== "_self") return;

      const href = anchor.getAttribute("href");
      if (!href || href.startsWith("#") || href.startsWith("mailto:") || href.startsWith("tel:")) {
        return;
      }

      let url: URL;
      try {
        url = new URL(href, window.location.href);
      } catch {
        return;
      }
      if (url.origin !== window.location.origin) return;
      if (
        url.pathname === window.location.pathname &&
        url.search === window.location.search &&
        url.hash === window.location.hash
      ) {
        return;
      }

      const name =
        anchor.getAttribute("aria-label")?.trim() ||
        anchor.textContent?.replace(/\s+/g, " ").trim() ||
        "";
      setLabel(name ? `Ouverture · ${name.slice(0, 48)}` : "Chargement…");
      setPending(true);
    };

    document.addEventListener("click", onClick, true);
    return () => document.removeEventListener("click", onClick, true);
  }, []);

  // Filet : si la navigation échoue / reste sur place, ne pas bloquer l’UI.
  useEffect(() => {
    if (!pending) return;
    const t = window.setTimeout(() => setPending(false), 12_000);
    return () => window.clearTimeout(t);
  }, [pending]);

  if (!pending) return null;

  return (
    <div className="pointer-events-none fixed inset-0 z-[200]" aria-live="polite" aria-busy="true">
      <div className="absolute inset-x-0 top-0 h-1 overflow-hidden bg-slate-200/40">
        <div className="nav-progress-bar h-full w-1/3 rounded-r bg-indigo-500 shadow-[0_0_12px_rgba(99,102,241,0.65)]" />
      </div>
      <div className="absolute inset-0 bg-slate-900/[0.06] backdrop-blur-[1px]" />
      <div className="absolute left-1/2 top-[42%] flex -translate-x-1/2 -translate-y-1/2 flex-col items-center gap-3 rounded-2xl border border-white/70 bg-white/90 px-5 py-4 shadow-[0_20px_50px_-28px_rgba(15,23,42,0.55)]">
        <span
          className="inline-block h-8 w-8 animate-spin rounded-full border-2 border-indigo-200 border-t-indigo-600"
          aria-hidden
        />
        <p className="max-w-[16rem] truncate text-center text-sm font-semibold text-slate-700">
          {label}
        </p>
      </div>
    </div>
  );
}
