package main

import (
	"crypto/tls"
	"crypto/x509"
	"encoding/binary"
	"fmt"
	"lib/entity"
	"os"
)

func main() {

	// Load the TLS certificate and key files
	cert, err := tls.LoadX509KeyPair("/Users/slma/RustProjects/prim/server/cert/localhost-client.crt", "/Users/slma/RustProjects/prim/server/cert/localhost-client.key") // Replace with your certificate and key file paths
	if err != nil {
		fmt.Println("Error loading certificate:", err)
		return
	}

	// Load the CA certificate file
	caCert, err := os.ReadFile("/Users/slma/RustProjects/prim/server/cert/PrimRootCA.crt") // Replace with your CA certificate file path
	if err != nil {
		fmt.Println("Error loading CA certificate:", err)
		return
	}
	caCertPool := x509.NewCertPool()
	caCertPool.AppendCertsFromPEM(caCert)

	// Configure the TLS connection
	tlsConfig := &tls.Config{
		Certificates: []tls.Certificate{cert},
		RootCAs:      caCertPool,
	}

	// Dial the TCP server using TLS
	conn, err := tls.Dial("tcp", "localhost:11152", tlsConfig) // Replace with your desired host and port
	if err != nil {
		fmt.Println("Error connecting:", err)
		return
	}
	defer conn.Close()
	for i := 1; i <= 20; i += 1 {
		// time.Sleep(time.Millisecond * 500)
		// Read three uint64 numbers as input
		//var num1, num2 uint64
		//fmt.Print("Enter user1: ")
		//fmt.Scan(&num1)
		//fmt.Print("Enter user2: ")
		//fmt.Scan(&num2)

		// Create a byte slice of length 16
		data := make([]byte, 16, 16)

		// Convert and write the numbers to the byte slice using big-endian encoding
		binary.BigEndian.PutUint64(data[0:8], 1)
		binary.BigEndian.PutUint64(data[8:16], 2)

		req := entity.WithResourceIdPayload(entity.Seqnum, data)
		req.SetReqId(uint64(i))
		n := 0
		bytes := req.AsSlice()
		for n < len(bytes) {
			size, err := conn.Write(bytes[n:])
			if err != nil {
				fmt.Println("Error sending data:", err)
				return
			}
			n += size
		}

		resp := make([]byte, 20, 20)
		n = 0
		for n < len(resp) {
			size, err := conn.Read(resp[n:])
			if err != nil {
				fmt.Println(err)
				return
			}
			n += size
		}
		val := binary.BigEndian.Uint64(resp[12:20])
		fmt.Printf("Seqnum: %d\n", val)
	}
}
