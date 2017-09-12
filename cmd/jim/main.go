package main

import (
	"bufio"
	"flag"
	"fmt"
	"log"
	"os"

	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/dsl/eval"
	"github.com/nathanielc/jim/smartmqtt"
)

var mqttURL = flag.String("mqtt", "tcp://localhost:1883", "URL of the MQTT broker")

func main() {
	flag.Parse()

	server, err := smartmqtt.New(*mqttURL)
	if err != nil {
		log.Fatal(err)
	}

	scanner := bufio.NewScanner(os.Stdin)
	e := eval.New(server)
	for scanner.Scan() {
		ast, err := dsl.Parse(scanner.Text())
		r, err := e.Eval(ast)
		if err != nil {
			fmt.Println("E", err)
			continue
		}
		if r != nil {
			fmt.Println(r.String())
		}
	}
}
