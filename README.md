[//]: # (README.md)
[//]: # (vim:set ft=markdown)
[//]: # (SPDX-License-Identifier: MIT)
# pCurl-Get

Very simple async URL get CLI program with unlimited parallelism. Parallel URL fetching is done with [Tokio async runtime](https://tokio.rs).

`pCurl-Get` takes one required command line argument -- path to the file with URLs list. It will silently ignore malformed and not responding URLs. For successful URL GET message will be printed on stdout with status code and received bytes.

There is optional argument `--save` which will make program to save fetched URL content into the file in current working directory. File name will follow this pattern:

`{index}-{host name}-{port number}_path-{Sha256 of URL}`,

```
where,
  index -- index of URL in URLs list,
  host name -- hostname for each URL, where each '.' is replaced with '_',
  port number, including default ports '80/443'.
```

If no `--save` option is used, content of fetched URLs will be discarded with reporting to `stdout`.

Also time statistics will be shown: test start time, test end time and duration of running tests in seconds.

## Testing

Unit tests and integration tests are in `tests` directory.

## TODO List

[TODO](TODO.md)