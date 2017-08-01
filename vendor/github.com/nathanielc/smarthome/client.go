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
	c mqtt.Client

	wg      sync.WaitGroup
	closing chan struct{}

	subUpdates     chan subUpdate
	statusMessages chan mqtt.Message
}

func NewClient(opts *mqtt.ClientOptions) (Client, error) {
	c := mqtt.NewClient(opts)
	if token := c.Connect(); token.Wait() && token.Error() != nil {
		return nil, token.Error()
	}
	cli := &client{
		c:              c,
		closing:        make(chan struct{}),
		subUpdates:     make(chan subUpdate),
		statusMessages: make(chan mqtt.Message, 100),
	}
	statusTopic := path.Join("+", statusPath, "#")
	if token := c.Subscribe(statusTopic, 0, cli.handleStatusMessage); token.Wait() && token.Error() != nil {
		return nil, token.Error()
	}

	cli.wg.Add(1)
	go func() {
		defer cli.wg.Done()
		cli.doSubs()
	}()
	return cli, nil
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
	ch := make(chan StatusMessage, 100)
	s := &Subscription{
		c:  c,
		ch: ch,
		C:  ch,
	}
	update := subUpdate{
		Topic: path.Join(toplevel, statusPath, item),
		Add:   true,
		Sub:   s,
		done:  make(chan struct{}),
	}

	// Send update
	select {
	case <-c.closing:
		return nil, errors.New("client closed")
	case c.subUpdates <- update:
	}

	// Wait till update is done
	select {
	case <-c.closing:
		return nil, errors.New("client closed")
	case <-update.done:
	}
	return s, nil
}

func (c *client) unsubscribe(s *Subscription) {
	update := subUpdate{
		Add: false,
		Sub: s,
	}

	select {
	case <-c.closing:
	case c.subUpdates <- update:
	}
}

func (c *client) handleStatusMessage(_ mqtt.Client, m mqtt.Message) {
	select {
	case <-c.closing:
	case c.statusMessages <- m:
	}
}

func (c *client) doSubs() {
	subs := make(map[string][]*Subscription)
	for {
		select {
		case <-c.closing:
			return
		case update := <-c.subUpdates:
			topic := update.Topic
			if update.Add {
				subs[topic] = append(subs[topic], update.Sub)
				close(update.done)
			} else {
				list := subs[topic]
				filtered := list[0:0]
				for _, s := range list {
					if s != update.Sub {
						filtered = append(filtered, s)
					}
				}
				subs[topic] = filtered
			}
		case m := <-c.statusMessages:
			//TODO add wildcard support
			topic := m.Topic()
			list := subs[topic]
			if len(list) == 0 {
				break
			}
			i := strings.Index(topic, statusPathComplete)
			sm := StatusMessage{
				Toplevel: topic[:i],
				Item:     topic[:i+len(statusPathComplete)],
				Value:    PayloadToValue(m.Payload()),
			}
			for _, s := range list {
				select {
				case s.ch <- sm:
				default:
				}
			}
		}
	}
}

func (c *client) Close() {
	close(c.closing)
	c.wg.Wait()
}

type subUpdate struct {
	Topic string
	Sub   *Subscription
	Add   bool
	done  chan struct{}
}

type Subscription struct {
	c  *client
	ch chan StatusMessage
	C  <-chan StatusMessage
}

func (s *Subscription) Unsubscribe() {
	s.c.unsubscribe(s)
}
