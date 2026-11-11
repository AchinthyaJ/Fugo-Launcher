import { useState } from "react";
import { BadgeCheck, Check, Plus, Trash2 } from "lucide-react";
import type { Account } from "../model";

interface AccountSwitcherProps {
  accounts: Account[];
  onSwitch: (id: string) => void;
  onAdd: (username: string) => Promise<void>;
  onRemove: (id: string) => Promise<void>;
  onMicrosoft: () => void;
}

/** The profile menu: pick the active account, add an offline one, sign in with Microsoft, or remove one. */
export default function AccountSwitcher({ accounts, onSwitch, onAdd, onRemove, onMicrosoft }: AccountSwitcherProps) {
  const [name, setName] = useState("");
  // The account whose removal is waiting for a second click. Removing is the only destructive
  // step here, so it takes two: a stray click on the trash icon deletes nothing.
  const [confirming, setConfirming] = useState<string | null>(null);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const username = name.trim();
    if (!username) return;
    setName("");
    await onAdd(username);
  };

  const remove = async (id: string) => {
    setConfirming(null);
    await onRemove(id);
  };

  return (
    <div role="menu" className="glass w-60 rounded-2xl p-2 flex flex-col gap-0.5">
      {accounts.map((a) =>
        confirming === a.id ? (
          <div key={a.id} className="flex items-center gap-2 px-2.5 py-2 rounded-xl bg-[#F87171]/10">
            <span className="flex-1 min-w-0 truncate text-[12px] text-[#FCA5A5]" title={a.username}>Remove?</span>
            <button
              type="button"
              onClick={() => setConfirming(null)}
              className="px-2 py-1 rounded-lg text-[12px] font-semibold text-white/70 hover:bg-white/10"
            >
              Keep
            </button>
            <button
              type="button"
              onClick={() => remove(a.id)}
              className="px-2 py-1 rounded-lg text-[12px] font-bold text-white bg-[#DC2626] hover:bg-[#EF4444]"
            >
              Yes, remove
            </button>
          </div>
        ) : (
          <div key={a.id} className="group flex items-center rounded-xl hover:bg-white/10 transition-colors duration-200">
            <button
              type="button"
              role="menuitemradio"
              aria-checked={a.is_active}
              onClick={() => onSwitch(a.id)}
              className="flex flex-1 min-w-0 items-center gap-3 px-2.5 py-2 text-left"
            >
              <span className="w-6 h-6 rounded-full overflow-hidden bg-[#22293F] shrink-0">
                <img src="/resources/Steve_classic.png" alt="" className="w-full h-full object-cover object-top scale-125" />
              </span>
              <span className="flex-1 min-w-0 truncate text-[13px] text-white">{a.username}</span>
              {!a.is_offline && (
                <span className="shrink-0 rounded-md bg-[#7DD3FC]/15 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-[#7DD3FC]">
                  Microsoft
                </span>
              )}
              {a.is_active && <Check className="w-3.5 h-3.5 text-white/80 shrink-0" />}
            </button>
            <button
              type="button"
              onClick={() => setConfirming(a.id)}
              aria-label={`Remove ${a.username}`}
              title="Remove account"
              className="mr-1.5 w-7 h-7 shrink-0 rounded-full flex items-center justify-center text-white/40 opacity-0 transition-all duration-200 hover:bg-[#F87171]/15 hover:text-[#FCA5A5] group-hover:opacity-100 focus:opacity-100"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        )
      )}

      <button
        type="button"
        onClick={onMicrosoft}
        className="mt-1 flex items-center gap-2.5 rounded-xl px-2.5 py-2 text-left text-[13px] font-semibold text-[#7DD3FC] hover:bg-white/10 transition-colors duration-200"
      >
        <BadgeCheck className="h-4 w-4 shrink-0" /> Sign in with Microsoft
      </button>

      <form onSubmit={submit} className="flex items-center gap-2 mt-1 pt-2 px-1 border-t border-white/10">
        <input
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Add offline account"
          maxLength={16}
          className="flex-1 min-w-0 bg-transparent text-[13px] text-white placeholder-white/40 outline-none px-1 py-1 select-text"
        />
        <button
          type="submit"
          aria-label="Add account"
          className="w-7 h-7 rounded-full flex items-center justify-center text-white/70 hover:bg-white/10 transition-colors duration-200"
        >
          <Plus className="w-3.5 h-3.5" />
        </button>
      </form>
    </div>
  );
}
