Configuration file for Kiosque




### Splunk reader, file output

```
input {
  splunk {
    # SSL for certificates
    cert-file = "assets/server.crt"
    cert-key = "assets/server.key"
  }
}

output {
  # capnp output
  file {
    # base directory where data will be written
    # to this directory is appended:
    # - the file creation date
    # - you can use the following interpolations:
    #   - attrs.##ATTRIBUTE_NAME##
    #   - host
    # D: "./logs"
    directory = "./logs/#{attrs.facility_name}"

    # - default log size in number of messages
    # D: 5000
    flush_every 1000
  }
}
```

### With web server

```
output {
  http {
    # HTTP Server for the logs. If file output is also activated, http has also access
    # to these logs
    # Where to find the log, perform recursive descent
    # D: "./logs"
    directory = "./logs"
    # max number of memory to allocare for raw files
    # D: "5G"
    max_memory = "2G"
    # Authentication for user
    # user = "foo"
    # pass = "bar"

      }
}
```

### Tests input

```
input {
  fake-apache {
    rate 10
  }
}
```
