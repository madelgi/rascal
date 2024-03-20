# Rascal

A simple request runner that can execute requests defined in templated JSON files.

## Usage

Requests have the following format:

```json
{
    // optional, defaults to HTTP/2
    "version": "str | null",
    // required, any valid HTTP verb
    "method": "str",
    // required, the url to fetch
    "url": "str",
    // optional, header map
    "headers": { "str": "str" },
    // optional, currently supports "Basic" and "Bearer" modes. "Basic" requires
    // username/password to be present, and "Bearer" requires "token". this
    // will ultimately overwrite the `authorization` header if you specify
    // that in `headers`.
    "auth": {
        "type": "str",
        "username": "str | null",
        "password": "str | null",
        "token": "str | null"
    }
```

Requests also support basic template arguments:

```json
{
    "method": "GET",
    "url": "https://{{ env_HOST }}/api/2/{{ arg_pathparam }}/foo"
}
```

Template parameters prefixed with `env_` search for environment variables, and those
prefixed with `arg_` expect a parameter to be passed via commandline using the `-k` flag.
Right now this isn't packaged/distributed in any sensible way, but you can clone the
repository and run it directly via cargo:

```
$ cargo run exec -k pathval --pretty-print path/to/req.json
```

Alternatively, you can build a release binary and use that:

```
$ cargo build --release
$ ./target/release/rascal exec -k pathval --pretty-print path/to/req.json
```

