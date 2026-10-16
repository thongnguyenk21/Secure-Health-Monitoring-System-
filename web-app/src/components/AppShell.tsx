import type { ReactNode } from "react";
import { NavLink } from "react-router-dom";

/** One item of the left navigation. */
export interface NavItem {
  /** Target path. */
  to: string;
  /** Visible label. */
  label: string;
  /** Optional red badge (for example the open alert count). */
  badge?: string | null;
  /** Match the path exactly instead of as a prefix. */
  end?: boolean;
}

/**
 * Two-column page frame: the left navigation and the content.
 *
 * @param props - Brand, items, footer of the navigation and content.
 */
export function AppShell({
  brand,
  items,
  footer,
  children,
}: {
  brand: string;
  items: NavItem[];
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="shell">
      <nav className="bnav">
        <b>{brand}</b>
        {items.map((item) => (
          <NavLink key={item.to} to={item.to} end={item.end} className={({ isActive }) => (isActive ? "on" : undefined)}>
            {item.label}
            {item.badge ? <em>{item.badge}</em> : null}
          </NavLink>
        ))}
        {footer ? <div className="bnav-foot">{footer}</div> : null}
      </nav>
      <main className="shell-main">{children}</main>
    </div>
  );
}
