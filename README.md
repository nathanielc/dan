# Dan

[![mqtt-smarthome](https://img.shields.io/badge/mqtt-smarthome-blue.svg)](https://github.com/mqtt-smarthome/mqtt-smarthome)

Dan is a home automation programming language.
The langauge has native support for working with MQTT.

## Dan Example

Lock all the doors at 10PM each night.

```
scene night {
    print "starting night scene"

    set zwave/Front/DoorLock/98/0/targetMode/set {value: 255}
    set zwave/Garage/DoorLock/98/0/targetMode/set {value: 255}

    set zwave/Kitchen/DoorLock/98/0/targetMode/set {value: 255}
}


at 10:00PM start night
```

## Installing

Install the dan binary using cargo:

```
$ cargo install dan
```


## Running

Place the above example in a directory `./dan.d` and invoke dan:

```
$ dan --mqtt-url mqtt://localhost --dir ./dan.d
```
