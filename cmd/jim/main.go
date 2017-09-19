package main

import (
	"flag"
	"fmt"
	"log"

	"github.com/chzyer/readline"
	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/dsl/eval"
	"github.com/nathanielc/jim/smartmqtt"
	"github.com/nathanielc/smarthome"
)

var mqttURL = flag.String("mqtt", "tcp://localhost:1883", "URL of the MQTT broker")

func main() {
	flag.Parse()

	opts := smarthome.DefaultMQTTClientOptions()
	opts.AddBroker(*mqttURL)
	opts.SetCleanSession(true)
	server, err := smartmqtt.New(opts)
	if err != nil {
		log.Fatal(err)
	}

	rl, err := readline.NewEx(&readline.Config{
		Prompt:      "> ",
		HistoryFile: "/tmp/jim.history",
	})
	if err != nil {
		log.Fatal(err)
	}
	defer rl.Close()

	e := eval.New(server)
	for {
		line, err := rl.Readline()
		if err != nil {
			break
		}
		ast, err := dsl.Parse(line)
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
