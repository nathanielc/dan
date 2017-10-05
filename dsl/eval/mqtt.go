package eval

import (
	"sync"

	"github.com/nathanielc/smarthome"
)

type Client interface {
	Set(toplevel, device, value string) error
	Get(toplevel, device string) (smarthome.Value, error)
	When(toplevel, device, value string, callback func()) (func(), error)
	Close()
}

type client struct {
	c smarthome.Client

	wg sync.WaitGroup
}

func (c *client) Close() {
	c.c.Close()
}

func (c *client) Set(toplevel, device, value string) error {
	return c.c.Set(toplevel, device, value)
}

func (c *client) Get(toplevel, device string) (smarthome.Value, error) {
	v, err := c.c.Get(toplevel, device)
	if err != nil {
		return smarthome.Value{}, err
	}
	return smarthome.Value(v), nil
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
