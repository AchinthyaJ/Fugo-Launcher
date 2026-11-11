import { useState } from "react";
import { Home as HomeIcon, Folder, Puzzle, Palette, Settings as SettingsIcon, Server } from "lucide-react";
import type { Tab } from "../model";

const ITEMS: { tab: Tab; label: string; Icon: typeof HomeIcon }[] = [
  { tab: "home", label: "Home", Icon: HomeIcon },
  { tab: "instances", label: "Instances", Icon: Folder },
  { tab: "mods", label: "Mods", Icon: Puzzle },
  { tab: "customize", label: "Customize", Icon: Palette },
  { tab: "servers", label: "Servers", Icon: Server },
  { tab: "settings", label: "Settings", Icon: SettingsIcon },
];

/** Smooth magnification curve: hovered is 1.18x, immediate neighbors 1.08x. */
function magnification(hover: number | null, index: number): number {
  if (hover === null) return 1;
  if (hover === index) return 1.18;
  return Math.abs(hover - index) === 1 ? 1.08 : 1;
}

/**
 * Dynamic horizontal spacing:
 * Hovered item gets increased margin (+8px on each side),
 * immediate neighbors get +5px, and outer components adjust (+2px),
 * so that component and other components smoothly increase their spacing
 * while the dock capsule itself stays rock-solid at fixed width without expanding.
 */
function itemSpacing(hover: number | null, index: number): number {
  if (hover === null) return 3;
  if (hover === index) return 8;
  if (Math.abs(hover - index) === 1) return 5;
  return 2;
}

interface FloatingDockProps {
  active: Tab;
  onSelect: (tab: Tab) => void;
}

export default function FloatingDock({ active, onSelect }: FloatingDockProps) {
  const [hover, setHover] = useState<number | null>(null);

  return (
    <nav
      aria-label="Main"
      onMouseLeave={() => setHover(null)}
      className="glass-dock flex items-center justify-center h-11 px-3 rounded-full select-none w-auto lg:w-[590px] mx-auto overflow-visible shadow-2xl flex-shrink-0"
    >
      {ITEMS.map(({ tab, label, Icon }, index) => {
        const isActive = tab === active;
        const scale = magnification(hover, index);
        const marginX = itemSpacing(hover, index);

        return (
          <button
            key={tab}
            type="button"
            title={label}
            aria-label={label}
            aria-current={isActive ? "page" : undefined}
            onMouseEnter={() => setHover(index)}
            onFocus={() => setHover(index)}
            onBlur={() => setHover(null)}
            onClick={() => onSelect(tab)}
            style={{
              marginLeft: `${marginX}px`,
              marginRight: `${marginX}px`,
            }}
            className={`dock-item relative flex items-center justify-center h-8 px-3 rounded-full font-medium cursor-pointer transition-all duration-250 ease-out ${
              isActive ? "is-active" : ""
            }`}
          >
            {/* Unified container for icon + text + exact scaled spacing between them */}
            <div
              className="flex items-center origin-center transition-all duration-250 ease-out will-change-transform"
              style={{
                transform: `scale(${scale})`,
                gap: `${Math.round(6 * scale)}px`,
              }}
            >
              <Icon className="w-4 h-4 shrink-0 transition-transform duration-200" />
              <span className="hidden lg:inline-block whitespace-nowrap text-[12px] font-semibold tracking-tight transition-all duration-200">
                {label}
              </span>
            </div>
          </button>
        );
      })}
    </nav>
  );
}
