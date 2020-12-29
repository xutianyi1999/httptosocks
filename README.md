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
#include <string>
#include "httptosocks.h"

int main() {
    // http/https listen address
    std::string http_addr = "127.0.0.1:10800";
    unsigned char http_addr_len = http_addr.length();

    // target socks5 server address
    std::string socks5_addr = "192.168.199.210:19999";
    unsigned char socks5_addr_len = socks5_addr.length();

    // thread will be blocked here
    start(http_addr.c_str(), http_addr_len, socks5_addr.c_str(), socks5_addr_len);
}
```
