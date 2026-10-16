/** Pastille type Apple pour compteurs de notifications. */

export const NOTIFICATION_COUNT_BADGE_UNAVAILABLE_CLASS =
  "bg-amber-100 text-amber-900 ring-1 ring-amber-300";

export const NOTIFICATION_COUNT_BADGE_COUNT_CLASS =
  "bg-[#FF3B30] tabular-nums text-white";

/** Texte visible court dans la pastille (tous emplacements). */
export const DASHBOARD_COUNTER_UNAVAILABLE_MARK = "!";

export const DASHBOARD_COUNTER_UNAVAILABLE_TITLE = "Compteur indisponible";

export type NotificationCountBadgeProps = {
  count?: number;
  /** Compteur non disponible (source timeout / error). */
  unavailable?: boolean;
  /** Pastille compacte « ! » (sidebar). */
  compact?: boolean;
  /** Compteur connu mais d’autres signaux du groupe sont indisponibles. */
  partialUnavailable?: boolean;
  className?: string;
  /** Masque le badge aux lecteurs d’écran si le nom est porté par le parent (lien). */
  decorative?: boolean;
};

export default function NotificationCountBadge({
  count,
  unavailable = false,
  compact = false,
  partialUnavailable = false,
  className = "",
  decorative = false,
}: NotificationCountBadgeProps) {
  const display = unavailable
    ? DASHBOARD_COUNTER_UNAVAILABLE_MARK
    : count !== undefined && count > 0
      ? count > 99
        ? "99+"
        : String(count)
      : null;
  if (!display) return null;

  const countLabel =
    count !== undefined && count > 0
      ? `${count} notification${count > 1 ? "s" : ""}`
      : undefined;

  return (
    <span
      role={decorative ? undefined : "img"}
      aria-hidden={decorative ? true : undefined}
      aria-label={decorative ? undefined : unavailable ? DASHBOARD_COUNTER_UNAVAILABLE_TITLE : countLabel}
      title={unavailable ? DASHBOARD_COUNTER_UNAVAILABLE_TITLE : undefined}
      className={`inline-flex h-[1.125rem] min-w-[1.125rem] shrink-0 items-center justify-center rounded-full px-[5px] text-[11px] font-semibold leading-none tracking-tight shadow-[0_1px_2px_rgba(0,0,0,0.18)] ${
        unavailable
          ? `${NOTIFICATION_COUNT_BADGE_UNAVAILABLE_CLASS} normal-case`
          : `${NOTIFICATION_COUNT_BADGE_COUNT_CLASS}${
              partialUnavailable ? " ring-2 ring-amber-300 ring-offset-1" : ""
            }`
      } ${className}`}
    >
      {display}
      {!decorative && unavailable ? (
        <span className="sr-only">compteur indisponible</span>
      ) : !decorative && countLabel ? (
        <span className="sr-only">{countLabel}</span>
      ) : null}
    </span>
  );
}
