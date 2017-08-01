package main

import (
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/davecheney/gpio"
	"github.com/nathanielc/smarthome"
)

const greenPinNumber = gpio.GPIO23
const redPinNumber = gpio.GPIO24

var greenPin gpio.Pin
var redPin gpio.Pin

func main() {

	// Setup GPIO pins
	pin, err := gpio.OpenPin(greenPinNumber, gpio.ModeOutput)
	if err != nil {
		log.Fatal(err)
	}
	greenPin = pin
	defer greenPin.Close()
	greenPin.Clear()

	pin, err = gpio.OpenPin(redPinNumber, gpio.ModeOutput)
	if err != nil {
		log.Fatal(err)
	}
	redPin = pin
	defer redPin.Close()
	redPin.Clear()

	// Connect to MQTT
	h := new(handler)
	opts := smarthome.DefaultMQTTClientOptions()
	opts.AddBroker("tcp://localhost:1883")
	opts.SetClientID("pidemo")
	s := smarthome.NewServer("rpi", h, opts)
	h.s = s
	if err := s.Connect(); err != nil {
		log.Fatal(err)
	}
	defer s.Disconnect()
	// Publish the state of the hardware connection.
	s.PublishHWStatus(smarthome.Connected)

	// Wait for signal
	c := make(chan os.Signal, 1)
	signal.Notify(c, os.Interrupt, syscall.SIGTERM)
	<-c
}

type handler struct {
	s smarthome.Server
}

func (h *handler) Set(toplevel, item string, value interface{}) {
	log.Println("set", toplevel, item, value)
	b := valueToBool(value)
	switch item {
	case "green":
		h.updatePin(item, greenPin, b)
	case "red":
		h.updatePin(item, redPin, b)
	}
}

func (h *handler) Get(toplevel, item string) (smarthome.Value, bool) {
	log.Println("get", toplevel, item)
	v := "off"
	switch item {
	case "green":
		if greenPin.Get() {
			v = "on"
		}
	case "red":
		if redPin.Get() {
			v = "on"
		}
	}
	return smarthome.Value{
		Value: v,
	}, true
}

func (h *handler) Command(toplevel string, cmd []byte) {
}

func (h *handler) updatePin(item string, pin gpio.Pin, new bool) {
	if pin.Get() != new {
		var str string
		if new {
			str = "on"
			pin.Set()
		} else {
			str = "off"
			pin.Clear()
		}
		h.s.PublishStatus(item, smarthome.Value{
			Value: str,
		})
	}
}

func valueToBool(v interface{}) bool {
	str, ok := v.(string)
	if !ok {
		return false
	}
	return str == "on"
}
