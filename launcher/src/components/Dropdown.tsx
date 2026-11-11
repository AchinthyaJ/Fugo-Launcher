import { useEffect, useRef, useState } from "react";
import { ChevronDown } from "lucide-react";

export interface DropdownOption<T extends string> {
  value: T;
  label: string;
}

interface Props<T extends string> {
  value: T;
  options: DropdownOption<T>[];
  onChange: (value: T) => void;
  ariaLabel?: string;
  className?: string;
}

/** The app's own dropdown: same height and surface as the text fields, dark list, no browser UI. */
export default function Dropdown<T extends string>({ value, options, onChange, ariaLabel, className = "" }: Props<T>) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onPointer = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const current = options.find((o) => o.value === value);

  return (
    <div ref={ref} className={`relative ${className}`}>
      <button
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className="h-9 w-full flex items-center justify-between gap-2 px-3 rounded-xl bg-[#141824] border border-[#23293D] text-[13px] text-white hover:border-[#E86A17]/60 focus:outline-none focus:border-[#E86A17] cursor-pointer"
      >
        <span className="truncate">{current?.label}</span>
        <ChevronDown className={`w-3.5 h-3.5 shrink-0 text-[#8E96A8] transition-transform ${open ? "rotate-180" : ""}`} />
      </button>
      {open && (
        <ul
          role="listbox"
          className="absolute left-0 z-50 mt-1 min-w-full w-max max-h-72 overflow-auto rounded-xl bg-[#0E121C] border border-[#23293D] shadow-2xl py-1"
        >
          {options.map((o) => (
            <li
              key={o.value}
              role="option"
              aria-selected={o.value === value}
              onClick={() => {
                onChange(o.value);
                setOpen(false);
              }}
              className={`px-3 py-1.5 text-[13px] whitespace-nowrap cursor-pointer ${
                o.value === value ? "text-[#E86A17] bg-[#E86A17]/10" : "text-white hover:bg-white/10"
              }`}
            >
              {o.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
