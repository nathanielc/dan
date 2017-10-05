package eval

import (
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/gorhill/cronexpr"
)

type schedule struct {
	mu      sync.Mutex
	wg      sync.WaitGroup
	closing chan struct{}
	closed  bool

	events []*event
}

func newSchedule() *schedule {
	return &schedule{
		closing: make(chan struct{}),
	}
}

type event struct {
	t           timer
	description string
}

type timer interface {
	next(time.Time) time.Time
}

type Event struct {
	Time        time.Time
	Description string
}

func (s *schedule) Upcoming(n time.Time) []Event {
	s.mu.Lock()
	defer s.mu.Unlock()

	events := make([]Event, len(s.events))
	for i, e := range s.events {
		events[i] = Event{
			Time:        e.t.next(n),
			Description: e.description,
		}
	}
	return events
}

func (s *schedule) Close() {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.closed {
		return
	}
	s.closed = true
	close(s.closing)
	s.wg.Wait()
}

func (s *schedule) Add(t timer, desc string, callback func(time.Time)) (func(), error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.closed {
		return nil, errors.New("schedule closed")
	}
	cancel := make(chan struct{}, 1)
	s.wg.Add(1)
	go func() {
		defer s.wg.Done()
		nextTime := time.Now()
		for {
			nextTime = t.next(nextTime)
			timer := time.NewTimer(nextTime.Sub(time.Now()))
			select {
			case <-s.closing:
				timer.Stop()
				return
			case <-cancel:
				timer.Stop()
				return
			case <-timer.C:
				callback(nextTime)
			}
			timer.Stop()
		}
	}()

	e := &event{
		t:           t,
		description: desc,
	}
	cancelF := func() { close(cancel); s.remove(e) }
	s.events = append(s.events, e)
	return cancelF, nil
}

func (s *schedule) remove(e *event) {
	s.mu.Lock()
	defer s.mu.Unlock()
	for i, evnt := range s.events {
		if evnt == e {
			events := s.events[0:i]
			s.events = append(events, s.events[i+1:]...)
			break
		}
	}
}

func (s *schedule) DailyTimer(hour, minute int) (timer, error) {
	cron := fmt.Sprintf("%d %d * * *", minute, hour)
	return s.CronTimer(cron)
}

func (s *schedule) CronTimer(cron string) (timer, error) {
	expr, err := cronexpr.Parse(cron)
	if err != nil {
		return nil, err
	}
	return cronTimer{expr: expr}, nil
}

type cronTimer struct {
	expr *cronexpr.Expression
}

func (t cronTimer) next(n time.Time) time.Time {
	return t.expr.Next(n)
}

type sunTimer struct {
	lat, lon float64
	nextF    func(time.Time, float64, float64) time.Time
}

func (t sunTimer) next(n time.Time) time.Time {
	return t.nextF(n, t.lat, t.lon)
}
