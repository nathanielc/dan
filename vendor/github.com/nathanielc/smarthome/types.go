package smarthome

import (
	"encoding/json"
	"time"

	"github.com/eclipse/paho.mqtt.golang"
)

const (
	setPath       = "set"
	getPath       = "get"
	commandPath   = "command"
	statusPath    = "status"
	connectedPath = "connected"

	statusPathComplete = "/" + statusPath + "/"
)

type Value struct {
	Value       interface{}
	Time        time.Time
	LastChanged time.Time
}

type valueJSON struct {
	Value       interface{} `json:"val"`
	Time        int64       `json:"ts"`
	LastChanged int64       `json:"lc"`
}

func (v *Value) UnmarshalJSON(text []byte) error {
	val := valueJSON{}
	if err := json.Unmarshal(text, &val); err != nil {
		return err
	}
	v.Value = val.Value
	v.Time = time.Unix(val.Time, 0).UTC()
	v.LastChanged = time.Unix(val.LastChanged, 0).UTC()
	return nil
}
func (v Value) MarshalJSON() ([]byte, error) {
	val := valueJSON{
		Value:       v.Value,
		Time:        v.Time.UnixNano() / 1e9,
		LastChanged: v.LastChanged.UnixNano() / 1e9,
	}
	return json.Marshal(val)
}

func PayloadToValue(data []byte) Value {
	v := Value{}
	if err := json.Unmarshal(data, &v); err != nil && v.Value != nil {
		return v
	}
	v.Value = string(data)
	return v
}

func DefaultMQTTClientOptions() *mqtt.ClientOptions {
	return mqtt.NewClientOptions().
		SetKeepAlive(5 * time.Second).
		SetAutoReconnect(true)
}

type StatusMessage struct {
	Toplevel string
	Item     string
	Value    Value
}
