
Zone of Control
===============

|license|_
|loc|_
|travis-ci|_
|appveyor-ci|_


The project is discontinued
---------------------------

Sorry, friends. ZoC is discontinued. See https://ozkriff.github.io/2017-08-17--devlog.html


Downloads
---------

Precompiled binaries for linux, win and osx: https://github.com/ozkriff/zoc/releases


Overview
--------

ZoC is a turn-based hexagonal strategy game written in Rust_.

Core game features are:

- advanced fog of war
- slot system (single tile fits multiple units)
- reaction fire (xcom-like)
- morale and suppression

.. image:: http://i.imgur.com/TYoAVj6.png

.. image:: http://i.imgur.com/DxfBok2.png

.. image:: http://i.imgur.com/V4ZPCrT.png

Videos:

- AI, reaction fire and sectors (2016.06.08): https://youtu.be/hI6YmZeuZ3s
- transporter, roads (2016.08.07): https://youtu.be/_0_U-h1KCAE
- smoke, water and bridges (2016.08.20): https://youtu.be/WJHkuWwAb7A


Assets
------

Basic game assets are stored in a separate repo:
https://github.com/ozkriff/zoc_assets

Run ``make assets`` (or ``git clone https://github.com/ozkriff/zoc_assets assets``) to download them.


Building
--------

``make`` or ``cargo build``.


Running
-------

``make run`` or ``cargo run`` or ``./target/zoc``.


Android
-------

For instructions on setting up your environment see
https://github.com/tomaka/android-rs-glue#setting-up-your-environment.

Then just: ``make android_run`` - this will build .apk, install and run it.


License
-------

ZoC is distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See `LICENSE-APACHE`_ and `LICENSE-MIT`_ for details.


.. |license| image:: https://img.shields.io/badge/license-MIT_or_Apache_2.0-blue.svg
.. |loc| image:: https://tokei.rs/b1/github/ozkriff/zoc
.. |travis-ci| image:: https://travis-ci.org/ozkriff/zoc.svg?branch=master
.. |appveyor-ci| image:: https://ci.appveyor.com/api/projects/status/49kqaol7dlt2xrec/branch/master?svg=true
.. _`This Month in ZoC`: https://users.rust-lang.org/t/this-month-in-zone-of-control/6993
.. _Rust: https://rust-lang.org
.. _LICENSE-MIT: LICENSE-MIT
.. _LICENSE-APACHE: LICENSE-APACHE
.. _loc: https://github.com/Aaronepower/tokei
.. _travis-ci: https://travis-ci.org/ozkriff/zoc
.. _appveyor-ci: https://ci.appveyor.com/project/ozkriff/zoc
