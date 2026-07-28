import * as Device from 'expo-device';
import { Platform, StyleSheet, View, Text, TouchableNativeFeedback } from 'react-native';
import { SafeAreaView } from 'react-native-safe-area-context';

import { ThemedText } from '@/components/themed-text';
import { ThemedView } from '@/components/themed-view';

import { BottomTabInset, MaxContentWidth, Spacing } from '@/constants/theme';
import dgram from 'react-native-udp'
import { useEffect, useState } from 'react';
import { useRouter } from 'expo-router';


enum Status {
  Online = 'online',
  Offline = 'offline',
  Busy = 'busy',
}

type Turret = {
  name: string;
  status: Status;
  address: string;
  lastSeen: Date;
}

type setTurretsCallback = (turrets: Turret[]) => Turret[];

function getList(setTurrets: (callback: setTurretsCallback) => void) {
  const socket: any = dgram.createSocket({ type: 'udp4' })
  socket.bind(9999)

  socket.on('message', function (data: Uint8Array, rinfo: { address: string }) {
    const parsedData = JSON.parse(String.fromCharCode(...Array.from(data)))

    if (parsedData.key !== 'RAT-CTRL') {
      console.log('Invalid key, ignoring message');
      return;
    }

    setTurrets(prev => prev.find(turret => turret.name === parsedData.name) ? prev.map(turret => turret.name === parsedData.name ? { ...turret, ...parsedData, lastSeen: new Date() } : turret) : [...prev, {
      name: parsedData.name,
      status: parsedData.status,
      address: rinfo.address,
      lastSeen: new Date()
    }]);
  })

  return () => {
    socket.removeAllListeners('message');
    socket.close();
  };
}

export default function HomeScreen() {
  const [turrets, setTurrets] = useState<Turret[]>([]);
  const router = useRouter();
  useEffect(() => {
    const cleanup = getList(setTurrets)

    return cleanup;
  }, [setTurrets]);
  return (
    <ThemedView style={{flex: 1}}>
      <SafeAreaView style={styles.safeArea}>
        <ThemedText type="title" style={styles.title}>Available Turrets</ThemedText>
        {turrets.map((turret, index) => (
          <TouchableNativeFeedback key={index}
          onPress={() => {
            console.log('Navigating to turret', turret.address);
            router.navigate('/test');}}>

          <ThemedView style={styles.listItem}>
            <ThemedText style={styles.title}>{turret.name}</ThemedText>
            <View style={{ flexDirection: 'row', gap: Spacing.two }}>
              <ThemedText type="small">{turret.lastSeen.toLocaleString()}</ThemedText>
              <ThemedText type="small">{turret.address}</ThemedText>
              
            </View>
            
          </ThemedView>
          </TouchableNativeFeedback>
        ))}
        <Text>too</Text>
      </SafeAreaView>
    </ThemedView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    flexDirection: 'row',
  },
  safeArea: {
    flex: 1,
    alignItems: 'center',
    gap: Spacing.three,
    paddingBottom: BottomTabInset + Spacing.three,
    maxWidth: MaxContentWidth,
  },
  heroSection: {
    alignItems: 'center',
    justifyContent: 'center',
    flex: 1,
    paddingHorizontal: Spacing.four,
    gap: Spacing.four,
  },
  title: {
    textAlign: 'center',
  },
  code: {
    textTransform: 'uppercase',
  },
  stepContainer: {
    gap: Spacing.three,
    alignSelf: 'stretch',
    paddingHorizontal: Spacing.three,
    paddingVertical: Spacing.four,
    borderRadius: Spacing.four,
  },
  listItem: {
    alignItems: 'flex-start',
    width: '100%',
    borderBottomWidth: 1,
    borderBottomColor: 'rgba(255, 255, 255, 0.1)',
    paddingVertical: Spacing.three,
    paddingHorizontal: Spacing.three,
    gap: Spacing.two,
  },
  listItemInfo: {
    flexDirection: 'row',
  }
});
