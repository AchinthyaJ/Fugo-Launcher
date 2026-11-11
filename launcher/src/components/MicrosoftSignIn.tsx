import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, Copy, ExternalLink, Loader2, RotateCcw, X } from "lucide-react";
import type { Account } from "../model";

/** What the launcher needs to finish a Microsoft sign-in (from `microsoft_begin_login`). */
interface LoginPrompt {
  userCode: string;
  verificationUri: string;
  deviceCode: string;
  interval: number;
  expiresIn: number;
}

interface MicrosoftSignInProps {
  onSignedIn: (account: Account) => void;
  onClose: () => void;
}

/**
 * The Microsoft sign-in: a code to type on Microsoft's page, which opens in the browser. The
 * launcher waits until the player approves it, then hands over the account.
 */
export default function MicrosoftSignIn({ onSignedIn, onClose }: MicrosoftSignInProps) {
  const [prompt, setPrompt] = useState<LoginPrompt | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  // Each attempt is a new code. A ref keeps React's development double-run from asking twice.
  const [attempt, setAttempt] = useState(0);
  const startedFor = useRef(-1);

  useEffect(() => {
    if (startedFor.current === attempt) return;
    startedFor.current = attempt;
    setError(null);
    setPrompt(null);
    if (!isTauri()) {
      setError("Microsoft sign-in works in the launcher app, not in a browser.");
      return;
    }
    (async () => {
      try {
        const next = await invoke<LoginPrompt>("microsoft_begin_login");
        setPrompt(next);
        openUrl(next.verificationUri).catch(() => {});
        const account = await invoke<Account>("microsoft_finish_login", { prompt: next });
        onSignedIn(account);
      } catch (e) {
        setError(String(e));
      }
    })();
    // The sign-in is started once per attempt; the callbacks are only read when it finishes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [attempt]);

  const cancel = () => {
    if (isTauri()) invoke("microsoft_cancel_login").catch(() => {});
    onClose();
  };

  const copyCode = async () => {
    if (!prompt) return;
    try {
      await navigator.clipboard.writeText(prompt.userCode);
      setCopied(true);
      setTimeout(() => setCopied(false), 1600);
    } catch {
      // The code is on screen anyway; a failed copy costs nothing.
    }
  };

  return (
    <div role="dialog" aria-modal="true" aria-labelledby="ms-signin-title" className="fixed inset-0 z-[70] flex items-center justify-center bg-[#05060A]/70 backdrop-blur-sm p-6 select-none">
      <div className="fugo-rise glass relative w-full max-w-md rounded-[22px] p-7 text-center">
        <button
          type="button"
          onClick={cancel}
          aria-label="Cancel sign-in"
          className="absolute right-4 top-4 flex h-8 w-8 items-center justify-center rounded-full text-white/60 hover:bg-white/10 hover:text-white"
        >
          <X className="h-4 w-4" />
        </button>

        <p className="text-[11px] font-bold uppercase tracking-[0.25em] text-[#7DD3FC]">Microsoft account</p>
        <h2 id="ms-signin-title" className="mt-2 text-2xl font-black tracking-tight text-white">
          Sign in to play online
        </h2>

        {error ? (
          <div className="mt-6">
            <p role="alert" className="rounded-xl bg-[#F87171]/10 px-4 py-3 text-[13px] leading-relaxed text-[#FCA5A5]">
              {error}
            </p>
            <div className="mt-6 flex items-center justify-center gap-3">
              <button
                type="button"
                onClick={cancel}
                className="rounded-full px-5 py-2.5 text-[13px] font-semibold text-white/70 hover:text-white"
              >
                Close
              </button>
              <button
                type="button"
                onClick={() => setAttempt((n) => n + 1)}
                className="flex items-center gap-2 rounded-full bg-gradient-to-r from-[#E86A17] to-[#F5B041] px-5 py-2.5 text-[13px] font-bold text-[#07080D]"
              >
                <RotateCcw className="h-4 w-4" /> Try again
              </button>
            </div>
          </div>
        ) : !prompt ? (
          <div className="mt-10 flex flex-col items-center gap-3 text-[13px] text-[#8E96A8]">
            <Loader2 className="h-6 w-6 animate-spin text-[#F5B041]" />
            Getting your code from Microsoft…
          </div>
        ) : (
          <>
            <p className="mt-3 text-[13px] leading-relaxed text-[#B8BFCF]">
              Your browser opened Microsoft's sign-in page. Type this code there, then sign in with the account
              that owns Minecraft: Java Edition.
            </p>

            <div className="mt-6 flex items-center justify-center gap-3">
              <span className="rounded-2xl border border-white/15 bg-white/[0.06] px-6 py-4 font-mono text-[28px] font-black tracking-[0.3em] text-white">
                {prompt.userCode}
              </span>
              <button
                type="button"
                onClick={copyCode}
                aria-label="Copy code"
                className="flex h-12 w-12 items-center justify-center rounded-2xl border border-white/15 bg-white/[0.06] text-white/70 hover:text-white"
              >
                {copied ? <Check className="h-4 w-4 text-[#34D399]" /> : <Copy className="h-4 w-4" />}
              </button>
            </div>

            <button
              type="button"
              onClick={() => openUrl(prompt.verificationUri).catch(() => {})}
              className="mt-5 inline-flex items-center gap-1.5 text-[12px] font-semibold text-[#7DD3FC] hover:text-white"
            >
              Open the sign-in page again <ExternalLink className="h-3.5 w-3.5" />
            </button>

            <p className="mt-7 flex items-center justify-center gap-2 text-[12px] text-[#8E96A8]">
              <Loader2 className="h-3.5 w-3.5 animate-spin" /> Waiting for you to approve in the browser…
            </p>
          </>
        )}
      </div>
    </div>
  );
}
