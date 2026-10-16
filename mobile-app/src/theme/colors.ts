/**
 * Colour tokens of the patient app.
 *
 * @remarks
 * Same values as the design tokens of the report wireframe
 * (`docs/reports/system-design-slides.html`, `:root`), so the app and the
 * slides stay one visual system.
 */
export const colors = {
  /** Screen background. */
  bg: "#F6F8F9",
  /** Card and tile background. */
  paper: "#FFFFFF",
  /** Main text. */
  ink: "#12212B",
  /** Secondary text. */
  muted: "#52636F",
  /** Borders and grid lines. */
  line: "#CBD5DB",
  /** Brand colour: active tab, chart line, primary button. */
  accent: "#0B6E7F",
  /** Soft brand background. */
  accentBg: "#D8EEF1",
  /** Urgent state (critical alert). */
  alert: "#C7382B",
  /** Dark red of the "need help" button on the alert screen. */
  alertDeep: "#7E1F17",
  /** Soft red background. */
  alertBg: "#FBE4E1",
  /** Attention state (warning alert). */
  warn: "#B7791F",
  /** Soft amber background. */
  warnBg: "#FBF0DC",
  /** Normal state. */
  ok: "#2E7D4F",
  /** Soft green background. */
  okBg: "#DFF1E6",
  /** Backdrop around the phone-width column on wide web screens. */
  backdrop: "#E6ECEF",
} as const;

/** Name of one colour token. */
export type ColorToken = keyof typeof colors;
