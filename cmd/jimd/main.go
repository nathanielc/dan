package main

import (
	"flag"
	"io/ioutil"
	"log"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/nathanielc/jim/dsl"
	"github.com/nathanielc/jim/dsl/eval"
	"github.com/pkg/errors"
)

var dir = flag.String("dir", "jim.d", "Directory containing the jim scripts")
var mqttURL = flag.String("mqtt", "tcp://localhost:1883", "URL of the MQTT broker")
var clientID = flag.String("client-id", "jimd", "Unique ID for this MQTT client")
var lat = flag.Float64("lat", 0, "Latitude, used for sun relative times")
var lon = flag.Float64("lon", 0, "Longitude, used for sun relative times")

func main() {
	flag.Parse()

	scripts, err := loadScripts(*dir)
	if err != nil {
		log.Fatal(err)
	}

	conf := eval.DefaultConfig()
	conf.MQTT.AddBroker(*mqttURL)
	conf.MQTT.SetClientID(*clientID)
	conf.Latitude = *lat
	conf.Longitude = *lon
	e, err := eval.New(conf)
	if err != nil {
		log.Fatal(err)
	}
	for _, script := range scripts {
		ast, err := dsl.Parse(script)
		if err != nil {
			log.Fatal(err)
		}
		_, err = e.Eval(ast)
		if err != nil {
			log.Fatal(err)
		}
	}

	log.Println("Started...")

	// Wait for signal to stop
	signalC := make(chan os.Signal, 1)
	signal.Notify(signalC, os.Interrupt, syscall.SIGTERM)
	<-signalC
	log.Println("Stopping...")
}

func loadScripts(dir string) ([]string, error) {
	files, err := ioutil.ReadDir(dir)
	if err != nil {
		return nil, errors.Wrapf(err, "reading dir %s", dir)
	}
	scripts := make([]string, 0, len(files))
	for _, fi := range files {
		if fi.IsDir() || !strings.HasSuffix(fi.Name(), ".jim") {
			continue
		}
		f, err := os.Open(filepath.Join(dir, fi.Name()))
		if err != nil {
			return nil, errors.Wrapf(err, "opening file %s", fi.Name())
		}
		data, err := ioutil.ReadAll(f)
		if err != nil {
			return nil, errors.Wrapf(err, "reading file %s", fi.Name())
		}
		scripts = append(scripts, string(data))
	}
	return scripts, nil
}
