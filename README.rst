
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

Player's objective is to capture and hold control zones for certain number of turns.

Terrain types:

- Plain
- Trees
- Water
- Road/Bridge
- City

Unit types:

- Infantry - weak, but can use terrain like Trees or City to get a defence bonus and hide from enemies; can be transported by trucks. Types:

  - rifleman - basic infantry type, 4 soldiers in a squad;
  - smg - more deadly on short distances, less deadly on full range, 3 soldiers in a squad;
  - scout - weak, but have advances visibility range and can better detect hidden enemies, 2 soldiers in a squad;
  - mortar - defenceless, but can shoot smokescreen rounds, slow;
  - field gun - effective against vehicles, slow and can't be transported inside of track, but can be _towed_;

- Vehicles - can't hide in terrain, can't occupy buildings. Can't see hidden infantry.  Leave a wreck when destroyed. Can take in a tow vehicle or wrecks lighter than themselves. Types:

  - jeep - fast and effective against infantry and helicopters;
  - truck - can transport infantry;
  - light tank
  - light self-propelled gun - has an armor of a light tank, but a gun of medium tank;
  - medium tank
  - heavy tank
  - mammoth tank

- Aircrafts - can fly above all terrain features; it's line of sight isn't blocked by terrain. Only one type was implemented:
  - Helicopter 

Morale/Suppression system:

- every unit initially have 100 morale points and restore 10 points every turn
- morale is reduced by half a a damage chance (hit chance / armor protection) when a unit is attacked even if attack missed;
- if a soldier of the squad is killed additional suppression is added
- if a unit's morale falls below 50, then it's suppressed and can't attack anymore

------

Videos:

- Some playtest (recorded in 2019, but uses a game build from 2017): https://youtu.be/3_ZPtwnMQVU
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
