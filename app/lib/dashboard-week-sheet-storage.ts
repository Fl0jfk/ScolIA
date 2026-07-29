import { getJson, putJson } from "@/app/lib/s3-storage";
import {
  WEEK_SHEET_STORAGE_PATH,
  type WeekSheetData,
  type WeekDayKey,
} from "@/app/lib/dashboard-week-sheet-types";
import { pickActiveWeekSheet } from "@/app/lib/dashboard-week-sheet-active";
import { todaySchoolWeekDayIndex } from "@/app/lib/dashboard-week";

export async function loadWeekSheetData(): Promise<WeekSheetData | null> {
  const hit = await getJson<WeekSheetData>(WEEK_SHEET_STORAGE_PATH);
  if (!hit?.data || !Array.isArray(hit.data.events)) return null;
  return hit.data;
}

export async function saveWeekSheetData(data: WeekSheetData): Promise<void> {
  await putJson(WEEK_SHEET_STORAGE_PATH, data);
  try {
    const { syncWeekSheetActualite } = await import("@/app/lib/brain-ai/sync/knowledge-writer");
    const active = pickActiveWeekSheet(data);
    const idx = todaySchoolWeekDayIndex();
    const keys: WeekDayKey[] = ["mon", "tue", "wed", "thu", "fri"];
    const dayKey = idx >= 0 ? keys[idx] : null;
    const todayTitles = dayKey
      ? (active.events || []).filter((e) => e.day === dayKey).map((e) => e.title)
      : [];
    void syncWeekSheetActualite({
      weekLabel: active.weekLabel,
      weekStart: active.weekStart,
      eventCount: (active.events || []).length,
      todayTitles,
    });
  } catch (err) {
    console.warn("[week-sheet] sync actualite failed", err);
  }
}
