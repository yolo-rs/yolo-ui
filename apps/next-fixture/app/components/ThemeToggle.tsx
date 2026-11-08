"use client";

import { useState } from "react";
import dynamic from "next/dynamic";

const HeavyChart = dynamic(() => import("./HeavyChart"), {
  ssr: false,
  loading: () => <p>chart</p>,
});

export function ThemeToggle() {
  const [theme, setTheme] = useState<"dark" | "light">("dark");
  const toggle = () => {
    const next = theme === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    setTheme(next);
  };
  return (
    <div>
      <button type="button" onClick={toggle} aria-pressed={theme === "light"}>
        Theme: {theme}
      </button>
      <HeavyChart />
    </div>
  );
}
