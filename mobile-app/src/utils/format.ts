import { vi } from "../i18n/vi";

function pad2(n: number): string {
  return String(n).padStart(2, "0");
}

/**
 * Format a time of day in the phone's time zone.
 *
 * @param unixMs - UTC epoch milliseconds.
 * @returns `HH:MM`, 24-hour clock.
 */
export function formatTime(unixMs: number): string {
  const d = new Date(unixMs);
  return `${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

/**
 * Short Vietnamese weekday name in the phone's time zone.
 *
 * @param unixMs - UTC epoch milliseconds.
 * @returns `CN`, `T2` … `T7`.
 */
export function formatWeekday(unixMs: number): string {
  return vi.weekday[new Date(unixMs).getDay()] ?? "";
}

/**
 * Day and month in the phone's time zone.
 *
 * @param unixMs - UTC epoch milliseconds.
 * @returns `DD/MM`.
 */
export function formatDayMonth(unixMs: number): string {
  const d = new Date(unixMs);
  return `${pad2(d.getDate())}/${pad2(d.getMonth() + 1)}`;
}

/**
 * Weekday and time for timeline rows, or only the time when it is today.
 *
 * @param unixMs - UTC epoch milliseconds.
 * @param nowMs - Current time, UTC epoch milliseconds.
 * @returns For example `T4 · 21:10`, or `21:10` for today.
 */
export function formatWhen(unixMs: number, nowMs: number): string {
  const a = new Date(unixMs);
  const b = new Date(nowMs);
  const sameDay = a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
  if (sameDay) return formatTime(unixMs);
  if (nowMs - unixMs < 6 * 86_400_000) return `${formatWeekday(unixMs)} · ${formatTime(unixMs)}`;
  return `${formatDayMonth(unixMs)} · ${formatTime(unixMs)}`;
}

/**
 * Format an integer with Vietnamese digit grouping.
 *
 * @param value - Number to format; rounded to an integer.
 * @returns For example `4.812`.
 * @remarks
 * Written by hand instead of `Intl.NumberFormat` so the output is the same on
 * Hermes, browsers and Jest.
 */
export function formatInteger(value: number): string {
  const rounded = Math.round(value);
  const sign = rounded < 0 ? "-" : "";
  return sign + String(Math.abs(rounded)).replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}

/**
 * Start of the current local day.
 *
 * @param nowMs - Current time, UTC epoch milliseconds.
 * @returns Local midnight as UTC epoch milliseconds.
 */
export function startOfLocalDay(nowMs: number): number {
  const d = new Date(nowMs);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}
