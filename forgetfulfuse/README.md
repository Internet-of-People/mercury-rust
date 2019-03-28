
# Forgetful Filesystem

## Table of contents

- [Forgetful Filesystem](#forgetful-filesystem)
  - [Table of contents](#table-of-contents)
  - [Reporting Vulnerabilities](#reporting-vulnerabilities)
  - [Usage](#usage)
  - [Contributing](#contributing)
  - [License](#license)

## Reporting Vulnerabilities

## Usage

To mount a new instance of the filesystem, run the binary like this:

`forgetfulfuse /run/user/1000/forgetful`

The binary will not exit until the mount point is not unmounted. As a non-privileged user this can help:

`fusermount -u /run/user/1000/forgetful`

Of course in practice the filesystem will be mounted by a systemd unit run by the user. For testing it, run these in the project directory:

```sh
# Install the binary to a user-specific folder
$ cargo install --path .
# Create a directory for user-specific systemd unit files
$ mkdir -p $HOME/.config/systemd/user
# Copy over the service file, editing the binary path to the user-specific one
$ cat forgetful.service | sed -e "s#/bin/forgetfulfuse#$HOME/.cargo/bin/forgetfulfuse#" > $HOME/.config/systemd/user/forgetful.service
# Systemd needs some nudging to reread files and directories...
$ systemctl --user daemon-reload
# Starting the service mounts the filesystem
$ systemctl --user start forgetful
# The last 100 characters in the sample file is shown
$ tail -c 100 /run/user/1000/forgetful/hello.txt
ellentesque ut metus non nulla luctus condimentum. Etiam quis lectus porta orci sagittis imperdiet.
# Stopping the service properly umounts the filesytem
$ systemctl --user stop forgetful
# So accessing files in it properly fails
$ tail -c 100 /run/user/1000/forgetful/hello.txt
tail: cannot open '/run/user/1000/forgetful/hello.txt' for reading: No such file or directory
```

If you want to always mount the filesystem whenever you login to your developer machine:

```sh
$ systemctl --user enable forgetful
Created symlink from $HOME/.config/systemd/user/multi-user.target.wants/forgetful.service to $HOME/.config/systemd/user/forgetful.service.
```

## Contributing

## License

[GPL-v3-or-later](LICENSE)
