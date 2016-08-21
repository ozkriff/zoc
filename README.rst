
Zone of Control
===============

|license|_
|travis-ci|_
|appveyor-ci|_
|dependency-ci|_
|gitter|_
|backhub|_


Overview
--------

ZoC is turn-based hexagonal strategy game written in Rust_.

.. image:: http://i.imgur.com/gZeJJGM.png

.. image:: http://i.imgur.com/POhSLwh.png

.. image:: http://i.imgur.com/qnZZgdX.png

Videos:

- AI, reaction fire and sectors (2016.06.08): https://youtu.be/hI6YmZeuZ3s
- transporter, roads (2016.08.07): https://youtu.be/_0_U-h1KCAE
- smoke, water and bridge (2016.08.20): https://youtu.be/WJHkuWwAb7A


Assets
------

Basic game assets are stored in separate repo:
https://github.com/ozkriff/zoc_assets

Run ``make assets`` (or ``git clone https://github.com/ozkriff/zoc_assets assets``) to download them.

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

Then just: ``make android_run`` - this will build .apk, install and run it.


Contribute
----------

Feel free to report bugs and patches using GitHub's pull requests
system on https://github.com/ozkriff/zoc. Any feedback would be much
appreciated!

NOTE: You must apologize my English level. I'm trying to do my best :) .
Please open an issue if anything in docs or comments is strange/unclear/can
be improved.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.


License
-------

ZoC is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See `LICENSE-APACHE`_ and `LICENSE-MIT`_ for details.


.. |license| image:: https://img.shields.io/badge/license-MIT_or_Apache_2.0-blue.svg
.. |travis-ci| image:: https://travis-ci.org/ozkriff/zoc.svg?branch=master
.. |appveyor-ci| image:: https://ci.appveyor.com/api/projects/status/49kqaol7dlt2xrec/branch/master?svg=true
.. |dependency-ci| image:: https://dependencyci.com/github/ozkriff/zoc/badge
.. |gitter| image:: https://badges.gitter.im/....svg
.. |backhub| image:: https://img.shields.io/badge/BackHub-Backed%20up-brightgreen.svg
.. _Rust: https://rust-lang.org
.. _LICENSE-MIT: LICENSE-MIT
.. _LICENSE-APACHE: LICENSE-APACHE
.. _travis-ci: https://travis-ci.org/ozkriff/zoc
.. _appveyor-ci: https://ci.appveyor.com/project/ozkriff/zoc
.. _dependency-ci: https://dependencyci.com/github/ozkriff/zoc
.. _gitter: https://gitter.im/ozkriff/zoc
.. _backhub: https://backhub.co
