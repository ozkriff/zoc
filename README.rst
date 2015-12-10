
Zone of Control
===============

|license|_
|travis-ci|_
|appveyor-ci|_
|gitter|_


Overview
--------

ZoC is turn-based hexagonal strategy game written in Rust_.

.. image:: http://i.imgur.com/ytI2tdk.png

Video: http://www.youtube.com/watch?v=srJAfngSwxk


Assets
------

Basic game assets are stored in separate repo:
https://github.com/ozkriff/zoc_assets

Run ``make assets`` to download them.

NOTE: If game will not die in early stage of development I'm planning
to release actual game resources under proprietary license.


Building
--------

``make`` or ``cargo build``.


Running
-------

``make run`` or ``cargo run`` or ``./target/zoc``.

(Tested in ubuntu 14.04 and win 8.1.)


Android
-------

For instructions on setting up your environment see
https://github.com/tomaka/android-rs-glue#setting-up-your-environment.

Then just: ``make android_run`` (rust-nightly is required).

(Tested on nexus7/android5)


Contribute
----------

Feel free to report bugs and patches using GitHub's pull requests
system on https://github.com/ozkriff/zoc. Any feedback would be much
appreciated!

NOTE: You must apologize my English level. I'm trying to do my best :) .
Please open an issue if anything in docs or comments is strange/unclear/can
be improved.


License
-------

MIT_


.. |license| image:: http://img.shields.io/badge/license-MIT-blue.svg
.. |travis-ci| image:: https://travis-ci.org/ozkriff/zoc.svg?branch=master
.. |appveyor-ci| image:: https://ci.appveyor.com/api/projects/status/49kqaol7dlt2xrec/branch/master?svg=true
.. |gitter| image:: https://badges.gitter.im/....svg
.. _Rust: https://rust-lang.org
.. _MIT: https://github.com/ozkriff/zoc/blob/master/LICENSE
.. _license: https://github.com/ozkriff/zoc/blob/master/LICENSE
.. _travis-ci: https://travis-ci.org/ozkriff/zoc
.. _appveyor-ci: https://ci.appveyor.com/project/ozkriff/zoc
.. _gitter: https://gitter.im/ozkriff/zoc
