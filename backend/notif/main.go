package main

import (
	"fmt"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	if len(os.Args) == 2 && os.Args[1] == "--check-smtp" {
		if err := checkSMTP(); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		return
	}

	url := envOrDefault("NATS_URL", "nats://localhost:4222")

	nc, err := StartNATS(url)
	if err != nil {
		panic(err)
	}
	defer nc.Close()

	fmt.Printf("notif subscribed to NATS: %s\n", url)

	sig := make(chan os.Signal, 1)
	signal.Notify(sig, syscall.SIGINT, syscall.SIGTERM)
	<-sig
}
