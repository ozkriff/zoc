
(прим для себя: объясняй от базовых действий к более сложным.
что бы человек сразу мог что-то сделать.
сначала как вообще камру двигать, потом как купить отряды, потом про базовое передвижение и атаки,
потом уже всякие подробности про стоимость движения, шатрафы и т.п.)


# Руководство

## Главное меню

Всего три кнопки:

- текущая карта - по нажатию переключит на следующую
- кнопка "играть с ИИ"
- кнопка "играть с человеком на одном устройстве"

Почти все каты откровенно тестовые.


## Камера

Для перемещения камеры двигайте мышку с зажатой левой кнопкой.
Для вращения камеры двигайте мышку с зажатой левой кнопкой.
Для приближения (отдаления) камеры используйте клавиши "+" ("-") или кнопки `[+]` (`[-]`).

Прим: весь интерфейс временный и в последствие будет переделан

На андроиде перетаскивание курсора на левой половине отвечает
за движение камеры, на правой - за вращение.
(да-да, надо будет сделать стандартный мультитач)


## Подкрепления

В начале сценария нужно вызвать подкрепления, иначе просто не кем командовать.
Изначально у игроков есть 10 очков подкреплений и еще столько же будет добавляться каждый ход.
Подкрепления вызываются в секторах подкреплений, которые обозначены двумя кружками цвета игрока.

<картинка>

Нажимем на такую клетку, появляется базовое контекстное меню с одним доступным пунктом "подкрепления":

<картинка>

(что делать с цветами игроков? откуда новичку знать что он синий, а враг зеленый?)

По нажатию на этот пункт меня открывается меню с доступными для покупки отрядами.
Что бы отряд показывался в меню в секторе должно быть достаточно места для его размещения
и должно хватать очков подкреплений на его покупку.

При нажатии на один из вариантов отряд сразу появится на карте, но
действовать он сможет только на следующий ход.


## Окончание хода

Что бы закончить свой ход надо жать кнопку "[end turn]".

TODO: Когда рассказать про туман войны?
Вроде бы можно и отложить, но где-то надо упомнять что туман войны
обнуляется только в начале твоего хода.


## Выделение отрядов

При нажатии на клетку с интересующим отрядом появится контекстно меню с
вариантом `select <тип_юнита>`.

Наверху экрана появится полоска с краткими свойствами отряда
MP AP RAP M и т.п.


## Основы движения

При выделении отряда подсвечиваются доступные для передвижения в этот
ход клетки.

"move" - обычное движение

"hunt" - как обычное движение, только не дает штрафов к стрельбе,
дает шанс избежать реакционного огня (типа, проверка реакции)
очки движения не отнимаются (даже при стрельбе в отряд)
и в два раза дороже.


## Слоты

Основа игровой механики.

TODO: слоты? в какой момент о них стоит рассказать? Разбить на два этапа?

- обычные слоты
- на всю клетку
- воздушный слот

"обычные слоты" и "на всю клетку" конфликтуют:
если в клетке уже есть обычный отряд, то большая техника не сомжет въехать туда,
и наоборот - большая техника занимает сразу все обычные слоты.
Кстати, да, лучше говорить что большие отряды занимают прямо все обычные слоты.


## Атака

ОА - очки атаки

Оружие может иметь не только максимальную дальность стрельбы,
но и минимальную.

Тратится одно ОА.
Цель должна быть видима и в зоне дальности стрельбы оружия.
При нажатии на клетку, в которой находится цель, 
при условии что цель видима, не слищком далеко или близко и хватает ОА,
в контекстном меню появится пункт "`attack <тип врага> шанс_убить`".

Шанс попасть в врага

Шанс нанести урон

И тут должна быть всякая математика о вариантах последствия атаки.

При атаке может пострадать поевой дух противника (см. "Боевой дух")


## Реакционная атака

реакционные ОА (РОА)

Каждый ход отряду дается одно РОА плюс
неиспользованные за свой ход ОА превращаются в РОА.

На каждую атаку требуется одно РОА.

Атака происходит по триггерам в зоне видимости, к ним относятся:
- передвижение
- атака
- погрузка / выгрузка
- прицепление / отцепление


## Два радиуса видимости

TODO: скрытие пехоты в лесу, дыме и городе

У каждого отряда есть два радиуса видимости

``los_range`` - обычный радиус видимости

``cover_los_range`` - радиус видимости объектов в укрытиях.
Пехота в лесу или зданиях (где еще?) становится видимой только в этмо радиусе.

Или при атаке, но на следующий ход она пропадет из видимости.

[тут нужна схематичная иллюстрация]

Последний выше у разведчиков.

[не реализовано]
Оба радиуса видимости могут быть увеличины на один ход
командой "присмотреться", за счет всех (?) активных очков атаки отряда.


## Боевой дух

Изначально 100.
Каждый ход восстанавливается по 10 очков.
При каждой атаке у отряда отнимается столько очков БД,
какова была вероятность успешности атаки.
Падает ниже 50 - отряд считается подавленым
и теряет возможность атаковать.

TODO описать стандартную ситуацию с подавлением опасных клеток
перед пересечением открытой местности.


## Туман войны

Ставие невидимыми клетки убираются только в начале хода игрока.

TODO: склеить с "конец хода"?


## Дороги

Ускоряют передвижение большей части техники, особенно колесной.

Не распространяется на большую технику.

TODO: виды дорог? дорога в лесу?


## Перевозка

Пехота и полевые орудия могут сильно выиграть если их погрузить в грузовик.

Погрузка и выгрузка лишают пессажиров всех очков движения.
За один ход и то, и другое сделтаь не выйдет, потому что
для выгрузки нужны ОД.

ОА остаются, потому что они компенсируются реакционым огнем противника.


## Буксировка

полевые орудия и поврежденная техника могут сильно выиграть
если прицепить их к более быстрому или проходимому тягачу.

Полевые орудия передвигаются своим ходом еще меделнней пехоты,
так что их можно прикреплять к грузовикам.

Остовы техники тоже можно буксировать (что бы освободить проезд).

Есть требование что транспортер должен быть больше буксира (поле `size`).
Например, легкий танк не сможет тащить за собой тяжелый танк,
а джип не может буксировать полевое орудие.


## Вода и мосты

Назменые отряды не могут передвигаться по водным клеткам, если через нее нет моста.

Мост выглядит как простая дорога через водную клетку.
Имеет только один слот, это может быть важно для удержания врага на другом берегу.

Корабли, амфибии и передвижении легкой пехоты вплавь еще не реализованы.


## Виды городских клеток

Одно здание - движение техники почти не затруднено
Два здания - движение техники сильно затруднено
Три здания - техника не может двигаться
Большое здание - техника не может двигаться

Для защиты пехоты не имеет значения находится ли она внутри городской
клетки в здании или в уличном слоте. 
Важно что она просто в городской клетке какогото типа.


## Воздушные юниты

Вертолеты есть, самолетов и зениток еще нет.

Воздушные отряды не могут захватыать сектора.

Видят без "теней" от препятствий, но совсем не могут замечать пехоту в укрытиях,
пока та не начнет стрелять.


## Водная техника

[не реализована]


## Дымовая завеса

В данный момент только миномет может стрелять дымовыми снарядами.
Дым остается на несколько ходов.
Учти, что видимость пропадет только на следующий ход (когда обновится туман войны)


## Сокращения в интерфейсе и назначение кнопок

- `[<]` - выбрать прошлый отряд
- `[>]` - выбрать следующий отряд
- `[X]` - снять выделение

- AP - attack points
- RAP - reactive AP
- MP - move points
- M - morale


## Коротко про архитектуру приложения

TODO Команды, состояния, события и т.п.
TODO Полные и частичные состояния
TODO Адаптируй схемку их диплома.


======

[Может пригодиться попозже, в основном куски из руководства BA2]

The battlefield is divided up into tiles and important information
about the various terrain features (forests, village buildings, muddy
areas etc) represented on the map can be accessed by moving the game
cursor over each square.

Terrain features that are not within sight of
any of your units are depicted in grey. 

You can use the mouse wheel to zoom in and out of the
map and you can survey
the map by moving the
cursor to the appropriate
map edge.

Buildings, forests and hills can
block the line of sight of units
and enemy units in grey areas of
the map cannot be seen by you.

Smoke artillery – gives smoke cover for two or three turns
enabling your units to advance more safely towards enemy
positions. Smoke will block line of sight.

When the artillery symbol has “Ready” showing, left click on it
and a pink grid 7x7 squares large will appear. Move the grid to your
desired target area and left click again and an “Order artillery”
icon will appear. Left-click for the third time to order your artillery
barrage that will take place two turns later.

To move a unit you left-click on it. The tile that the unit is in is
highlighted by a green rectangle.
A small pop-up shows you the unit type, terrain type and the
cover given by the terrain, expressed as a percentage. All the tiles
that the unit may move into are
highlighted in green.

You also have the option
to order your unit to “Hold
Fire”, which will allow it to
remain hidden, if in terrain,
and ambush enemy units.

Infantry and self-propelled artillery units also have the capability
to use “Smoke” to hide their position from the enemy on a limited
number of occasions during a scenario.

You may set your units to “Hold Fire” if you do not want them
to engage the enemy. This may be important if you are planning
an ambush and you have moved your units into position without
being spotted by the enemy. Once your unit starts firing then it
is likely to be spotted by the enemy fairly quickly. Remaining
concealed and holding your fire until the enemy is in close range
is a very effective way of maximising the damage inflicted on
them. Any shots not used in your turn are saved to be used in
your opponents turn.

All units have the capability to use
“reaction fire” if an enemy moves
into their “line of sight”. Some units
are better at this than others because
of such attributes as poor viewing
devices, fixed guns and slow turrets. A unit will always have
one reaction shot, but if they have not fired in the previous turn
then they will able to fire all of
their shots. It is important to
remember that units are always
better at reacting to units that are
in front of them rather than at
their side or behind them.

5.2 Morale
Every time a unit is shot at by the enemy its morale will drop. When
the morale of a unit drops below 50 it will become “suppressed” 
and will not be able to fire at the enemy. When the morale of a unit
drops to 0, it will surrender if it is then shot at by an enemy in an
adjacent tile. Should the morale of a unit drop to – 100, then it will
permanently rout from the battle.
Morale will recover if a unit can go through a complete turn
without suffering further morale loss. So it can be judicious to
move “suppressed” units out of the front line in order to give them
time to recover.

Terrain is a very important consideration in Battle Academy 2.
Open ground is best for tanks and those units that need to move
quickly but it affords very little cover for infantry units moving on
foot. On the other hand, forests, buildings and fortifications give
very good cover to infantry units but are impassable to all vehicles.
Vehicles may enter terrain such as high vegetation, rough ground
and marshes to avail themselves of the extra cover these provide
but they will suffer movement penalties in the process.

Infantry units that come under enemy fire when moving will
immediately stop and may be stranded in the open as a result.

Units which come under fire will have their morale eroded. If
morale drops below 50, then units will not react to nor be able to fire
at the enemy. Suppressing the enemy can be vital before advancing
across open ground. Note that armour will still react to being
attacked, even when suppressed, but with large penalties to accuracy.
If you use suppression fire intelligently you can prevent enemy
units from recovering their morale.

receives 2 gold pieces per turn

Core game features are:

- advanced fog of war
- slot system (multiple units per tile)
- reaction fire (xcom-like)
- morale and suppression
