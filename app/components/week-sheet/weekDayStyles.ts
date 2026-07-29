export function weekDayColumnClass(isToday: boolean, extra = ""): string {
  return `flex min-w-0 flex-col rounded-md ${
    isToday
      ? "border-2 border-[var(--dash-primary)] bg-[color:var(--dash-soft-muted)]/55 shadow-sm"
      : "border border-[color:var(--dash-border)]/90 bg-[color:var(--dash-soft-muted)]/25"
  } ${extra}`.trim();
}

export function weekDayLabelClass(isToday: boolean): string {
  return `shrink-0 truncate text-center text-[10px] font-black uppercase tracking-tight sm:text-[9px] ${
    isToday ? "text-[var(--dash-primary)]" : "text-[var(--dash-mid)]"
  }`;
}
