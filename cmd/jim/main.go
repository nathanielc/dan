package main

import (
	"flag"
	"fmt"
	"log"
	"strings"

	"github.com/chzyer/readline"
	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/dsl/eval"
)

var mqttURL = flag.String("mqtt", "tcp://localhost:1883", "URL of the MQTT broker")
var lat = flag.Float64("lat", 0, "Latitude, used for sun relative times")
var lon = flag.Float64("lon", 0, "Longitude, used for sun relative times")

func main() {
	flag.Parse()

	rl, err := readline.NewEx(&readline.Config{
		Prompt:      "> ",
		HistoryFile: "/tmp/jim.history",
	})
	if err != nil {
		log.Fatal(err)
	}
	defer rl.Close()

	conf := eval.DefaultConfig()
	conf.ClientOnly = true
	conf.MQTT.AddBroker(*mqttURL)
	conf.MQTT.SetCleanSession(true)
	conf.Latitude = *lat
	conf.Longitude = *lon
	e, err := eval.New(conf)
	if err != nil {
		log.Fatal(err)
	}
	for {
		line, err := rl.Readline()
		if err != nil {
			break
		}
		if strings.TrimSpace(line) == "upcoming" {
			events := e.Upcoming()
			if len(events) > 0 {
				fmt.Println("Time\t\t\t\tDescription")
				for _, e := range events {
					fmt.Printf("%s\t%s\n", e.Time, e.Description)
				}
			}
			continue
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
