## Splunk server for docker logging

```
cargo run splunk_server
```

```
docker run -it -P --log-driver=splunk --log-opt splunk-token=TOKEN --log-opt splunk-url=https://10.0.2.2:6767/ --log-opt splunk-insecureskipverify=true  gogs/gogs
```


## Log manipulation Engine

# After ec90190dedc7275710bbf41408147d6bd142809f

Still 8 worker threads


- capnp is parser during search
- faster startup, lower memory usage
- slower perf(?)


## Timing information for root profiler (for 1000 * 5000 lines)
- Simple search - 1 * 8.5s = 8.5s @ 0.1hz
- Regex search - 1 * 2.7s = 2.7s @ 0.4hz


## Timing information for root profiler (for 10000 * 5000 lines)
  Simple search - 1 * 31.9s = 31.9s @ 0.0hz
  Regex search - 1 * 15.9s = 15.9s @ 0.1hz

# Old engine

- capn proto is parsed at startup. Slower startup, larger memory usage


100 files -> 130Mo

Timing information for root profiler:
  Simple search - 99 * 38.0ms = 3.8s @ 26.3hz
  Regex search - 99 * 34.0ms = 3.4s @ 29.4hz

1000 files -> 1.3Go

Timing information for root profiler:
  Simple search - 99 * 430.9ms = 42.7s @ 2.3hz
  Regex search - 99 * 297.9ms = 29.5s @ 3.4hz


