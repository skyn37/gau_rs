### **gau_rs** is a load testing tool written in Rust the name is inspired by "**GAU-8 Avenger**" autocannon mounted on the **Fairchild Republic A-10** ***Thunderbolt II***. 
---

### Supports the following features:

- **GET** and **POST** requests
- Configurable number of requests
- Configurable number of concurrent requests
- Configurable tasks

```rust
gau_rs [OPTIONS] --url <URL> --method <METHOD>
```
```rust
Options:
  -u, --url <URL>
  -m, --method <METHOD>
  -d, --data <DATA>
  -n, --number-of-requests <NUMBER_OF_REQUESTS>  [default: 1]
  -c, --concurent-requests <CONCURENT_REQUESTS>  [default: 1]
  -t, --tasks <TASKS>                            [default: 1]
  -h, --help                                     Print help
  -V, --version                                  Print version
  ```

| Status | Feature |
| --- | -------------------------- |
0%	|  Add tracking & reporting  |
0%	|  Load from JSON |
0%  |  Add random delays & ramp-up |
0%  |  Scripted multi-server launch |
0%  |  Headers, cookies, auth |
0%  |  UNIX domain socket support |
0%  |  Pipelining |




---
### The tool is writen in Rust it uses the following libraries:
- [reqwest](https://docs.rs/reqwest/0.11.3/reqwest/) for making HTTP requests
- [tokio](https://docs.rs/tokio/1.0.1/tokio/) for asyncronous programming
- [serde](https://docs.rs/serde/1.0.123/serde/) for serializing and deserializing data
- [serde_json](https://docs.rs/serde_json/1.0.64/serde_json/) for serializing and deserializing JSON data
- [clap](https://docs.rs/clap/2.33.3/clap/) for parsing command line arguments
