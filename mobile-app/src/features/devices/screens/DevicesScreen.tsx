import { useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { StyleSheet, Text, View } from "react-native";
import { api } from "../../../api/client";
import type { Device } from "../../../api/types";
import { useNotice } from "../../../components/Notice";
import { Button, ErrorNote, Loading, NoDeviceCard, Screen, Tile } from "../../../components/ui";
import { vi } from "../../../i18n/vi";
import { usePrimaryDevice } from "../../../state/patient";
import { colors } from "../../../theme/colors";
import { fonts } from "../../../theme/fonts";
import { useNow } from "../../../utils/useNow";
import { resetEmergencyShown } from "../../alerts/emergency";
import { isFresh } from "../../home/healthStatus";
import { useLatest } from "../../home/queries";
import { ToggleRow } from "../components/Toggle";
import { consentRequestFor, latestConsentByScope, PRIVACY_TOGGLES, type PrivacyToggle } from "../consents";
import { useConsents, useRecordConsent } from "../queries";

function deviceStatus(device: Device): { label: string; color: string } {
  if (device.status === "active") return { label: vi.devices.paired, color: colors.ok };
  if (device.status === "pending") return { label: vi.devices.pending, color: colors.warn };
  return { label: vi.devices.revoked, color: colors.muted };
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <View style={styles.infoRow}>
      <Text style={styles.infoLabel}>{label}</Text>
      <Text style={styles.infoValue}>{value}</Text>
    </View>
  );
}

/**
 * Devices tab: the paired watch, privacy choices and family contacts.
 *
 * @returns The screen.
 */
export function DevicesScreen() {
  const { device, isLoading } = usePrimaryDevice();
  const latest = useLatest(device?.device_id ?? null);
  const consents = useConsents();
  const record = useRecordConsent();
  const notify = useNotice();
  const client = useQueryClient();
  const now = useNow(10_000);
  // Caregiver alerts have no backend scope or contact list yet (M1); the choice stays on this phone.
  const [notifyFamily, setNotifyFamily] = useState(false);

  const granted = latestConsentByScope(consents.data ?? []);

  const onToggle = (id: PrivacyToggle, next: boolean) => {
    const body = consentRequestFor(id, next);
    if (!body) {
      setNotifyFamily(next);
      return;
    }
    record.mutate(body, { onError: () => notify(vi.devices.consentSaveFailed, "warn") });
  };

  const onLogout = async () => {
    await api.logout();
    resetEmergencyShown();
    client.clear();
  };

  const fresh = isFresh(latest.data ?? null, now);

  return (
    <Screen title={vi.devices.title}>
      {isLoading ? (
        <Loading />
      ) : device ? (
        <Tile>
          <View style={styles.headRow}>
            <Text style={styles.cardTitle}>{vi.devices.watch}</Text>
            <View style={styles.status}>
              <View style={[styles.dot, { backgroundColor: deviceStatus(device).color }]} />
              <Text style={[styles.statusText, { color: deviceStatus(device).color }]}>{deviceStatus(device).label}</Text>
            </View>
          </View>
          <View style={styles.info}>
            <InfoRow label={vi.devices.battery} value={vi.common.dash} />
            <InfoRow label={vi.devices.version} value={device.fw_version ?? vi.common.dash} />
            <InfoRow label={vi.devices.unsent} value={fresh ? vi.devices.unsentNone : vi.devices.unsentUnknown} />
          </View>
        </Tile>
      ) : (
        <NoDeviceCard />
      )}
      <Tile>
        <Text style={styles.cardTitle}>{vi.devices.privacy}</Text>
        {consents.error ? <ErrorNote message={vi.common.loadError} onRetry={() => consents.refetch()} /> : null}
        <View style={styles.toggles}>
          {PRIVACY_TOGGLES.map((t) => (
            <ToggleRow
              key={t.id}
              label={t.label}
              value={t.scope ? granted[t.scope] === true : notifyFamily}
              onChange={(next) => onToggle(t.id, next)}
              disabled={t.scope !== null && (consents.isLoading || record.isPending)}
              testID={`toggle-${t.id}`}
            />
          ))}
        </View>
      </Tile>
      <Tile>
        <Text style={styles.cardTitle}>{vi.devices.family}</Text>
        <Text style={styles.muted}>{vi.devices.familyEmpty}</Text>
      </Tile>
      <View style={styles.logout}>
        <Button label={vi.devices.logout} onPress={onLogout} variant="secondary" testID="logout" />
      </View>
    </Screen>
  );
}

const styles = StyleSheet.create({
  headRow: { flexDirection: "row", justifyContent: "space-between", alignItems: "center" },
  cardTitle: { fontFamily: fonts.display, fontSize: 18, color: colors.ink },
  status: { flexDirection: "row", alignItems: "center", gap: 6 },
  dot: { width: 9, height: 9, borderRadius: 5 },
  statusText: { fontFamily: fonts.bodySemi, fontSize: 15 },
  info: { marginTop: 10, gap: 6 },
  infoRow: { flexDirection: "row" },
  infoLabel: { flex: 1, fontFamily: fonts.body, fontSize: 16, color: colors.muted },
  infoValue: { flex: 1, fontFamily: fonts.bodyBold, fontSize: 16, color: colors.ink },
  toggles: { marginTop: 4 },
  muted: { fontFamily: fonts.body, fontSize: 15.5, color: colors.muted, marginTop: 6, lineHeight: 21 },
  logout: { marginTop: 8 },
});
