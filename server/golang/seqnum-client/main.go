package main

import (
	"crypto/tls"
	"crypto/x509"
	"encoding/binary"
	"fmt"
	"lib/entity/reqwest"
	"os"
	"sync"
	"time"
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

	cd := sync.WaitGroup{}
	m := 10
	n := 5
	cd.Add(m)
	t := time.Now()
	for i := 1; i <= m; i += 1 {
		index := i
		go func() {
			// Dial the TCP server using TLS
			conn, err := tls.Dial("tcp", "localhost:11152", tlsConfig) // Replace with your desired host and port
			if err != nil {
				fmt.Println("Error connecting:", err)
				return
			}
			defer conn.Close()
			for j := 1; j <= n; j += 1 {

				// Create a byte slice of length 16
				data := make([]byte, 16, 16)

				// Convert and write the numbers to the byte slice using big-endian encoding
				binary.BigEndian.PutUint64(data[0:8], uint64(index))
				binary.BigEndian.PutUint64(data[8:16], uint64(j))

				req := reqwest.WithResourceIdPayload(reqwest.Seqnum, data)
				req.SetReqId(uint64(j))
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
				binary.BigEndian.Uint64(resp[12:20])
				// fmt.Printf("%d seqnum: %d\n", index, val)
			}
			cd.Done()
		}()
	}
	cd.Wait()
	fmt.Println(time.Since(t).Microseconds() / int64(m*n))
}
