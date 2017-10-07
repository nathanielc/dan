package smarthome

import (
	"errors"
	"path"
	"strings"
	"sync"
	"time"

	"github.com/eclipse/paho.mqtt.golang"
)

type Client interface {
	// Set publishes a set message with the value
	Set(toplevel, item string, value string) error
	// Get publishes a get request message.
	Get(toplevel, item string) (Value, error)
	// Command publishes a command to the toplevel topic.
	Command(toplevel string, cmd []byte) error

	// Subscribe to receive callbacks whenever a status message is received.
	Subscribe(toplevel, item string) (*Subscription, error)

	// Close disconnects the client.
	Close()
}

type client struct {
	mu         sync.Mutex
	c          mqtt.Client
	disconnect bool

	closed  bool
	closing chan struct{}

	subs map[string]*subscription
}

func NewClient(opts *mqtt.ClientOptions) (Client, error) {
	c := mqtt.NewClient(opts)
	if token := c.Connect(); token.Wait() && token.Error() != nil {
		return nil, token.Error()
	}
	return newClient(c, true), nil
}

func newClient(c mqtt.Client, disconnect bool) Client {
	return &client{
		c:          c,
		disconnect: disconnect,
		closing:    make(chan struct{}),
		subs:       make(map[string]*subscription),
	}
}

func (c *client) Set(toplevel, item string, value string) error {
	topic := path.Join(toplevel, setPath, item)
	token := c.c.Publish(topic, 0, false, value)
	token.Wait()
	return token.Error()
}

func (c *client) Get(toplevel, item string) (Value, error) {
	s, err := c.Subscribe(toplevel, item)
	if err != nil {
		return Value{}, err
	}
	defer s.Unsubscribe()

	getTopic := path.Join(toplevel, getPath, item)
	if token := c.c.Publish(getTopic, 0, false, "?"); token.Wait() && token.Error() != nil {
		return Value{}, token.Error()
	}

	timer := time.NewTimer(5 * time.Second)
	defer timer.Stop()
	select {
	case <-c.closing:
		return Value{}, errors.New("client closed")
	case <-timer.C:
		return Value{}, errors.New("timed out waiting for get response")
	case sm := <-s.C:
		return sm.Value, nil
	}
}

func (c *client) Command(toplevel string, cmd []byte) error {
	topic := path.Join(toplevel, commandPath)
	token := c.c.Publish(topic, 0, false, cmd)
	token.Wait()
	return token.Error()
}

func (c *client) Subscribe(toplevel, item string) (*Subscription, error) {
	c.mu.Lock()
	defer c.mu.Unlock()

	statusTopic := path.Join(toplevel, statusPath, item)
	sub, ok := c.subs[statusTopic]
	if !ok {
		sub = &subscription{
			c:     c,
			topic: statusTopic,
		}
		if token := c.c.Subscribe(statusTopic, 0, sub.handleStatusMessage); token.Wait() && token.Error() != nil {
			return nil, token.Error()
		}
		c.subs[statusTopic] = sub
	}
	return sub.subscribe(), nil
}

func (c *client) unsubscribe(topic string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	delete(c.subs, topic)
	c.c.Unsubscribe(topic)
}

func (c *client) Close() {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.closed {
		return
	}
	c.closed = true
	close(c.closing)
	if c.disconnect {
		c.c.Disconnect(defaultDisconnectQuiesce)
	}
}

type Subscription struct {
	s  *subscription
	ch chan StatusMessage
	C  <-chan StatusMessage
}

func (s *Subscription) Unsubscribe() {
	s.s.unsubscribe(s)
}

type subscription struct {
	topic string
	c     *client
	mu    sync.Mutex
	subs  []*Subscription
}

func (s *subscription) handleStatusMessage(c mqtt.Client, m mqtt.Message) {
	s.mu.Lock()
	defer s.mu.Unlock()
	topic := m.Topic()
	i := strings.Index(topic, statusPathComplete)
	sm := StatusMessage{
		Toplevel: topic[:i],
		Item:     topic[i+len(statusPathComplete):],
		Value:    PayloadToValue(m.Payload()),
	}

	for _, sub := range s.subs {
		select {
		case sub.ch <- sm:
		}
	}
}

func (s *subscription) subscribe() *Subscription {
	s.mu.Lock()
	defer s.mu.Unlock()
	ch := make(chan StatusMessage)
	sub := &Subscription{
		s:  s,
		ch: ch,
		C:  ch,
	}
	s.subs = append(s.subs, sub)
	return sub
}

func (s *subscription) unsubscribe(unsub *Subscription) {
	s.mu.Lock()
	defer s.mu.Unlock()
	filtered := s.subs[0:0]
	for _, sub := range s.subs {
		if sub != unsub {
			filtered = append(filtered, sub)
		}
	}
	s.subs = filtered
	if len(filtered) == 0 {
		s.c.unsubscribe(s.topic)
	}
}
