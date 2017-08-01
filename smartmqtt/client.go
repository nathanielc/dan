package smartmqtt

import (
	"sync"

	"github.com/nathanielc/smarthome"
)

type Value smarthome.Value

type Client interface {
	Set(toplevel, device, value string) error
	Get(toplevel, device string) (Value, error)
	When(toplevel, device, value string, callback func()) error
}

type client struct {
	c smarthome.Client

	wg sync.WaitGroup
}

func New() (Client, error) {
	opts := smarthome.DefaultMQTTClientOptions()
	opts.AddBroker("tcp://localhost:1883")
	opts.SetClientID("jim-smartmqtt")
	c, err := smarthome.NewClient(opts)
	if err != nil {
		return nil, err
	}
	return &client{
		c: c,
	}, nil
}

func (c *client) Set(toplevel, device, value string) error {
	return c.c.Set(toplevel, device, value)
}

func (c *client) Get(toplevel, device string) (Value, error) {
	v, err := c.c.Get(toplevel, device)
	if err != nil {
		return Value{}, err
	}
	return Value(v), nil
}

func (c *client) When(toplevel, device, value string, callback func()) error {
	sub, err := c.c.Subscribe(toplevel, device)
	if err != nil {
		return err
	}
	c.wg.Add(1)
	go func() {
		defer c.wg.Done()
		for m := range sub.C {
			if str, ok := m.Value.Value.(string); ok && str == value {
				callback()
			}
		}
	}()
	return nil
}
