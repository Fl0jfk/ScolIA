"use client";

import Link from "next/link";
import type { HomeMainTask } from "@/app/lib/home-main-tasks";

type Props = {
  tasks: HomeMainTask[];
};

/**
 * Bento « mes tâches » — raccourcis vitaux selon le rôle (pas des notifs).
 */
export default function HomeMainTasks({ tasks }: Props) {
  if (tasks.length === 0) return null;

  return (
    <div className="space-y-3">
      <div className="flex items-end justify-between gap-3 px-1">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-[0.18em] text-[var(--dash-mid)]">
            Mes tâches
          </p>
          <h2 className="text-lg font-semibold tracking-tight text-[var(--dash-ink)]">
            Accès rapides selon votre rôle
          </h2>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {tasks.map((task, index) => {
          const hero = index === 0;
          return (
            <Link
              key={task.id}
              href={task.href}
              className={`group relative flex min-h-[8.5rem] flex-col justify-between overflow-hidden rounded-[1.75rem] p-5 transition duration-300 hover:-translate-y-0.5 hover:shadow-[0_18px_40px_-24px_rgba(0,0,0,0.4)] ${
                hero
                  ? "bg-[color:var(--dash-lime)] text-[var(--dash-ink)] ring-1 ring-black/5 sm:col-span-2 lg:col-span-1"
                  : "bg-white text-[var(--dash-ink)] ring-1 ring-black/6"
              }`}
            >
              <div className="relative flex items-start justify-between gap-3">
                <span
                  className={`inline-flex h-11 w-11 items-center justify-center rounded-2xl text-xl ${
                    hero ? "bg-black/10" : "bg-[color:var(--dash-soft-muted)]"
                  }`}
                  aria-hidden
                >
                  {task.emoji}
                </span>
                <span
                  className={`inline-flex h-9 w-9 items-center justify-center rounded-full text-lg transition group-hover:translate-x-0.5 ${
                    hero ? "bg-[var(--dash-ink)] text-white" : "bg-[var(--dash-ink)] text-white"
                  }`}
                  aria-hidden
                >
                  →
                </span>
              </div>
              <div className="relative mt-4 space-y-1">
                <p className="text-xl font-semibold tracking-tight">{task.label}</p>
                <p className={`text-sm ${hero ? "text-[var(--dash-ink)]/65" : "text-neutral-500"}`}>
                  {task.detail}
                </p>
              </div>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
