import { useState } from "react";
import { Plus } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { Instance } from "../model";

/** Time-of-day greeting for the local clock. */
export function greetingFor(date: Date): string {
  const hour = date.getHours();
  if (hour < 12) return "Good Morning";
  if (hour < 18) return "Good Afternoon";
  return "Good Evening";
}

/** A random translucent glass tint: the fill and the rim each get their own random opacity. */
function randomGlass(): { fill: string; stroke: string } {
  const a = (min: number, max: number) => (min + Math.random() * (max - min)).toFixed(3);
  return {
    fill: `rgba(255, 255, 255, ${a(0.02, 0.14)})`,
    stroke: `rgba(255, 255, 255, ${a(0.25, 0.6)})`,
  };
}

interface HomeHeroProps {
  download: { stage: string; done: number; total: number } | null;
  username: string;
  instances: Instance[];
  selected: Instance | undefined;
  running: boolean;
  onSelect: (id: string) => void;
  onLaunch: () => void;
  onNewInstance: () => void;
  onOpenDashboard?: (inst: Instance) => void;
}

/**
 * Home: a centred greeting, the hexagon play button naming the selected edition,
 * and a minimal row of instance names.
 */
export default function HomeHero({
  download,
  username,
  instances,
  selected,
  running,
  onSelect,
  onLaunch,
  onNewInstance,
  onOpenDashboard,
}: HomeHeroProps) {
  const edition = selected ? (selected.edition === "rust" ? "Rust" : "Java") : "";
  const [glass] = useState(randomGlass);

  return (
    <section className="flex flex-col items-center gap-8 w-full max-w-[640px]">
      <h1 className="text-[34px] leading-none font-bold uppercase tracking-[0.14em] text-white text-center drop-shadow-[0_2px_14px_rgba(0,0,0,0.6)]">
        {greetingFor(new Date())}, {username}
      </h1>

      <button
        type="button"
        onClick={onLaunch}
        disabled={!selected}
        className="hex-btn hex-btn-orange relative w-[300px] h-16 flex items-center justify-center text-white disabled:opacity-40"
      >
        <div aria-hidden="true" className="hex-blur" />
        <svg aria-hidden="true" viewBox="0 0 300 64" className="absolute inset-0 w-full h-full overflow-visible">
          <polygon
            className="hex-shape"
            style={{ "--hex-fill": glass.fill, "--hex-stroke": glass.stroke } as React.CSSProperties}
            points="40,0.5 260,0.5 299.5,32 260,63.5 40,63.5 0.5,32"
          />
        </svg>
        <span className="hex-label relative text-[12px] font-semibold uppercase tracking-[0.3em]">
          {running ? `Stop ${edition}` : `Play ${edition}`}
        </span>
      </button>

      {selected?.edition === "rust" && (
        <p className="-mt-4 max-w-[420px] text-center text-[12px] leading-relaxed text-white/55">
          Horizon Rust Edition is not a project maintained by Fugo.{" "}
          <button
            type="button"
            onClick={() => openUrl("https://playhorizon.dev")}
            className="underline underline-offset-2 text-white/80 hover:text-white transition-colors duration-200"
          >
            Visit playhorizon.dev
          </button>{" "}
          to support it.
        </p>
      )}

      <nav aria-label="Instances" className="flex flex-wrap items-baseline justify-center gap-x-7 gap-y-2">
        {instances.map((inst) => {
          const isSelected = inst.id === selected?.id;
          return (
            <button
              key={inst.id}
              type="button"
              aria-pressed={isSelected}
              onClick={() => onSelect(inst.id)}
              onDoubleClick={() => {
                onSelect(inst.id);
                onOpenDashboard?.(inst);
              }}
              title={`${inst.name} (Double-click to open dashboard)`}
              className={`instance-name relative pb-1 tracking-wide transition-colors duration-300 ${
                isSelected ? "text-white" : "text-white/40 hover:text-white/80"
              }`}
            >
              {inst.name}
              {isSelected && <span className="absolute left-0 right-0 -bottom-px h-px bg-white" />}
            </button>
          );
        })}
        <button
          type="button"
          onClick={onNewInstance}
          aria-label="New instance"
          className="instance-name pb-1 text-white/40 hover:text-white/80 transition-colors duration-300"
        >
          <Plus className="w-3.5 h-3.5" />
        </button>
      </nav>

      {download && (
        <div className="flex flex-col items-center gap-2" role="status" aria-live="polite">
          {/* Minecraft's loading bar: a 1px white outline, a 2px gap, and a solid white fill. */}
          <div className="relative w-[240px] h-[10px] border border-white">
            <div className="absolute inset-[2px] overflow-hidden">
              <div
                className="h-full bg-white transition-[width] duration-300 ease-out"
                style={{ width: `${download.total ? Math.min(100, (download.done / download.total) * 100) : 0}%` }}
              />
            </div>
          </div>
          <span className="text-[11px] tracking-wide text-white/50">{download.stage}</span>
        </div>
      )}
    </section>
  );
}
