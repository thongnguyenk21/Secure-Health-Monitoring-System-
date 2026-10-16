import { vi } from "../i18n/vi";

const timeFmt = new Intl.DateTimeFormat("vi-VN", { hour: "2-digit", minute: "2-digit" });
const dateTimeFmt = new Intl.DateTimeFormat("vi-VN", {
  day: "2-digit",
  month: "2-digit",
  hour: "2-digit",
  minute: "2-digit",
});
const dateFmt = new Intl.DateTimeFormat("vi-VN", { day: "2-digit", month: "2-digit", year: "numeric" });
const weekdayFmt = new Intl.DateTimeFormat("vi-VN", { weekday: "short" });

/**
 * Format an epoch time as local `HH:mm`.
 *
 * @param unixMs - UTC epoch ms.
 * @returns The local time of day.
 */
export function formatTime(unixMs: number): string {
  return timeFmt.format(unixMs);
}

/**
 * Format an epoch time as local `dd/MM HH:mm`.
 *
 * @param unixMs - UTC epoch ms.
 * @returns The local day and time.
 */
export function formatDateTime(unixMs: number): string {
  return dateTimeFmt.format(unixMs);
}

/**
 * Format an epoch time as a local date.
 *
 * @param unixMs - UTC epoch ms.
 * @returns The local date.
 */
export function formatDate(unixMs: number): string {
  return dateFmt.format(unixMs);
}

/**
 * Format an epoch time as a short local weekday (for example `T4` or `CN`).
 *
 * @param unixMs - UTC epoch ms.
 * @returns The weekday.
 * @remarks
 * Intl gives "Thứ 4" / "Chủ Nhật"; clinical notes use the shorter T4 / CN.
 */
export function formatWeekday(unixMs: number): string {
  const text = weekdayFmt.format(unixMs);
  if (/^ch/i.test(text)) return vi.common.sunday;
  return text.replace(/^th(ứ)?\s*/i, vi.common.weekdayPrefix);
}

/**
 * Format an optional number, or a dash when it is missing.
 *
 * @param value - The number.
 * @param suffix - Unit text appended to a present value.
 * @returns The text to show.
 */
export function formatOptional(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) return vi.common.dash;
  return `${Math.round(value)}${suffix}`;
}

/**
 * Shorten a pseudonymous device id to its first four characters.
 *
 * @param deviceId - 32-hex device id.
 * @returns For example `a1f3…`.
 */
export function shortDeviceId(deviceId: string): string {
  return `${deviceId.slice(0, 4)}${vi.common.ellipsis}`;
}

/**
 * Tell whether a string is a valid pseudonymous device id.
 *
 * @param value - Candidate id.
 * @returns True for exactly 32 lowercase hex characters.
 */
export function isDeviceId(value: string): boolean {
  return /^[0-9a-f]{32}$/.test(value);
}
