import { ThemedText } from "@/components/themed-text";
import { ThemedView } from "@/components/themed-view";
import { useLocalSearchParams, useRouter } from "expo-router";
import { useEffect, useRef, useState } from "react";
import { SafeAreaView } from "react-native-safe-area-context";
import { StyleSheet, Text } from "react-native";
import {
  RTCSessionDescription,
  RTCPeerConnection,
  RTCView,
  MediaStream,
} from 'react-native-webrtc';
import type { RTCSessionDescriptionInit } from 'react-native-webrtc/lib/typescript/RTCSessionDescription';

type SignalingMessage = {
  type: 'offer' | 'answer' | 'candidate';
  sdp?: RTCSessionDescriptionInit;
  candidate?: RTCIceCandidateInit;
};

// function startWebRTC() {
//   let peerConstraints = {
//     iceServers: [
//       {
//         urls: 'stun:stun.l.google.com:19302'
//       }
//     ]
//   };
//   let pc = new RTCPeerConnection( peerConstraints );
//   pc.current.ontrack = (event) => {
//       if (event.streams && event.streams[0]) {
//         setRemoteStream(event.streams[0]);
//       }
//     };
//   pc.setRemoteDescription( new RTCSessionDescription( { type: 'offer', sdp: '' } ) );
// }

// function openWebSocket(address: string) {
//   const ws = new WebSocket(`ws://${address}:8080`);
//   ws.onopen = () => {
//     console.log('WebSocket connection opened');
//   };

//   ws.onmessage = (event) => {
//     console.log('Message from server:', event.data);
//   };
//   return ()=>ws.close();
// }

export default function ControlPage() {
  const router = useRouter();
  const { address } = useLocalSearchParams<{ address: string }>();
  const [remoteStream, setRemoteStream] = useState<MediaStream | null>(null);
  const pc = useRef<RTCPeerConnection | null>(null);
  const ws = useRef<WebSocket | null>(null);

  useEffect(() => {
    // 1. Initialize PeerConnection
    pc.current = new RTCPeerConnection({
      iceServers: [
        {
          urls: 'stun:stun.l.google.com:19302'
        }
      ]
    });
    console.log('PeerConnection initialized:', pc.current);

    const peerConnection = pc.current as any;
    peerConnection.addEventListener('connectionstatechange', (event: any) => {
      switch (pc.current?.connectionState) {
        case 'closed':
          router.back();
          break;
        case 'connected':
          console.log('Peer connection established');
          break;
      }
    });

    peerConnection.addEventListener('signalingstatechange', (event: any) => {
      console.log('Signaling state changed:', pc.current?.signalingState);
      switch (pc.current?.signalingState) {
        case 'closed':
          router.back();
          break;
      }
    });

    peerConnection.addEventListener('iceconnectionstatechange', (event: any) => {
      console.log('ICE connection state changed:', pc.current?.iceConnectionState);
      switch (pc.current?.iceConnectionState) {
        case 'closed':
          router.back();
          break;
      }
    });

    try {
      ws.current = new WebSocket(`ws://${address}:8080/ws`);
    } catch (error) {
      console.error('Error connecting to WebSocket:', error);
      router.back();
    }

    if (!ws.current) {
      console.error('WebSocket is not initialized');
      router.back();
      return;
    }

    ws.current.onmessage = async (message) => {
      const data = JSON.parse(message.data) as SignalingMessage;
      console.log('Received message from Go server:', data);

      // If we receive an Offer from the Go server
      if (data.type === 'offer' && pc.current && ws.current) {
        console.log('Received offer from Go server');
        if (data.sdp) {
          await pc.current.setRemoteDescription(new RTCSessionDescription(data.sdp));
          const awnser = await pc.current.createAnswer();
          await pc.current.setLocalDescription(awnser);

          ws.current.send(JSON.stringify({
            type: 'answer',
            sdp: pc.current.localDescription,
          }));
        }
      }
      else if (data.type === 'candidate' && data.candidate && pc.current) {
        await pc.current.addIceCandidate(data.candidate);
        ws.current?.send(JSON.stringify({
          type: 'candidate',
          candidate: data.candidate,
        }));
      }
    }
    ws.current.onclose = () => {
      console.log('WebSocket connection closed');
      router.back();
    }
    
    peerConnection.addEventListener('track', (event: any) => {
      if (event.streams && event.streams[0]) {
        setRemoteStream(event.streams[0]);
      }
    });
    /*
    const pendingRemoteCandidates: RTCIceCandidateInit[] = [];
    let remoteDescriptionApplied = false;

    peerConnection.addEventListener('connectionstatechange', (_: unknown) => {
      switch (peerConnection.connectionState) {
        case 'closed':
          router.back();

          break;
      };
    });

    // 2. Handle incoming media stream from Go Server
    peerConnection.addEventListener('track', (event: any) => {
      if (event.streams && event.streams[0]) {
        setRemoteStream(event.streams[0]);
      }
    });

    // 3. Connect to Go Signaling Server
    try {
      ws.current = new WebSocket(`ws://${address}:8080/ws`);
    } catch (error) {
      console.error('Error connecting to WebSocket:', error);
      router.back();
    }

    peerConnection.addEventListener('icecandidate', (event: any) => {
      if (!event.candidate || !ws.current || ws.current.readyState !== WebSocket.OPEN) {
        return;
      }

      ws.current.send(JSON.stringify({
        type: 'candidate',
        candidate: event.candidate,
      }));
    });

    if (!ws.current) {
      console.error('WebSocket is not initialized');
      router.back();
      return;
    }

    ws.current.onclose = () => {
      console.log('WebSocket connection closed');
      router.back();
    }
    ws.current.onmessage = async (message) => {
      const data = JSON.parse(message.data) as SignalingMessage;
      console.log('Received message from Go server:', data);

      // If we receive an Offer from the Go server
      if (data.type === 'offer' && pc.current && ws.current) {
        console.log('Received offer from Go server');
        if (data.sdp) {
          await pc.current.setRemoteDescription(new RTCSessionDescription(data.sdp as any));
          remoteDescriptionApplied = true;
        }

        while (pendingRemoteCandidates.length > 0) {
          const candidate = pendingRemoteCandidates.shift();
          if (candidate) {
            await pc.current.addIceCandidate(candidate);
          }
        }

        // Create an Answer and send it back to Go
        const answer = await pc.current.createAnswer();
        await pc.current.setLocalDescription(answer);

        if (pc.current.localDescription) {
          ws.current.send(JSON.stringify({
            type: 'answer',
            sdp: pc.current.localDescription,
          }));
        }
      }

      if (data.type === 'candidate' && data.candidate && pc.current) {
        if (!remoteDescriptionApplied) {
          pendingRemoteCandidates.push(data.candidate);
          return;
        }

        await pc.current.addIceCandidate(data.candidate);
      }
    };*/

    return () => {
      pc.current?.close();
      ws.current?.close();
    };
  }, []);

  console.log('Remote Stream:', remoteStream?.toURL());

  return (
    <SafeAreaView style={styles.container}>
      {remoteStream ? (
        <RTCView
          streamURL={remoteStream.toURL()}
          style={styles.video}
          objectFit="cover"
        />
      ): <Text>Connecting...</Text>}
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, width: '100%', height: '100%', backgroundColor: '#00f' },
  video: { flex: 1, width: '100%', height: '100%' },
});
