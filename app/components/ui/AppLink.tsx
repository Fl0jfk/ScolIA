"use client";

import NextLink from "next/link";
import { useRouter } from "next/navigation";
import { forwardRef, useCallback, useRef, type ComponentProps } from "react";

type AppLinkProps = ComponentProps<typeof NextLink>;

/**
 * Lien interne de l'intranet.
 *
 * Par défaut, `next/link` précharge toutes les pages dont un lien est visible à l'écran :
 * une vingtaine de requêtes serveur par page (menu, tuiles, onglets), chacune passant par
 * le proxy d'authentification. Ici, on ne précharge qu'au survol, au focus clavier ou au
 * toucher — juste avant le clic probable. Passer `prefetch` explicitement garde le
 * comportement Next d'origine.
 */
const AppLink = forwardRef<HTMLAnchorElement, AppLinkProps>(function AppLink(
  { prefetch, onMouseEnter, onFocus, onTouchStart, href, ...rest },
  ref,
) {
  const router = useRouter();
  const done = useRef(false);

  const warm = useCallback(() => {
    if (done.current) return;
    if (typeof href !== "string" || !href.startsWith("/") || href.startsWith("//")) return;
    done.current = true;
    try {
      router.prefetch(href);
    } catch {
      /* préchargement facultatif */
    }
  }, [href, router]);

  if (prefetch !== undefined) {
    return (
      <NextLink
        ref={ref}
        href={href}
        prefetch={prefetch}
        onMouseEnter={onMouseEnter}
        onFocus={onFocus}
        onTouchStart={onTouchStart}
        {...rest}
      />
    );
  }

  return (
    <NextLink
      ref={ref}
      href={href}
      prefetch={false}
      onMouseEnter={(e) => {
        warm();
        onMouseEnter?.(e);
      }}
      onFocus={(e) => {
        warm();
        onFocus?.(e);
      }}
      onTouchStart={(e) => {
        warm();
        onTouchStart?.(e);
      }}
      {...rest}
    />
  );
});

export default AppLink;
