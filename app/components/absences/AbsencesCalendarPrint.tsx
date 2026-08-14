"use client";

import { escapeHtml } from "@/app/lib/escape-html";
import { appearanceForEvent, sortCalendarEvents, type CalendarEvent } from "@/app/lib/absences-calendar";

export function printDaySummary(date: Date, dayEvents: CalendarEvent[], teacherColorIndexMap: Map<string, number>) {
  const sorted = sortCalendarEvents(dayEvents);
  const dayTitle = date.toLocaleDateString("fr-FR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  const printedAt = new Date().toLocaleString("fr-FR", { dateStyle: "long", timeStyle: "short" });
  const countLabel = sorted.length <= 1 ? "1 absence" : `${sorted.length} absences`;

  const rows =
    sorted.length === 0
      ? `<p class="empty">Aucune absence enregistrée pour cette journée.</p>`
      : sorted
          .map((event) => {
            const style = appearanceForEvent(event, teacherColorIndexMap).print;
            return `
              <article class="card" style="background:${style.bg};color:${style.text};border-color:${style.border}">
                <div class="card-head">
                  <div class="teacher">${escapeHtml(event.displayName)}</div>
                  <span class="badge">${escapeHtml(event.reason || "Absence")}</span>
                </div>
                <p class="time">${escapeHtml(event.displayTime)}</p>
                ${event.hasDocument ? `<p class="doc">Justificatif PDF disponible</p>` : ""}
              </article>
            `;
          })
          .join("");

  const html = `
    <div class="absence-day-print-page">
      <div class="absence-day-print-top">
        <div class="absence-day-print-kicker">La Providence Nicolas Barré</div>
        <div class="absence-day-print-title">Absences professeurs</div>
        <div class="absence-day-print-date">Journée du ${escapeHtml(dayTitle)}</div>
        <div class="absence-day-print-meta">Document généré le ${escapeHtml(printedAt)}</div>
        <div class="absence-day-print-count">${escapeHtml(countLabel)}</div>
      </div>
      <div class="absence-day-print-list">${rows}</div>
      <div class="absence-day-print-bottom">Usage interne — calendrier des absences</div>
    </div>
  `;

  const PRINT_ROOT_ID = "absence-day-print-root";
  const PRINT_STYLE_ID = "absence-day-print-style";
  const PRINT_BODY_CLASS = "is-printing-absence-day";

  document.getElementById(PRINT_ROOT_ID)?.remove();
  document.getElementById(PRINT_STYLE_ID)?.remove();
  document.documentElement.classList.remove(PRINT_BODY_CLASS);
  document.body.classList.remove(PRINT_BODY_CLASS);

  const root = document.createElement("div");
  root.id = PRINT_ROOT_ID;
  root.setAttribute("aria-hidden", "true");
  root.innerHTML = html;

  const style = document.createElement("style");
  style.id = PRINT_STYLE_ID;
  style.textContent = `
    #${PRINT_ROOT_ID} { display: none; }

    html.${PRINT_BODY_CLASS},
    body.${PRINT_BODY_CLASS} {
      height: auto !important;
      min-height: 0 !important;
      overflow: visible !important;
      background: #fff !important;
    }

    body.${PRINT_BODY_CLASS} > *:not(#${PRINT_ROOT_ID}) {
      display: none !important;
    }

    body.${PRINT_BODY_CLASS} #${PRINT_ROOT_ID} {
      display: block !important;
      position: static !important;
      width: 100% !important;
      min-height: 0 !important;
      height: auto !important;
      margin: 0 !important;
      padding: 0 !important;
    }

    #${PRINT_ROOT_ID} .absence-day-print-page {
      box-sizing: border-box;
      width: 100%;
      margin: 0;
      padding: 0;
      font-family: "Segoe UI", system-ui, sans-serif;
      color: #0f172a;
      background: #fff;
    }
    #${PRINT_ROOT_ID} .absence-day-print-top {
      border-bottom: 2px solid #0f172a;
      padding-bottom: 14px;
      margin-bottom: 18px;
      page-break-after: avoid;
    }
    #${PRINT_ROOT_ID} .absence-day-print-kicker {
      margin: 0 0 8px;
      font-size: 11px;
      font-weight: 700;
      letter-spacing: 0.06em;
      text-transform: uppercase;
      color: #64748b;
    }
    #${PRINT_ROOT_ID} .absence-day-print-title {
      margin: 0;
      font-size: 28px;
      font-weight: 900;
      line-height: 1.1;
      color: #0f172a;
    }
    #${PRINT_ROOT_ID} .absence-day-print-date {
      margin: 12px 0 0;
      padding: 10px 12px;
      font-size: 20px;
      font-weight: 800;
      line-height: 1.25;
      color: #0f172a;
      text-transform: capitalize;
      background: #f1f5f9;
      border: 1px solid #cbd5e1;
      border-radius: 10px;
      -webkit-print-color-adjust: exact;
      print-color-adjust: exact;
    }
    #${PRINT_ROOT_ID} .absence-day-print-meta {
      margin: 10px 0 0;
      font-size: 11px;
      color: #64748b;
    }
    #${PRINT_ROOT_ID} .absence-day-print-count {
      display: inline-block;
      margin-top: 10px;
      padding: 5px 12px;
      border-radius: 999px;
      background: #e2e8f0;
      font-size: 12px;
      font-weight: 700;
      color: #334155;
    }
    #${PRINT_ROOT_ID} .absence-day-print-list { display: grid; gap: 10px; }
    #${PRINT_ROOT_ID} .card {
      border: 1px solid;
      border-radius: 12px;
      padding: 12px 14px;
      break-inside: avoid;
      page-break-inside: avoid;
      -webkit-print-color-adjust: exact;
      print-color-adjust: exact;
    }
    #${PRINT_ROOT_ID} .card-head {
      display: flex;
      flex-wrap: wrap;
      align-items: baseline;
      justify-content: space-between;
      gap: 8px;
    }
    #${PRINT_ROOT_ID} .card .teacher { margin: 0; font-size: 16px; font-weight: 800; }
    #${PRINT_ROOT_ID} .badge { font-size: 11px; font-weight: 700; opacity: 0.9; }
    #${PRINT_ROOT_ID} .time { margin: 8px 0 0; font-size: 13px; font-weight: 600; }
    #${PRINT_ROOT_ID} .doc { margin: 6px 0 0; font-size: 10px; opacity: 0.75; }
    #${PRINT_ROOT_ID} .empty {
      margin: 0;
      padding: 24px;
      text-align: center;
      color: #64748b;
      border: 1px dashed #cbd5e1;
      border-radius: 12px;
    }
    #${PRINT_ROOT_ID} .absence-day-print-bottom {
      margin-top: 16px;
      padding-top: 8px;
      border-top: 1px solid #e2e8f0;
      font-size: 10px;
      color: #94a3b8;
      text-align: center;
      page-break-after: avoid;
    }

    @media print {
      @page { size: auto; margin: 12mm; }
      html, body {
        height: auto !important;
        min-height: 0 !important;
        overflow: visible !important;
      }
    }
  `;

  document.head.appendChild(style);
  document.body.appendChild(root);
  document.documentElement.classList.add(PRINT_BODY_CLASS);
  document.body.classList.add(PRINT_BODY_CLASS);

  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    root.remove();
    style.remove();
    document.documentElement.classList.remove(PRINT_BODY_CLASS);
    document.body.classList.remove(PRINT_BODY_CLASS);
    window.removeEventListener("afterprint", cleanup);
  };

  window.addEventListener("afterprint", cleanup);
  window.setTimeout(cleanup, 60_000);
  window.setTimeout(() => window.print(), 80);
}

export function PrinterIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="M6 9V2h12v7" />
      <path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2" />
      <path d="M6 14h12v8H6z" />
    </svg>
  );
}

export function PaperclipIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      <path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48" />
    </svg>
  );
}
