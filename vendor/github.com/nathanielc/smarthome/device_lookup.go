package smarthome

import (
	"regexp"
	"sync"
)

type DeviceLookup interface {
	Device(toplevel, item string) (Device, bool)
	Find(toplevel string, itemMatch *regexp.Regexp) []Device
	Close()
}

type Device struct {
	Toplevel string
	Item     string
	Value    Value
}

func NewDeviceLookup(c Client) (DeviceLookup, error) {
	// Subscribe to all status messages
	sub, err := c.Subscribe("+", "#")
	if err != nil {
		return nil, err
	}
	dl := &deviceLookup{
		devices: make(map[deviceID]Device),
		closing: make(chan struct{}),
	}
	dl.wg.Add(1)
	go func() {
		defer dl.wg.Done()
		dl.watch(sub)
	}()
	return dl, nil
}

type deviceLookup struct {
	mu      sync.RWMutex
	wg      sync.WaitGroup
	devices map[deviceID]Device
	closing chan struct{}
	closed  bool
}

type deviceID struct {
	Toplevel string
	Item     string
}

func (dl *deviceLookup) Device(toplevel, item string) (Device, bool) {
	dl.mu.RLock()
	defer dl.mu.RUnlock()
	d, ok := dl.devices[deviceID{Toplevel: toplevel, Item: item}]
	return d, ok
}
func (dl *deviceLookup) Find(toplevel string, itemMatch *regexp.Regexp) []Device {
	dl.mu.RLock()
	defer dl.mu.RUnlock()
	var found []Device
	for id, d := range dl.devices {
		if id.Toplevel == toplevel &&
			itemMatch.MatchString(id.Item) {
			found = append(found, d)
		}
	}
	return found
}

func (dl *deviceLookup) Close() {
	dl.mu.Lock()
	defer dl.mu.Unlock()
	if dl.closed {
		return
	}
	dl.closed = true
	close(dl.closing)
	dl.wg.Wait()
}

func (dl *deviceLookup) watch(sub *Subscription) {
	defer sub.Unsubscribe()
	for {
		select {
		case <-dl.closing:
			return
		case sm := <-sub.C:
			dl.handleStatusMessage(sm)
		}
	}
}

func (dl *deviceLookup) handleStatusMessage(sm StatusMessage) {
	dl.mu.Lock()
	defer dl.mu.Unlock()
	id := deviceID{
		Toplevel: sm.Toplevel,
		Item:     sm.Item,
	}
	d, ok := dl.devices[id]
	if !ok {
		d = Device{
			Toplevel: sm.Toplevel,
			Item:     sm.Item,
			Value:    sm.Value,
		}
	}
	d.Value = sm.Value
	dl.devices[id] = d
}
