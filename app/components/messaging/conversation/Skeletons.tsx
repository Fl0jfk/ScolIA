/** Squelettes de chargement (messages et liste). Statiques si l'utilisateur réduit les animations. */

const pulse = "motion-safe:animate-pulse";

export function MessagesSkeleton() {
  const rows: Array<{ mine: boolean; w: string; h: string }> = [
    { mine: false, w: "w-48", h: "h-9" },
    { mine: false, w: "w-64", h: "h-14" },
    { mine: true, w: "w-40", h: "h-9" },
    { mine: false, w: "w-56", h: "h-9" },
    { mine: true, w: "w-60", h: "h-12" },
  ];
  return (
    <div role="status" aria-label="Chargement des messages" className="space-y-3 pt-2">
      {rows.map((r, i) => (
        <div key={i} className={`flex ${r.mine ? "justify-end" : "justify-start"}`}>
          <div
            className={`${r.w} ${r.h} max-w-[75%] rounded-[1.25rem] ${pulse} ${
              r.mine ? "rounded-br-md bg-black/10" : "rounded-bl-md bg-black/[0.06]"
            }`}
          />
        </div>
      ))}
    </div>
  );
}

export function ListSkeleton() {
  return (
    <div role="status" aria-label="Chargement des conversations" className="space-y-1 px-1 py-1">
      {[0, 1, 2, 3, 4, 5].map((i) => (
        <div key={i} className="flex items-center gap-3 rounded-2xl px-2.5 py-2.5">
          <div className={`h-11 w-11 shrink-0 rounded-full bg-black/[0.08] ${pulse}`} />
          <div className="min-w-0 flex-1 space-y-2">
            <div className={`h-3 rounded bg-black/[0.08] ${pulse}`} style={{ width: `${55 + ((i * 13) % 30)}%` }} />
            <div className={`h-2.5 rounded bg-black/[0.05] ${pulse}`} style={{ width: `${70 - ((i * 9) % 25)}%` }} />
          </div>
        </div>
      ))}
    </div>
  );
}
