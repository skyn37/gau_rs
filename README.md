### **gau_rs** is a load testing tool written in Rust the name is inspired by "**GAU-8 Avenger**" autocannon mounted on the **Fairchild Republic A-10** ***Thunderbolt II***. 
---

### Supports the following features:

- **GET** and **POST** requests
- Configurable number of requests
- Configurable number of concurrent requests
- Configurable tasks

-- **USAGE**:

```rust
gau_rs [OPTIONS] --url <URL> --method <METHOD>
```

-- **What works**:

| Status | Feature |
| --- | -------------------------- |
20%	|  Add tracking & reporting  |
0%	|  Load from JSON |
0%  |  Add random delays & ramp-up |
0%  |  Scripted multi-server launch |
50%  |  Headers, cookies, | 
0%  |  UNIX domain socket support |




---
### The tool is writen in Rust it uses the following libraries:
- [reqwest](https://docs.rs/reqwest/0.11.3/reqwest/) for making HTTP requests
- [tokio](https://docs.rs/tokio/1.0.1/tokio/) for asyncronous programming
- [serde](https://docs.rs/serde/1.0.123/serde/) for serializing and deserializing data
- [serde_json](https://docs.rs/serde_json/1.0.64/serde_json/) for serializing and deserializing JSON data
- [clap](https://docs.rs/clap/2.33.3/clap/) for parsing command line arguments
