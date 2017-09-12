package eval

import (
	"fmt"
	"time"

	"github.com/gorhill/cronexpr"
)

func scheduleDaily(hour, minute int, callback func(time.Time)) (func(), error) {
	cron := fmt.Sprintf("%d %d * * *", minute, hour)
	return scheduleCron(cron, callback)
}

func scheduleCron(cron string, callback func(time.Time)) (func(), error) {
	expr, err := cronexpr.Parse(cron)
	if err != nil {
		return nil, err
	}
	cancel := make(chan struct{}, 1)
	go func() {
		nextTime := time.Now()
		for {
			nextTime = expr.Next(nextTime)
			timer := time.NewTimer(nextTime.Sub(time.Now()))
			defer timer.Stop()
			select {
			case <-cancel:
				return
			case <-timer.C:
				callback(nextTime)
			}
		}
	}()
	return func() {
		close(cancel)
	}, nil
}
