/** Pastille rouge type Apple pour compteurs de notifications. */
export default function NotificationCountBadge({
  count,
  className = "",
}: {
  count: number;
  className?: string;
}) {
  if (count <= 0) return null;
  const label = count > 99 ? "99+" : String(count);
  return (
    <span
      className={`inline-flex h-[1.125rem] min-w-[1.125rem] shrink-0 items-center justify-center rounded-full bg-[#FF3B30] px-[5px] text-[11px] font-semibold tabular-nums leading-none tracking-tight text-white shadow-[0_1px_2px_rgba(0,0,0,0.18)] ${className}`}
      aria-label={`${count} notification${count > 1 ? "s" : ""}`}
    >
      {label}
    </span>
  );
}
