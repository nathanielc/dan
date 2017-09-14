# Jim

Jim is a home automation assistant.

Jim comes with a simple DSL for connecting to devices via the [mqtt-smarthome](https://github.com/mqtt-smarthome) architecture.

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

* jim - An interactive REPL command for running DSL commands interactively.
* jimd - A server daemon that will run scripts from a configured directory.
