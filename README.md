# rkvm2
rusty kvm based on the great work done here https://github.com/htrefil/rkvm.  The big differences are:
1. Connectionless because I think that's cool
2. Disconnected the input server from the main app
3. Clipboard integration
4. Notification integration

Lots of work to be done, including:
1. encryption over the net
2. cli for fun stuff
3. doesn't really work well yet lol
4. Screen geometry support

## Setup

1. Install dependencies
* [libevdev](https://www.freedesktop.org/wiki/Software/libevdev/) (development libraries)
* [protobuf](https://grpc.io/docs/protoc-installation/) (compiler and development libraries)
* [libclang-dev](https://releases.llvm.org/) (development libraries)

On ubuntu, you can install these dependencies with:  `sudo apt install libevdev-dev protobuf-compiler libprotobuf-dev libclang-dev`

2. Build 
```shell
$ cargo build --release
```
3. Make your config
```shell
$ mkdir -p ~/.config/rkvm2 && target/release/rkvm2 --dump-config > ~/.config/rkvm2/config.yml
```

You'll get something that looks like this:
```yaml
# RKVM2 Config

broadcast_address: 192.168.24.255:45321
switch_keys:
- RightCtrl
- RightAlt
commander_keys:
- RightCtrl
- Home
commander: false
socket_gid: 0
```

* Change the broadcast address.  You can find the broadcast address by running:  `ip address` on linux/mac or `ifconfig` on windows.
* Change the `commander` to `true` on the machine hosting the keyboard and mouse.
* Change the `socket_gid` to a group to which your user belongs (only required on linux/mac).

4. Run the input server
```shell
$ sudo RUST_LOG=debug target/release/rkvm2-inputd -c $HOME/.config/rkvm2/config.yml
```

5. Run rkvm2
```shell
$ RUST_LOG=debug target/release/rkvm2
```
