# httptosocks
http/https proxy to socks5

This is a dynamic library, only need to include httptosocks.h to use

### Build library
```shell
cargo build --release
```

Want to compile into a static library, modify it in the Cargo.toml file
```toml
[lib]
crate-type = ["staticlib"]
```

### How to use
in cpp 
```cpp
#include "httptosocks.h"

int main() {
    // http/https listen address
    const char* http_addr = "127.0.0.1:10800";
    // target socks5 server address
    const char* socks5_addr = "192.168.199.210:19999";

    // thread will be blocked here
    start(http_addr, socks5_addr);
}
```
