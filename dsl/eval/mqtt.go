package eval

import (
	"path"
	"regexp"
	"strings"
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
	c            smarthome.Client
	deviceLookup smarthome.DeviceLookup

	wg sync.WaitGroup
}

func newClient(c smarthome.Client) (*client, error) {
	deviceLookup, err := smarthome.NewDeviceLookup(c)
	if err != nil {
		return nil, err
	}
	return &client{
		c:            c,
		deviceLookup: deviceLookup,
	}, nil
}

func (c *client) Close() {
	c.deviceLookup.Close()
	c.c.Close()
}

func (c *client) Set(toplevel, device, value string) error {
	if containsWildcard(device) {
		match := convertToRegex(device)
		devices := c.deviceLookup.Find(toplevel, match)
		var lastErr error
		for _, d := range devices {
			err := c.c.Set(toplevel, d.Item, value)
			if err != nil {
				lastErr = err
			}
		}
		return lastErr
	}
	return c.c.Set(toplevel, device, value)
}

func containsWildcard(device string) bool {
	parts := strings.Split(device, "/")
	for _, p := range parts {
		if p == "*" || p == "**" {
			return true
		}
	}
	return false
}

func convertToRegex(device string) *regexp.Regexp {
	parts := strings.Split(device, "/")
	for i, p := range parts {
		switch p {
		case "*":
			parts[i] = "[^/]+"
		case "**":
			parts[i] = ".*"
		default:
			parts[i] = regexp.QuoteMeta(p)
		}
	}
	r := path.Join(parts...)
	return regexp.MustCompile(r)
}

func (c *client) Get(toplevel, device string) (smarthome.Value, error) {
	// First check the deviceLookup
	d, ok := c.deviceLookup.Device(toplevel, device)
	if ok {
		return d.Value, nil
	}
	// Make active request
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
