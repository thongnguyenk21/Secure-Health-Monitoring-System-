import type { ReactNode } from "react";
import { StyleSheet, Text, View } from "react-native";
import { Caption, Tile } from "../../../components/ui";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";

/** Props of {@link VitalTile}. */
export interface VitalTileProps {
  /** Upper-case caption, e.g. "Oxy máu". */
  caption: string;
  /** Main value; a number is drawn large, a word in bold body text. */
  value: string;
  /** Unit after the value, e.g. "%". */
  unit?: string;
  /** Draw the value as a word (activity) instead of a number. */
  word?: boolean;
  /** Extra content under the value. */
  children?: ReactNode;
  /** Test id for screen tests. */
  testID?: string;
}

/**
 * One tile of the 2 × 2 vitals grid on the home screen.
 *
 * @param props - See {@link VitalTileProps}.
 * @returns The tile.
 */
export function VitalTile({ caption, value, unit, word = false, children, testID }: VitalTileProps) {
  return (
    <Tile style={styles.tile}>
      <Caption>{caption}</Caption>
      <View style={styles.row} testID={testID}>
        <Text style={word ? styles.word : styles.num}>{value}</Text>
        {unit ? <Text style={styles.unit}>{` ${unit}`}</Text> : null}
      </View>
      {children}
    </Tile>
  );
}

const styles = StyleSheet.create({
  tile: { flex: 1, minHeight: 92, justifyContent: "space-between" },
  row: { flexDirection: "row", alignItems: "baseline", marginTop: 6 },
  num: { fontFamily: fonts.display, fontSize: 30, color: colors.ink, fontVariant: ["tabular-nums"] },
  word: { fontFamily: fonts.display, fontSize: 21, color: colors.ink, marginTop: 6 },
  unit: { fontFamily: fonts.body, fontSize: 16, color: colors.muted },
});
