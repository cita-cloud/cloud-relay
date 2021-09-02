# cloud-relay

> `cloud-relay` 作为中间件用于将 http 请求转发为链上 grpc 调用，目前支持两种转发：
> 1. GET http://<ip>:<port>/block_number 查询当前块高
> 2. GET http://<ip>:<port>/auto_send_store_transaction 自动发送存证交易，不发送任何参数，由 `cloud-relay` 自动完成交易签名和打包

## help

```
$ ./cloud-relay --help
CITA-CLOUD RELAY SERVER 0.1.0

Yieazy, Rivtower Technologies.

relay http request to grpc

USAGE:
    cloud-relay [OPTIONS]

FLAGS:
    -h, --help       Print help information
    -V, --version    Print version information

OPTIONS:
    -c, --controller <controller-addr>
            Set controller rpc address, default http://127.0.0.1:50004

    -e, --evm <evm-addr>
            Set evm rpc address, default http://127.0.0.1:50002

    -k, --key <private-key>
            Set private key, default
            3ef2627393529fed043c7dbfd9358a4ae47a88a59949b07e7631722fd6959002

    -p, --port <listen-port>
            Relay server listen port, default 1337
```

ps. 四个配置项均有默认值，如需要修改请参考默认值形式，controller-addr 和 evm-addr 的端口均为各自的 rpc 端口，private-key 为 64位hexstr，listen-port 为服务监听端口