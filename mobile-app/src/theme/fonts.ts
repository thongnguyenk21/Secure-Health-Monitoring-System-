import { BeVietnamPro_600SemiBold } from "@expo-google-fonts/be-vietnam-pro/600SemiBold";
import { BeVietnamPro_700Bold } from "@expo-google-fonts/be-vietnam-pro/700Bold";
import { IBMPlexMono_500Medium } from "@expo-google-fonts/ibm-plex-mono/500Medium";
import { SourceSans3_400Regular } from "@expo-google-fonts/source-sans-3/400Regular";
import { SourceSans3_600SemiBold } from "@expo-google-fonts/source-sans-3/600SemiBold";
import { SourceSans3_700Bold } from "@expo-google-fonts/source-sans-3/700Bold";

/**
 * Font files loaded at start-up with `useFonts`.
 *
 * @remarks
 * Each weight is imported from its own subpath so only these six files are bundled.
 *
 * Be Vietnam Pro for headings and numbers, Source Sans 3 for body text and
 * IBM Plex Mono for small captions, as in the report wireframe.
 */
export const fontAssets = {
  BeVietnamPro_600SemiBold,
  BeVietnamPro_700Bold,
  SourceSans3_400Regular,
  SourceSans3_600SemiBold,
  SourceSans3_700Bold,
  IBMPlexMono_500Medium,
};

/** Font family names to use in styles; each name is one weight. */
export const fonts = {
  /** Headings, semibold. */
  displaySemi: "BeVietnamPro_600SemiBold",
  /** Headings and big numbers, bold. */
  display: "BeVietnamPro_700Bold",
  /** Body text. */
  body: "SourceSans3_400Regular",
  /** Body text, semibold. */
  bodySemi: "SourceSans3_600SemiBold",
  /** Body text, bold. */
  bodyBold: "SourceSans3_700Bold",
  /** Upper-case captions of tiles. */
  mono: "IBMPlexMono_500Medium",
} as const;
