import { useMemo, useState } from "react";
import { ArrowLeft, ArrowRight, BadgeCheck, Check, Cpu, Gamepad2, Layers, Minus, Sparkles, UserRound, X } from "lucide-react";
import ArtworkBackground from "./ArtworkBackground";

/** The skins a new offline account can start with. Each has a full-body render in /resources. */
export const STARTER_SKINS = ["Steve", "Alex", "Efe", "Noor", "Sunny", "Ari", "Kai", "Makena"];

/** Minecraft's rule for an offline name: 3 to 16 letters, digits or underscores. */
export function nameProblem(name: string): string | null {
  if (name.length === 0) return null;
  if (name.length < 3) return "At least 3 characters.";
  if (name.length > 16) return "At most 16 characters.";
  if (!/^[A-Za-z0-9_]+$/.test(name)) return "Letters, digits and underscores only.";
  return null;
}

const FEATURES = [
  { icon: Gamepad2, title: "Java Edition", text: "Minecraft as Mojang ships it, with your mods and modpacks." },
  { icon: Cpu, title: "Horizon Rust Edition", text: "The Rust rewrite, installed and kept up to date for you." },
  { icon: Layers, title: "Everything in one place", text: "Instances, skins, mods and servers, side by side." },
];

const CONFETTI_COLORS = ["#E86A17", "#F5B041", "#7DD3FC", "#A78BFA", "#F472B6", "#34D399"];

/** The screens in order: hero, choice, offline name and skin, done. */
type Step = 0 | 1 | 2 | 3;

interface WelcomeProps {
  onCreate: (username: string, skin: string) => Promise<void>;
  onMicrosoft: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

/** The first screen a player sees: shown while no account exists. */
export default function Welcome({ onCreate, onMicrosoft, onMinimize, onClose }: WelcomeProps) {
  const [step, setStep] = useState<Step>(0);
  const [name, setName] = useState("");
  const [skin, setSkin] = useState(STARTER_SKINS[0]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const trimmed = name.trim();
  const problem = nameProblem(trimmed);
  const valid = trimmed.length >= 3 && problem === null;

  // Fixed for the life of the screen, so the burst does not reshuffle on every keystroke.
  const confetti = useMemo(
    () =>
      Array.from({ length: 42 }, (_, i) => ({
        id: i,
        left: Math.random() * 100,
        delay: Math.random() * 0.5,
        duration: 2.2 + Math.random() * 1.6,
        color: CONFETTI_COLORS[i % CONFETTI_COLORS.length],
        rotate: Math.random() * 360,
      })),
    []
  );

  const finish = async () => {
    if (!valid || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onCreate(trimmed, skin);
      // On success the launcher swaps this screen for the main one; nothing more to do here.
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  };

  const nameNext = () => {
    if (valid) setStep(3);
  };

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-[#07080D] text-[#F3F4F6] select-none overflow-hidden">
      <ArtworkBackground src="/assets/backgrounds/1.png" />
      {/* A dark pool behind the text, so it reads over the bright landscape. */}
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 z-0 bg-[radial-gradient(ellipse_at_center,rgba(7,8,13,0.9)_0%,rgba(7,8,13,0.55)_50%,rgba(7,8,13,0.1)_80%)]" />

      {/* Aurora: two slow blurred glows behind everything. */}
      <div aria-hidden="true" className="pointer-events-none absolute inset-0 z-0 overflow-hidden">
        <div className="fugo-drift absolute -top-1/3 -left-1/4 h-[70vh] w-[70vh] rounded-full bg-[#E86A17]/25 blur-3xl" />
        <div className="fugo-drift absolute -bottom-1/3 -right-1/4 h-[70vh] w-[70vh] rounded-full bg-[#7DD3FC]/20 blur-3xl" />
      </div>

      {/* Window chrome: the frameless window still needs to drag, minimise and close. */}
      <header data-tauri-drag-region className="relative z-10 flex items-center justify-between px-5 pt-4">
        <div data-tauri-drag-region className="flex items-center gap-2.5">
          <img src="/assets/fugo-logo-transparent.png" alt="" className="w-7 h-7 object-contain" />
          <span className="text-[12px] font-extrabold tracking-[0.2em] text-white/90">FUGO</span>
        </div>
        <div className="glass flex items-center gap-0.5 h-9 px-1 rounded-full">
          <button type="button" onClick={onMinimize} className="win-btn" aria-label="Minimize">
            <Minus className="w-3.5 h-3.5" />
          </button>
          <button type="button" onClick={onClose} className="win-btn is-danger" aria-label="Close">
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </header>

      <main className="relative z-10 flex flex-1 items-center justify-center px-6 pb-8 overflow-y-auto">
        {step === 0 && (
          <section key="hero" className="fugo-rise flex max-w-3xl flex-col items-center text-center">
            <div className="relative mb-7">
              <div className="fugo-halo absolute inset-0 -m-6 rounded-full bg-[#E86A17]/35 blur-2xl" />
              <img
                src="/assets/fugo-logo-transparent.png"
                alt=""
                className="fugo-float relative w-28 h-28 object-contain drop-shadow-[0_0_30px_rgba(232,106,23,0.45)]"
              />
            </div>
            <p className="mb-3 flex items-center gap-2 text-[11px] font-bold uppercase tracking-[0.3em] text-[#F5B041]">
              <Sparkles className="w-3.5 h-3.5" /> Welcome
            </p>
            <h1 className="text-5xl font-black tracking-tight sm:text-6xl">Hello, adventurer.</h1>
            <p className="mt-4 max-w-xl text-[15px] leading-relaxed text-[#B8BFCF]">
              Fugo is your home for Minecraft. Install, play and tune every version from one calm, fast launcher.
              Let's set you up in under a minute.
            </p>

            <ul className="mt-9 grid w-full gap-3 sm:grid-cols-3">
              {FEATURES.map(({ icon: Icon, title, text }, i) => (
                <li
                  key={title}
                  className="fugo-rise glass rounded-2xl p-4 text-left"
                  style={{ animationDelay: `${0.15 + i * 0.1}s` }}
                >
                  <Icon className="mb-3 h-5 w-5 text-[#F5B041]" />
                  <h2 className="text-[13px] font-bold text-white">{title}</h2>
                  <p className="mt-1 text-[12px] leading-snug text-[#8E96A8]">{text}</p>
                </li>
              ))}
            </ul>

            <button
              type="button"
              onClick={() => setStep(1)}
              className="mt-10 flex items-center gap-2 rounded-full bg-gradient-to-r from-[#E86A17] to-[#F5B041] px-8 py-3.5 text-[14px] font-bold text-[#07080D] shadow-[0_10px_40px_-8px_rgba(232,106,23,0.7)] transition-transform duration-200 hover:scale-[1.04] active:scale-[0.98]"
            >
              Get started <ArrowRight className="h-4 w-4" />
            </button>
          </section>
        )}

        {step === 1 && (
          <section key="choose" className="fugo-rise flex w-full max-w-3xl flex-col items-center text-center">
            <p className="text-[11px] font-bold uppercase tracking-[0.3em] text-[#F5B041]">Step 1 of 3</p>
            <h1 className="mt-2 text-4xl font-black tracking-tight sm:text-5xl">How do you want to play?</h1>
            <p className="mt-3 max-w-lg text-[14px] text-[#B8BFCF]">
              You can add more accounts later from the top bar.
            </p>

            <div className="mt-9 grid w-full gap-4 md:grid-cols-2">
              <button
                type="button"
                onClick={onMicrosoft}
                className="group glass relative rounded-[22px] p-6 text-left transition-transform duration-200 hover:scale-[1.02] focus-visible:ring-2 focus-visible:ring-[#7DD3FC]"
              >
                <span className="absolute right-4 top-4 rounded-full bg-[#7DD3FC]/15 px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide text-[#7DD3FC]">
                  Recommended
                </span>
                <BadgeCheck className="mb-4 h-8 w-8 text-[#7DD3FC]" />
                <h2 className="text-lg font-bold text-white">Sign in with Microsoft</h2>
                <p className="mt-2 text-[13px] leading-relaxed text-[#B8BFCF]">
                  Use the account that owns Minecraft: Java Edition. Play online and keep your profile and skin.
                </p>
              </button>

              <button
                type="button"
                onClick={() => setStep(2)}
                className="glass rounded-[22px] p-6 text-left transition-transform duration-200 hover:scale-[1.02] focus-visible:ring-2 focus-visible:ring-[#F5B041]"
              >
                <UserRound className="mb-4 h-8 w-8 text-[#F5B041]" />
                <h2 className="text-lg font-bold text-white">Play offline</h2>
                <p className="mt-2 text-[13px] leading-relaxed text-[#B8BFCF]">
                  Pick a name and a skin that live on this computer. Works in single player and on offline servers.
                </p>
              </button>
            </div>

            <button
              type="button"
              onClick={() => setStep(0)}
              className="mt-8 flex items-center gap-2 rounded-full px-4 py-2.5 text-[13px] font-semibold text-white/70 hover:text-white"
            >
              <ArrowLeft className="h-4 w-4" /> Back
            </button>
          </section>
        )}

        {step === 2 && (
          <section key="name" className="fugo-rise grid w-full max-w-4xl items-center gap-10 md:grid-cols-[1fr_1.1fr]">
            <div className="relative mx-auto flex h-80 w-56 items-center justify-center">
              <div className="fugo-halo absolute inset-6 rounded-full bg-[#7DD3FC]/25 blur-3xl" />
              <img
                key={skin}
                src={`/resources/${skin}_classic.png`}
                alt={`${skin} skin`}
                className="fugo-float relative h-full w-auto drop-shadow-[0_18px_30px_rgba(0,0,0,0.5)]"
              />
            </div>

            <div className="flex flex-col">
              <p className="text-[11px] font-bold uppercase tracking-[0.3em] text-[#F5B041]">Offline account</p>
              <h1 className="mt-2 text-4xl font-black tracking-tight">Who's playing?</h1>
              <p className="mt-2 text-[14px] text-[#B8BFCF]">Pick a name and a look. You can change both any time.</p>

              <label className="mt-7 block">
                <span className="text-[12px] font-semibold text-white/80">Player name</span>
                <input
                  autoFocus
                  value={name}
                  maxLength={16}
                  onChange={(e) => setName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") nameNext();
                  }}
                  placeholder="Steve_42"
                  className="glass mt-2 w-full rounded-xl bg-transparent px-4 py-3 text-[16px] font-semibold text-white placeholder-white/30 outline-none ring-0 focus:ring-2 focus:ring-[#F5B041]/60 select-text"
                />
                <span className={`mt-2 block h-4 text-[12px] ${problem ? "text-[#F87171]" : "text-[#8E96A8]"}`}>
                  {problem ?? "3 to 16 letters, digits or underscores."}
                </span>
              </label>

              <div className="mt-4">
                <span className="text-[12px] font-semibold text-white/80">Starter skin</span>
                <div className="mt-2 grid grid-cols-4 gap-2.5 sm:grid-cols-8 md:grid-cols-4">
                  {STARTER_SKINS.map((s) => {
                    const chosen = s === skin;
                    return (
                      <button
                        key={s}
                        type="button"
                        onClick={() => setSkin(s)}
                        aria-pressed={chosen}
                        aria-label={s}
                        title={s}
                        className={`relative aspect-square rounded-xl p-1.5 transition-all duration-200 ${
                          chosen ? "bg-white/15 ring-2 ring-[#F5B041] scale-105" : "glass hover:bg-white/10 hover:scale-105"
                        }`}
                      >
                        <img src={`/resources/${s}_classic.png`} alt="" className="block h-full w-full object-contain" />
                      </button>
                    );
                  })}
                </div>
              </div>

              <div className="mt-8 flex items-center justify-between">
                <button
                  type="button"
                  onClick={() => setStep(1)}
                  className="flex items-center gap-2 rounded-full px-4 py-2.5 text-[13px] font-semibold text-white/70 hover:text-white"
                >
                  <ArrowLeft className="h-4 w-4" /> Back
                </button>
                <button
                  type="button"
                  disabled={!valid}
                  onClick={nameNext}
                  className="flex items-center gap-2 rounded-full bg-gradient-to-r from-[#E86A17] to-[#F5B041] px-6 py-3 text-[13px] font-bold text-[#07080D] transition-all duration-200 enabled:hover:scale-[1.04] disabled:opacity-40"
                >
                  Continue <ArrowRight className="h-4 w-4" />
                </button>
              </div>
            </div>
          </section>
        )}

        {step === 3 && (
          <section key="ready" className="fugo-rise relative flex w-full max-w-xl flex-col items-center text-center">
            {/* The burst: falling ribbons, played once when this screen appears. */}
            <div aria-hidden="true" className="pointer-events-none absolute inset-x-0 -top-10 h-[120vh] overflow-hidden">
              {confetti.map((c) => (
                <span
                  key={c.id}
                  className="fugo-confetti"
                  style={{
                    left: `${c.left}%`,
                    background: c.color,
                    animationDelay: `${c.delay}s`,
                    animationDuration: `${c.duration}s`,
                    transform: `rotate(${c.rotate}deg)`,
                  }}
                />
              ))}
            </div>

            <div className="relative flex h-72 w-44 items-center justify-center">
              <div className="fugo-halo absolute inset-6 rounded-full bg-[#F5B041]/30 blur-3xl" />
              <img
                src={`/resources/${skin}_classic.png`}
                alt={`${skin} skin`}
                className="fugo-float relative h-full w-auto drop-shadow-[0_18px_30px_rgba(0,0,0,0.5)]"
              />
            </div>
            <span className="mt-4 flex h-9 w-9 items-center justify-center rounded-full bg-[#34D399]/20 text-[#34D399]">
              <Check className="h-5 w-5" />
            </span>
            <h1 className="mt-4 text-4xl font-black tracking-tight">Welcome, {trimmed}!</h1>
            <p className="mt-3 max-w-md text-[14px] leading-relaxed text-[#B8BFCF]">
              Your account is ready. Create an instance next, or add more accounts and skins from the top bar later.
            </p>

            {error && (
              <p role="alert" className="mt-5 max-w-md rounded-xl bg-[#F87171]/10 px-4 py-2.5 text-[13px] text-[#FCA5A5]">
                {error}
              </p>
            )}

            <div className="mt-8 flex items-center gap-4">
              <button
                type="button"
                onClick={() => setStep(2)}
                disabled={busy}
                className="flex items-center gap-2 rounded-full px-4 py-2.5 text-[13px] font-semibold text-white/70 hover:text-white disabled:opacity-40"
              >
                <ArrowLeft className="h-4 w-4" /> Change
              </button>
              <button
                type="button"
                onClick={finish}
                disabled={busy}
                className="flex items-center gap-2 rounded-full bg-gradient-to-r from-[#E86A17] to-[#F5B041] px-8 py-3.5 text-[14px] font-bold text-[#07080D] shadow-[0_10px_40px_-8px_rgba(232,106,23,0.7)] transition-transform duration-200 enabled:hover:scale-[1.04] disabled:opacity-60"
              >
                {busy ? "Setting up…" : "Enter Fugo"} {!busy && <ArrowRight className="h-4 w-4" />}
              </button>
            </div>
          </section>
        )}
      </main>

      {/* Progress dots */}
      <footer className="relative z-10 flex justify-center gap-2 pb-6">
        {[0, 1, 2, 3].map((i) => (
          <span
            key={i}
            className={`h-1.5 rounded-full transition-all duration-300 ${i === step ? "w-6 bg-[#F5B041]" : "w-1.5 bg-white/20"}`}
          />
        ))}
      </footer>
    </div>
  );
}
