# Jim

[![mqtt-smarthome](https://img.shields.io/badge/mqtt-smarthome-blue.svg)](https://github.com/mqtt-smarthome/mqtt-smarthome)

Jim is a home automation assistant.

Jim comes with a simple DSL for connecting to devices via MQTT.
The language suppors working with the [mqtt-smarthome](https://github.com/mqtt-smarthome) architecture.

## DSL Example

```
scene nightime {
    set almond/door/DoorLock/front locked

    when
        almond/door/DoorLock/front is unlocked
    wait 5m
        set almond/door/DoorLock/front locked
}

scene daytime {
    when
        almond/door/DoorLock/front is unlocked
    wait 15m
        set almond/door/DoorLock/front locked
}

at 8:00AM {
    stop  nightime
    start daytime
}

at 9:00AM {
    stop daytime
    start nightime
}

```


## Binaries

This project contains two commands that can be run:

* jim - An command for running a Jim script file.
