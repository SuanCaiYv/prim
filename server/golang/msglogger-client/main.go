package main

import (
	"fmt"
	"lib/entity/msg"
	"net"
	"os"
)

func main() {
	socketPath := "/tmp/msglogger-1.sock"

	// Connect to the Unix domain socket
	conn, err := net.Dial("unix", socketPath)
	if err != nil {
		fmt.Println("Error connecting to Unix domain socket:", err)
		os.Exit(1)
	}
	defer conn.Close()
	req := msg.TextMsg(1, 2, 3, "aaa")
	fmt.Println(req.AsBytes())

	// Send the message
	_, err = conn.Write(req.AsBytes())
	if err != nil {
		fmt.Println("Error sending message:", err)
		os.Exit(1)
	}
	resp := make([]byte, 10, 10)
	n, err := conn.Read(resp)
	if err != nil {
		fmt.Println("Error reading response:", err)
		os.Exit(1)
	}
	fmt.Println("Response:", string(resp[:n]))

	fmt.Println("Message sent successfully!")
}
