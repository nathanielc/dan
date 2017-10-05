package eval

import (
	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/nathanielc/smarthome"
)

type Config struct {
	TopLevel   string
	Latitude   float64
	Longitude  float64
	MQTT       *mqtt.ClientOptions
	ClientOnly bool
}

func DefaultConfig() Config {
	return Config{
		TopLevel: "jim",
		MQTT:     smarthome.DefaultMQTTClientOptions(),
	}
}
