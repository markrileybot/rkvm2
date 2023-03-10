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

1. Build 
```shell
$ cargo build --release
```
2. Make your config
```shell
$ mkdir -p ~/.config/rkvm2 && ./target/release/rkvm2 --dump-config > ~/.config/rkvm2/config.yml
```

You'll get something that looks like this:
```yaml
# RKVM2 Config
#
broadcast_address: 192.168.24.255:45321
switch_keys:
- RightAlt
commander_keys:
- RightCtrl
- Home
commander: false
socket_gid: 0
```

Change the `commander` to `true` on the machine hosting the keyboard and mouse.  Also, change the `socket_gid` to a 
group to which your user belongs.

3. Run the input server
```shell
$ sudo ./target/release/rkvm2-inputd -c /home/mriley/.config/rkvm2/config.yml
```

4. Run rkvm2
```shell
$ ./target/release/rkvm2
```