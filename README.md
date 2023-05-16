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
$ ./target/release/rkvm2
```

## Make things run automagically.  Do this on all machines sharing the keyboard/mouse.

1. Create a systemd unit for the input server in `/etc/systemd/system/rkvm2-inputd.service`

```
[Unit]
Description=RKVM2 Input Daemon

[Service]
Type=simple
ExecStart=/path/to/rkvm2/target/release/rkvm2-inputd -c /path/to/.config/rkvm2/config.yml
Restart=on-failure
RestartSec=1
Environment=RUST_LOG=debug
Environment=RUST_BACKTRACE=1

[Install]
WantedBy=default.target
```

2. Reload units

```shell
sudo systemctl daemon-reload
```

3. Enable the service

```shell
sudo systemctl enable rkvm2-inputd
```

4. Start the service

```shell
sudo systemctl start rkvm2-inputd
```

5. Start the client.  I use i3 so I have this in my sway config:

```
exec --no-startup-id exec systemd-cat -t rkvm2 /path/to/rkvm2/target/release/rkvm2
```
