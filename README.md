# doh-proxy-rs

Proxy DNS over HTTPS requests

## Usage

```bash
doh-proxy-rs --server --listen=0.0.0.0:443 --upstream=1.1.1.1:53 --identity=./server.pfx --password=foobar
```

The server requires a PKCS#12 identity. You can generate it from a key pair in PEM format:

```bash
openssl pkcs12 -export -out server.pfx -inkey privkey.pem -in fullchain.pem
```

PEM certificate support depends on `native-tls` (sfackler/rust-native-tls#27).

## Known issues

- Identity loads failed when the password is empty.

## TODO

- Client
