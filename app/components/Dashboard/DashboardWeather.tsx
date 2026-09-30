"use client";

import { useEffect, useState } from "react";
import { dash } from "@/app/lib/dashboard-brand";
import { DASH_CHIP_SHELL, DASH_WEATHER_WIDTH } from "@/app/lib/dashboard-chip";

type WeatherPayload = {
  location: string;
  temperature: number | null;
  min: number | null;
  max: number | null;
  windKmh: number | null;
  label: string;
  icon: string;
};

type Props = {
  /** Variante dense pour la sidebar (logo + météo sur une ligne). */
  compact?: boolean;
};

export default function DashboardWeather({ compact = false }: Props) {
  const [weather, setWeather] = useState<WeatherPayload | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetch("/api/weather")
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (!cancelled && data && !data.error) setWeather(data as WeatherPayload);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  if (compact) {
    if (!weather || weather.temperature === null) {
      return (
        <div
          className="flex min-w-0 flex-1 items-center gap-1.5 rounded-xl bg-white/70 px-2 py-1.5 animate-pulse"
          aria-hidden
          title="Chargement météo"
        >
          <span className="h-5 w-5 rounded-full bg-stone-200/80" />
          <span className="h-3 w-10 rounded bg-stone-200/80" />
        </div>
      );
    }
    return (
      <div
        className="flex min-w-0 flex-1 items-center gap-1.5 overflow-hidden rounded-xl bg-white/80 px-2 py-1.5 shadow-sm ring-1 ring-black/5"
        title={`Météo à ${weather.location} — ${weather.label}`}
      >
        <span className="shrink-0 text-lg leading-none" aria-hidden>
          {weather.icon}
        </span>
        <span className={`shrink-0 text-sm font-black tabular-nums ${dash.ink}`}>
          {weather.temperature}°
        </span>
        <span className="min-w-0 truncate text-[10px] font-semibold text-stone-500">
          {weather.label}
        </span>
      </div>
    );
  }

  if (!weather || weather.temperature === null) {
    return (
      <div
        className={`${DASH_CHIP_SHELL} ${DASH_WEATHER_WIDTH} animate-pulse`}
        aria-hidden
        title="Chargement météo"
      >
        <span className="h-7 w-7 rounded-full bg-stone-200/80" />
        <div className="min-w-0 flex-1 space-y-1.5">
          <div className="h-2.5 w-16 rounded bg-stone-200/80" />
          <div className="h-4 w-20 rounded bg-stone-200/80" />
          <div className="h-2 w-24 rounded bg-stone-100" />
        </div>
      </div>
    );
  }

  return (
    <div className={`${DASH_CHIP_SHELL} ${DASH_WEATHER_WIDTH}`} title={`Météo à ${weather.location}`}>
      <span className="text-2xl leading-none" aria-hidden>
        {weather.icon}
      </span>
      <div className="min-w-0 flex-1 overflow-hidden">
        <p className={`truncate text-[10px] font-bold uppercase tracking-widest ${dash.label}`}>
          {weather.location}
        </p>
        <p className="flex items-baseline gap-x-1.5">
          <span className={`text-lg font-black tabular-nums leading-tight ${dash.ink}`}>
            {weather.temperature}°
          </span>
          <span className="truncate text-xs font-semibold text-stone-600">{weather.label}</span>
        </p>
        <p className="truncate text-[10px] font-medium leading-tight text-stone-400">
          {weather.min !== null && weather.max !== null
            ? `${weather.min}° · ${weather.max}°`
            : weather.min !== null
              ? `Min ${weather.min}°`
              : weather.max !== null
                ? `Max ${weather.max}°`
                : "\u00a0"}
          {weather.windKmh != null ? ` · vent ${weather.windKmh} km/h` : ""}
        </p>
      </div>
    </div>
  );
}
