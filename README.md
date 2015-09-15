# Zone of Control

[![travis-ci-img][]] [travis-ci]
[![gitter-img][]] [gitter]

## Overview

ZoC is turn-based hexagonal strategy game written in
[Rust][].

![gameplay-gif][]

(recorded with [byzanz](http://askubuntu.com/a/201018))


## Assets

Basic game assets are stored in [separate repo][].
ZoC expects them in `assets` directory:

`git clone --depth=1 https://github.com/ozkriff/zoc_assets assets`

NOTE: If game will not die in early stage of development I'm planning
to release actual game resources under proprietary license.


## Building

`make` or `cargo build` (latest rust-nightly is required, see `.travis.yml`).


## Running

`make run` or `cargo run` or `./target/zoc`.

(Tested in ubuntu 14.04 and win 8.1.)


## Android

For instructions on setting up your environment see
https://github.com/tomaka/android-rs-glue#setting-up-your-environment.

Then just: `make android_run`.

![android-img][]

(Tested on nexus7 2014/android 5 and fly iq440/android 4.)


## Contribute

Feel free to report bugs and patches using GitHub's pull requests
system on https://github.com/ozkriff/zoc. Any feedback would be much
appreciated!

NOTE: You must apologize my English level. I'm trying to do my best :) . Please open an issue if anything in docs or comments is strange/unclear/can be improved.


## License

ZoC is licensed under the MIT license (see the `LICENSE` file).


[rust]: https://rust-lang.org
[gameplay-gif]: http://i.imgur.com/orQtkqF.gif
[separate repo]: https://github.com/ozkriff/zoc_assets
[travis-ci-img]: https://travis-ci.org/ozkriff/zoc.png?branch=master
[travis-ci]: https://travis-ci.org/ozkriff/zoc
[gitter-img]: https://badges.gitter.im/....svg
[gitter]: https://gitter.im/ozkriff/zoc
[android-img]: http://i.imgur.com/Fp3Z5I1l.png
