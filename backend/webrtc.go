package main

import (
	"bufio"
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os/exec"
	"time"

	"github.com/go-gst/go-gst/gst"
	"github.com/go-gst/go-gst/gst/app"
	"github.com/gorilla/websocket"
	"github.com/pion/rtp"
	"github.com/pion/webrtc/v3"
	"github.com/pion/webrtc/v3/pkg/media"
	"github.com/pion/webrtc/v3/pkg/media/h264reader"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool { return true },
}

type signalingMessage struct {
	Type      string                     `json:"type"`
	SDP       *webrtc.SessionDescription `json:"sdp,omitempty"`
	Candidate *webrtc.ICECandidateInit   `json:"candidate,omitempty"`
}

// Global track that we will stream to all connected clients
var videoTrack *webrtc.TrackLocalStaticSample

// JSON encode + base64 a SessionDescription.
func encode(obj *webrtc.SessionDescription) string {
	b, err := json.Marshal(obj)
	if err != nil {
		panic(err)
	}

	return base64.StdEncoding.EncodeToString(b)
}

func signalHandler(ctx context.Context, sdp *webrtc.SessionDescription, ws *websocket.Conn, peerConnection *webrtc.PeerConnection) {
	ws.WriteJSON(signalingMessage{Type: "offer", SDP: sdp})

	for {
		select {
		case <-ctx.Done():
			log.Println("Context cancelled, closing WebSocket and WebRTC connection")
			return
		default:
			_, message, err := ws.ReadMessage()
			if err != nil {
				log.Println("WebSocket error:", err)
				return
			}
			var signal signalingMessage
			if err := json.Unmarshal(message, &signal); err != nil {
				log.Println("Failed to decode signaling message:", err)
				continue
			}
			if signal.Type == "answer" && signal.SDP != nil {
				err = peerConnection.SetRemoteDescription(*signal.SDP)
				if err != nil {
					log.Println("Failed to set remote description:", err)
					return
				}
			}

		}
	}
}

func webRTCServer(ctx context.Context) {
	videoSrc := "v4l2src device=/dev/video0"
	gst.Init(nil)

	// Create a new RTCPeerConnection
	peerConnection, err := webrtc.NewPeerConnection(webrtc.Configuration{
		ICEServers: []webrtc.ICEServer{
			{
				URLs: []string{"stun:stun.l.google.com:19302"},
			},
		},
	})
	if err != nil {
		panic(err)
	}

	// Set the handler for ICE connection state
	// This will notify you when the peer has connected/disconnected
	peerConnection.OnICEConnectionStateChange(func(connectionState webrtc.ICEConnectionState) {
		fmt.Printf("Connection State has changed %s \n", connectionState.String())
	})

	// Create a video track
	vp8Track, err := webrtc.NewTrackLocalStaticSample(webrtc.RTPCodecCapability{MimeType: "video/vp8"}, "video", "pion2")
	if err != nil {
		panic(err)
	} else if _, err = peerConnection.AddTrack(vp8Track); err != nil {
		panic(err)
	}

	// Create an offer to send to the browser
	offer, err := peerConnection.CreateOffer(nil)
	if err != nil {
		panic(err)
	}

	// Create channel that is blocked until ICE Gathering is complete
	gatherComplete := webrtc.GatheringCompletePromise(peerConnection)

	// Sets the LocalDescription, and starts our UDP listeners
	if err = peerConnection.SetLocalDescription(offer); err != nil {
		panic(err)
	}

	// Block until ICE Gathering is complete, disabling trickle ICE
	// we do this because we only can exchange one signaling message
	// in a production application you should exchange ICE Candidates via OnICECandidate
	<-gatherComplete

	pipelineForCodec("vp8", []*webrtc.TrackLocalStaticSample{vp8Track}, videoSrc)

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		ws, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			log.Println("Upgrade error:", err)
			return
		}

		go signalHandler(ctx, peerConnection.LocalDescription(), ws, peerConnection)

		select {
		case <-ctx.Done():
			log.Println("Context cancelled, closing WebSocket and WebRTC connection")
			ws.Close()
			return
		}
	})
	log.Println("Server started on :8080")
	log.Fatal(http.ListenAndServe(":8080", nil))
}

func handleSignaling(ctx context.Context, w http.ResponseWriter, r *http.Request) {
	ws, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println("Upgrade error:", err)
		return
	}
	defer ws.Close()

	configuration := webrtc.Configuration{ // No ICE servers for local only
		ICEServers: []webrtc.ICEServer{
			{
				URLs: []string{"stun:stun.l.google.com:19302"},
			},
		},
	}

	peerConnection, err := webrtc.NewPeerConnection(configuration)
	if err != nil {
		log.Println("Failed to create peer connection:", err)
		return
	}
	dc, err := peerConnection.CreateDataChannel("data", nil)
	if err != nil {
		log.Println("Failed to create data channel:", err)
		return
	}

	dc.OnOpen(func() {
		log.Println("Data channel opened")
	})

	// We use TrackLocalStaticRTP because GStreamer handles the RTP packetization for us
	videoTrack, err := webrtc.NewTrackLocalStaticRTP(
		webrtc.RTPCodecCapability{MimeType: webrtc.MimeTypeVP8},
		"video",
		"pion-stream",
	)
	if err != nil {
		log.Fatalf("Failed to create video track: %v", err)
	}

	_, err = peerConnection.AddTrack(videoTrack)
	if err != nil {
		log.Fatalf("Failed to add track to PeerConnection: %v", err)
	}

	// 3. Define the Linux GStreamer hardware capture & encoding pipeline.
	// We use appsink at the end to pull the raw RTP packets directly into Go memory.
	pipelineStr := "v4l2src device=/dev/video0 ! " +
		"video/x-raw,width=640,height=480,framerate=30/1 ! " +
		"videoconvert ! " +
		"vp8enc error-resilient=partitions keyframe-max-dist=10 auto-alt-ref=true cpu-used=5 deadline=1 threads=4 ! " +
		"rtpvp8pay ! " +
		"appsink name=sink"

	pipeline, err := gst.NewPipelineFromString(pipelineStr)
	if err != nil {
		log.Fatalf("Failed to parse pipeline string: %v", err)
	}

	// 4. Extract the 'appsink' element from the pipeline to listen for data
	sinkEl, err := pipeline.GetElementByName("sink")
	if err != nil {
		log.Fatalf("Failed to find appsink element: %v", err)
	}
	appSink := app.SinkFromElement(sinkEl)

	// 5. Set up callbacks to receive RTP packets from GStreamer and forward to Pion
	appSink.SetCallbacks(&app.SinkCallbacks{
		NewSampleFunc: func(sink *app.Sink) gst.FlowReturn {
			// Pull the next packet buffer from the pipeline
			sample := sink.PullSample()
			if sample == nil {
				return gst.FlowEOS
			}
			buffer := sample.GetBuffer()
			if buffer == nil {
				return gst.FlowOK
			}

			// Map the buffer memory so Go can read it safely
			mapInfo := buffer.Map(gst.MapRead)
			defer buffer.Unmap()

			// Create a Pion RTP packet from the raw byte stream coming out of rtpvp8pay
			rtpPacket := &rtp.Packet{}
			if err := rtpPacket.Unmarshal(mapInfo.Bytes()); err != nil {
				log.Printf("Failed to unmarshal RTP packet: %v", err)
				return gst.FlowOK
			}

			// Forward the RTP packet over our WebRTC connection
			if err := videoTrack.WriteRTP(rtpPacket); err != nil {
				log.Printf("Failed to write RTP packet to track: %v", err)
			}

			return gst.FlowOK
		},
	})

	// 6. Start the camera capture pipeline
	err = pipeline.SetState(gst.StatePlaying)
	if err != nil {
		log.Fatalf("Failed to set pipeline state to PLAYING: %v", err)
	}

	log.Println("Camera pipeline running. WebRTC track accepting packets...")

	offer, err := peerConnection.CreateOffer(nil)
	if err != nil {
		log.Println("Failed to create offer:", err)
		return
	}

	err = peerConnection.SetLocalDescription(offer)
	if err != nil {
		log.Println("Failed to set local description:", err)
		return
	}

	peerConnection.OnICEConnectionStateChange(func(connectionState webrtc.ICEConnectionState) {
		log.Printf("Connection State has changed %s \n", connectionState.String())
	})

	peerConnection.OnConnectionStateChange(func(state webrtc.PeerConnectionState) {
		log.Println("Peer Connection State has changed:", state.String())
		if state == webrtc.PeerConnectionStateFailed {
			log.Println("Peer Connection has gone to failed exiting")
			pipeline.SetState(gst.StateNull)
			peerConnection.Close()
		}
	})

	peerConnection.OnSignalingStateChange(func(state webrtc.SignalingState) {
		log.Println("Signaling State has changed:", state.String())
	})

	finalOffer := peerConnection.LocalDescription()

	err = ws.WriteJSON(signalingMessage{Type: "offer", SDP: finalOffer})
	log.Println("Sent offer to client:", finalOffer)

	peerConnection.OnICECandidate(func(c *webrtc.ICECandidate) {
		if c == nil {
			return // Gathering finished
		}
		candidateInit := c.ToJSON()
		err = ws.WriteJSON(signalingMessage{Type: "candidate", Candidate: &candidateInit})
	})

	for {
		_, message, err := ws.ReadMessage()
		if err != nil {
			log.Println("WebSocket error:", err)
			return
		}

		var signal signalingMessage
		if err := json.Unmarshal(message, &signal); err != nil {
			log.Println("Failed to decode signaling message:", err)
			continue
		}

		if signal.Type == "answer" && signal.SDP != nil {
			err = peerConnection.SetRemoteDescription(*signal.SDP)
			if err != nil {
				log.Println("Failed to set remote description:", err)
				return
			}
		} else if signal.Type == "candidate" && signal.Candidate != nil {
			err = peerConnection.AddICECandidate(*signal.Candidate)
			if err != nil {
				log.Println("Failed to add ICE candidate:", err)
				return
			}
		}
	}
}

// Create the appropriate GStreamer pipeline depending on what codec we are working with.
func pipelineForCodec(codecName string, tracks []*webrtc.TrackLocalStaticSample, pipelineSrc string) { // nolint
	pipelineStr := "appsink name=appsink"
	switch codecName {
	case "vp8":
		pipelineStr = pipelineSrc + " ! vp8enc error-resilient=partitions keyframe-max-dist=10 auto-alt-ref=true cpu-used=5 deadline=1 ! " + pipelineStr // nolint
	case "vp9":
		pipelineStr = pipelineSrc + " ! vp9enc ! " + pipelineStr
	case "h264":
		pipelineStr = pipelineSrc + " ! video/x-raw,format=I420 ! x264enc speed-preset=ultrafast tune=zerolatency key-int-max=20 ! video/x-h264,stream-format=byte-stream ! " + pipelineStr // nolint
	case "opus":
		pipelineStr = pipelineSrc + " ! opusenc ! " + pipelineStr
	case "pcmu":
		pipelineStr = pipelineSrc + " ! audio/x-raw, rate=8000 ! mulawenc ! " + pipelineStr
	case "pcma":
		pipelineStr = pipelineSrc + " ! audio/x-raw, rate=8000 ! alawenc ! " + pipelineStr
	default:
		panic("Unhandled codec " + codecName) //nolint
	}

	pipeline, err := gst.NewPipelineFromString(pipelineStr)
	if err != nil {
		panic(err)
	}

	if err = pipeline.SetState(gst.StatePlaying); err != nil {
		panic(err)
	}

	appSink, err := pipeline.GetElementByName("appsink")
	if err != nil {
		panic(err)
	}

	app.SinkFromElement(appSink).SetCallbacks(&app.SinkCallbacks{
		NewSampleFunc: func(sink *app.Sink) gst.FlowReturn {
			sample := sink.PullSample()
			if sample == nil {
				return gst.FlowEOS
			}

			buffer := sample.GetBuffer()
			if buffer == nil {
				return gst.FlowError
			}

			samples := buffer.Map(gst.MapRead).Bytes()
			defer buffer.Unmap()

			for _, t := range tracks {
				if err := t.WriteSample(media.Sample{Data: samples, Duration: *buffer.Duration().AsDuration()}); err != nil {
					panic(err) //nolint
				}
			}

			return gst.FlowOK
		},
	})
}
