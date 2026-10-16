import type { ReactNode } from "react";
import { ApiError } from "../api/http";
import { vi } from "../i18n/vi";

/**
 * Small round status dot.
 *
 * @param props - CSS colour of the dot.
 */
export function Dot({ color }: { color: string }) {
  return <span className="dot" style={{ background: color }} aria-hidden="true" />;
}

/**
 * Rounded label, or a button styled as one when `onClick` is set.
 *
 * @param props - Content, variant and optional click handler.
 */
export function Pill({
  children,
  variant = "plain",
  onClick,
  disabled,
  type = "button",
}: {
  children: ReactNode;
  variant?: "plain" | "alert" | "warn" | "accent";
  onClick?: () => void;
  disabled?: boolean;
  type?: "button" | "submit";
}) {
  const className = `pill pill-${variant}`;
  if (!onClick && type !== "submit") return <span className={className}>{children}</span>;
  return (
    <button type={type} className={className} onClick={onClick} disabled={disabled}>
      {children}
    </button>
  );
}

/**
 * Vietnamese text of an error thrown by the API layer.
 *
 * @param error - Error from a query or mutation.
 * @returns The message to show.
 */
export function errorText(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.status === 403) return vi.common.forbidden;
    return `${vi.common.errorPrefix}${error.message}`;
  }
  if (error instanceof Error && error.name === "SessionExpiredError") return vi.common.sessionExpired;
  return vi.common.genericError;
}

/**
 * Show a loading line or an error line for a query, or nothing.
 *
 * @param props - Query flags and error.
 */
export function QueryStatus({ isLoading, error }: { isLoading: boolean; error: unknown }) {
  if (isLoading) return <p className="muted small">{vi.common.loading}</p>;
  if (error) return <p className="error small">{errorText(error)}</p>;
  return null;
}

/**
 * Muted line shown when a list is empty.
 *
 * @param props - Text to show.
 */
export function Empty({ children }: { children: ReactNode }) {
  return <p className="empty">{children}</p>;
}
