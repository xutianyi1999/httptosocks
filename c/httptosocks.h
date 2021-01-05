//
// Created by xutia on 2020/12/19.
//

#ifndef HTTPTOSOCKS_HTTPTOSOCKS_H
#define HTTPTOSOCKS_HTTPTOSOCKS_H

extern "C" {

void start(const char *proxy_addr,
           unsigned char proxy_addr_len,
           const char *socks5_addr,
           unsigned char socks5_addr_len);
};


#endif //HTTPTOSOCKS_HTTPTOSOCKS_H
