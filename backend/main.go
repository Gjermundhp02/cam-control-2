package main

import (
	"context"
	"encoding/json"
	"fmt"
	"net"
	"os"
	"os/signal"
	"syscall"
	"time"
)

const (
	discoveryPort = 9999
	announceEvery = 3 * time.Second
	announceText  = "RAT-CTRL"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	conn, err := net.ListenUDP("udp4", &net.UDPAddr{Port: discoveryPort})
	if err != nil {
		fmt.Fprintf(os.Stderr, "listen udp: %v\n", err)
		os.Exit(1)
	}
	defer conn.Close()

	go announceLoop(ctx, conn)
	go webRTCServer(ctx)
	<-ctx.Done()
}

type discoveryMessage struct {
	Key    string `json:"key"`
	Name   string `json:"name"`
	Status string `json:"status"`
}

func announceLoop(ctx context.Context, conn *net.UDPConn) {
	ticker := time.NewTicker(announceEvery)
	defer ticker.Stop()

	if err := announce(conn); err != nil {
		fmt.Fprintf(os.Stderr, "initial announce: %v\n", err)
	}

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			if err := announce(conn); err != nil {
				fmt.Fprintf(os.Stderr, "announce: %v\n", err)
			}
		}
	}
}

func announce(conn *net.UDPConn) error {
	message, err := json.Marshal(discoveryMessage{
		Key:    "RAT-CTRL",
		Name:   "Larry",
		Status: "online",
	})
	if err != nil {
		return fmt.Errorf("marshal discovery message: %w", err)
	}
	addr := &net.UDPAddr{IP: net.IPv4bcast, Port: discoveryPort}
	_, err = conn.WriteToUDP(message, addr)
	return err
}
