"use client";

import type { HTMLAttributes, ReactNode } from "react";

export default function ModuleCard({
  children,
  className = "",
  bodyClassName = "",
  ...rest
}: {
  children: ReactNode;
  bodyClassName?: string;
} & HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={`relative overflow-hidden rounded-[1.35rem] border border-black/6 bg-white shadow-[0_12px_40px_-28px_rgba(15,23,42,0.45)] ${className}`}
      {...rest}
    >
      <div className={`relative z-[1] ${bodyClassName}`}>{children}</div>
    </div>
  );
}
