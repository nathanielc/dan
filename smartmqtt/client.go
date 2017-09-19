package smartmqtt

import (
	"sync"

	mqtt "github.com/eclipse/paho.mqtt.golang"
	"github.com/nathanielc/smarthome"
)

type Value smarthome.Value

type Client interface {
	Set(toplevel, device, value string) error
	Get(toplevel, device string) (Value, error)
	When(toplevel, device, value string, callback func()) (func(), error)
}

type client struct {
	c smarthome.Client

	wg sync.WaitGroup
}

func New(opts *mqtt.ClientOptions) (Client, error) {
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

func (c *client) When(toplevel, device, value string, callback func()) (func(), error) {
	sub, err := c.c.Subscribe(toplevel, device)
	if err != nil {
		return nil, err
	}
	c.wg.Add(1)
	cancel := make(chan struct{}, 1)
	go func() {
		defer c.wg.Done()
		defer sub.Unsubscribe()

		for {
			select {
			case <-cancel:
				return
			case m := <-sub.C:
				if str, ok := m.Value.Value.(string); ok && str == value {
					callback()
				}
			}
		}
	}()
	return func() {
		close(cancel)
	}, nil
}
