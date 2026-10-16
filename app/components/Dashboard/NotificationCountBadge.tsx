/** Pastille type Apple pour compteurs de notifications. */
export default function NotificationCountBadge({
  count,
  label,
  className = "",
}: {
  count?: number;
  /** Si fourni, remplace le compteur (ex. « indisponible »). */
  label?: string;
  className?: string;
}) {
  const display =
    label ??
    (count !== undefined && count > 0 ? (count > 99 ? "99+" : String(count)) : null);
  if (!display) return null;
  const unavailable = display === "indisponible";
  return (
    <span
      className={`inline-flex h-[1.125rem] min-w-[1.125rem] shrink-0 items-center justify-center rounded-full px-[5px] text-[11px] font-semibold leading-none tracking-tight shadow-[0_1px_2px_rgba(0,0,0,0.18)] ${
        unavailable
          ? "bg-amber-600 text-white normal-case"
          : "bg-[#FF3B30] tabular-nums text-white"
      } ${className}`}
      aria-label={
        unavailable
          ? "Compteur indisponible"
          : `${count ?? 0} notification${(count ?? 0) > 1 ? "s" : ""}`
      }
    >
      {display}
    </span>
  );
}
