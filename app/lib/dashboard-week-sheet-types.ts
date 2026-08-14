import type { DashboardCategory } from "@/app/lib/intranet-modules";

const DASHBOARD_WEEK_SHEET_MODULE_ID = "dashboard-week-sheet";

export const WEEK_SHEET_STORAGE_PATH = "dashboard/week-sheet/current.json";

export type WeekDayKey = "mon" | "tue" | "wed" | "thu" | "fri";

export const WEEK_DAYS: { key: WeekDayKey; label: string; short: string }[] = [
  { key: "mon", label: "Lundi", short: "Lun" },
  { key: "tue", label: "Mardi", short: "Mar" },
  { key: "wed", label: "Mercredi", short: "Mer" },
  { key: "thu", label: "Jeudi", short: "Jeu" },
  { key: "fri", label: "Vendredi", short: "Ven" },
];

export type WeekSheetEvent = {
  id: string;
  day: WeekDayKey;
  startTime: string;
  endTime?: string;
  title: string;
  location?: string;
  notes?: string;
};

export type WeekSheetWeek = {
  weekLabel?: string;
  /** Lundi de la semaine (YYYY-MM-DD). */
  weekStart?: string;
  events: WeekSheetEvent[];
};

export type WeekSheetData = {
  weekLabel?: string;
  weekStart?: string;
  events: WeekSheetEvent[];
  /** Plusieurs semaines extraites d'un même PDF. */
  weeks?: WeekSheetWeek[];
  sourcePdfKey?: string;
  uploadedAt?: string;
  uploadedBy?: string;
  rawTextPreview?: string;
  /** Reparse multi-semaines effectué depuis le PDF source. */
  multiWeekParsed?: boolean;
};

const WEEK_SHEET_DASHBOARD_CATEGORY: DashboardCategory = {
  id: 9001,
  moduleId: DASHBOARD_WEEK_SHEET_MODULE_ID,
  name: "Feuille de semaine",
  link: "#",
  img: "",
  description: "Planning de la semaine",
  allowedRoles: [],
  variant: "week-sheet",
};
