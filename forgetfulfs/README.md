
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

`forgetfulfs /run/user/1000/forgetful`

The binary will not exit until the mount point is not unmounted. As a non-privileged user this can help:

`fusermount -u /run/user/1000/forgetful`

Of course in practice the filesystem will be mounted by a systemd unit owned by the user. **TODO describe how to write such a unit**

## Contributing

## License

[GPL-v3-or-later](LICENSE)
