import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { vi } from "../i18n/vi";

/** One toast message. */
interface Toast {
  id: number;
  text: string;
}

/** Function that shows a toast. */
export type ShowToast = (text: string) => void;

const ToastContext = createContext<ShowToast>(() => undefined);
const TOAST_MS = 8_000;

/**
 * Provide the toast area and the {@link useToast} function.
 *
 * @param props - Children that may show toasts.
 */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const dismiss = useCallback((id: number) => setToasts((all) => all.filter((t) => t.id !== id)), []);
  const show = useCallback<ShowToast>(
    (text) => {
      const id = Date.now() + Math.random();
      setToasts((all) => [...all.slice(-3), { id, text }]);
      setTimeout(() => dismiss(id), TOAST_MS);
    },
    [dismiss],
  );
  const value = useMemo(() => show, [show]);
  return (
    <ToastContext.Provider value={value}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className="toast">
            <span>{t.text}</span>
            <button type="button" className="link" onClick={() => dismiss(t.id)} aria-label={vi.toast.close}>
              {"×"}
            </button>
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

/**
 * Get the function that shows a toast.
 *
 * @returns The toast function.
 */
export function useToast(): ShowToast {
  return useContext(ToastContext);
}
