TODO: переведи, когда закончишь

Частичные повреждения? Эффекты?

Хрен знает, как затолкат ьсистему эффектов в текущую архитектуру.
Я могу в каждое событие атаки добавить вектор эффектов, но это решение
далеко не универсальное:

* я все еще не могу одной атакой повредить несколько целей (большой взрыв)

* хотелось бы иметь возможность накладывать эффекты при любом событии
  Допустим, при движении по сложной местности техника может застрять на ход.


Можно было бы обойти все это, просто добавив событие CoreEvent::Effect и добавляя его
в симуляции в нужный момент, НО

- теряется семантическая связсь между событиями

  - как визуализатору отличить простое передвижение врага от бегства после атаки?
  - как визуализировать атаку, которая не уничтожила врага, но наложила эффект?
    ввести несколько результатов атаки (убил,промазал, эффект), последний из которых
    будет как-то активировать сложную логику "посмотри, какое событие следующее"?

Посему есть мысль преобразовать CoreEvent в

```
enum Event {
    Move(id, from, to},
    Attack(id, id},
    ...,
}

enum Time {
    Instant,
    Turns(u8),
    Forever,
}

struct TimedEffect {
    effect: Effect,
    time: Time,
}

enum Effect {
    Killed(u8),
    Suppressed,
    MoraleDown,
    ...,
}

struct CoreEvent {
    event: Event,
    effects: HashMap<UnitId, Vec<TimedEffect>>,
}
```

Агент - объект, который инициировал событие.
У события может и не быть агента (конец хода),
а может быть разу несколько агентов (теоретически).

Цель - объект, который является невольным участником события.
Цель может и одобрять событие (транспортер одобрит погрущку в него пехоты),
так и не одобрять (при обстреле вражеского отряда).

При применении эффекта в метод будет передано само событие, что бы был контекст.

Вопрос: где провести грань - что должно быть полем события, а что отдельным эффектом?
Первое что мне пока приходит в голову - к чему относится информация? К цели или к агенту?
Если к агенту, то пускай будет полем события, если к цели - эффект.
Вроде, разумно.

TODO: приведи примеры

Таким образом, визуализатор из события и эффектов сможет создать целостную,
логически связанную картину произошедшего.

------

// `State::apply_effect` - параметр event?

TODO: И вот тут возникает вопрос - что делать с событием?
если я соранил TimedEffect, то самого события у меня уже на руках нет.

Т.е. выходит что мгновенно применяемые эффекты могут полагаться на
данные из самого события, а вот отложенные эффекты должно быть
независимы ни от чего. Ну отлично.

С другой стороны - а где мне само событие нужно?
только в визуализаторе, потому что на уровне логики событие
отдельно применится к агенту.

------

Есть сложность с визуализатором - я не могу просто визуализировать событие атаки,
потому что не знаю, например, куда должен лететь снаряд.

Какой выход я могу найти? Переместить всю эту логику в визуализатор эффекта,
потому что у него есть доступ и к самому событию.
Но к какому из эффектов? Это у меня сейчас временный общий эффект от атаки,
а так их много должно быть разных сразу.

С другой стороны у визуализатора события может быть доступ и ко всем эффектам -
можно на основе эффектов и типа атаки чет делать. Хз.

Или в AttackInfo можно положить `target_pos: ExactPos`?
Этого хватит для начальной визуализации, а там разберемся.

-------

Вообще, это странный момент: как визуализировать событие атаки,
если оно из засады и я вообще не могу рисовать снаряд?

Может, надо как-то обозначать район, из которого "прилетело"?
В духе "случайно сдвинутый круг из 7 клеток,
из одной из которых и стреляли".

-------

Для нормального показа эффектов мне таки нужно разбить монолитные
визуализаторы осбытий на микродействия с узлами (найти номер задачи).

Как их реализовать?

Для начала, пусть в TacticalScreen поле
`event_visualizer: Option<Box<event_visualizer::EventVisualizer>>`
станет
`event_visualizers: Vec<Box<event_visualizer::EventVisualizer>>`.

Причем все события из ядра тоже должны получаться вектором,
а не по одной функции.

Т.е.
`fn get_event(&mut self) -> Option<CoreEvent>`
станет
`fn get_events(&mut self) -> Vec<CoreEvent>`.

-------

```
// TODO: вот это поле тоже надо обработать и втолкать в `event_visualizers`,
// ведь все это дело после показа тоже применять придется.
// Только учти что у эффектов отдельные визуализаторы.
event: Option<CoreEvent>,


fn logic(&mut self, context: &mut Context) {
    // TODO: переделать на вектор, эта логика совсем устарела.
    //
    // Что тут надо делать? Пробовать вытянуть из ядра события,
    // добавлять их визуализаторы в self.event_visualizers.
    // И, если event_visualizers до этого был пустым, начинать играть?
    //
    // Если текущий визуализатор закончился, то применить его событие-эффект
    // И убрать из вектора.
    //
    // Кстати, хороший вопрос - как мне применять всю эту хрень?
    // Если у меня используются для событий и эффектов одни EventVisualizer
    //
    // NOTE: если я буду много удалять из начала вектора, то, наверное
    // лучше взять тут VecDeque?
    //
    // if self.event_visualizer.is_none() {
    //     if let Some(event) = self.core.get_event() {
    //         self.start_event_visualization(context, event);
    //     }
    // } else if self.is_event_visualization_finished() {
    //     self.end_event_visualization(context);
    // }
}
```

------

I should replace all the MapText machinery with simple
Actions somehow.

------

Ok, I have a problem: when unit is created its NodeId is allocated
dynamically, but I need to know the NodeId to create a chain of
actions like `Create->Move` :-(

I still have an UnitId and can pass it to every action, but
this way my actions will be tied too closely to units
and I want Action to be useful for all SceneNodes.

In the new event-action architecture NodeId must be reserved somehow I think.

------

`pub struct ActionShowText {...}`

TODO: I need the camera's angle to make it work :-\ Context?
This ruins the idea of working with SceneGraph only :'-(

I can mark SceneNode as `Sprite` so that scene itself will rotate it.
Hmm... :(

-------

I'll try to stick with passing Context to Actoin's methods =\

I need it anyway to generat new mesh with text.
If I create a Mesh in ActionShowText::begin - where should i save it?
There's no access to specialized manager anymore.
Should I put in in the Action itself?

Ooops. I don't know how to make it work with SceneNodes:
SceneNode assumes that mesh is accesed with MeshId.
But if I generate and save Mesh inside the Action there will be
no MeshId for it :(

Can I put my generated mesh into MeshManager somehow?

------

One solution is to create a tmp struct like

```
pub struct NameMe<'a> {
    pub scene: &'a mut Scene,
    pub context: &'a mut Context,
    pub meshes: &'a mut MeshManager,
}
```

and pass it to every Action's method.
The downside is that whole Context is mutable for some reason =\

And, by the way, I have no idea how to name it.

------

Ok, next problem is transparacy.
Omg. I need real z-sorting of scene nodes.

...

Done. I've created three lists of NodeId: normal, transparent and planes.
Second list is resorted on every frame.


------

Now I need to employ ActionMove and ActionNodeRemove somehow.
I don't want to duplicate their logic in ActionText.
And the question is - do i really need ActionText?

------

I hate all this mutable references to Xxx everywhere!
Xxx contains all tactical screen state :-\

I want the creation of new actions to be fully declarative
and non-destroying.
But how can I allocate new `mesh_id` or `node_id` withut mutability?

I don't care much about `&mut Xxx` in Action::begin/update/end.

How can I get rid of the mut here?
This IDs are needed only to connect Actions.
Can I use something else to do it?..

------

TODO:

- before:
  - [x] fix smoke transparacy
    - forgot to set mesh to NoDepth! :(
  - [ ] fix FoW
    - [ ] Convert to Actions. How?
          Add some specialized actions? Like `FogTile`\`UnfogTile`?
          But first I need to implement similtanius actions
  - [ ] shadows
    - [x] basic
    - [ ] make them darker
    - [ ] I need to rework map creation in order to implement this properly
  - [ ] more crisp tile's border (redraw texture)
  - [ ] fork action
  - [ ] FIX RANDOM FREEZES during enemy's turn
  - [ ] fix text labels
    - [ ] rotate with camera
    - [ ] appear from alpha
    - [ ] fade to alpha
  - [ ] arc trajectory for mortar
  - [ ] change selecting ring size based on unit's size
  - [ ] change towing distance based on unit's size
  - [ ] smoke event -> smoke effect + shell visualization
  - [x] `new -> Box<Action>` -> `new -> Self`
  - [ ] remove all other new TODOs
  - [ ] check it still works on android
  - [ ] `git rm` this file
- separate commits for:
  - transparent node type?
  - effects-actions
  - shadows
  - visual unit sizes
- after (in separate branches):
  - src/screens/tactical/mod.rs
    - .../action/mod.rs
  - src/screens/main/mod.rs
  - src/screens/end_turn/mod.rs
  - make gui independant of screen's size
    - i need to get rid of the Size2 somehow
  - replace walk and attack lines with colored tile (like FoW)
  - update gfx
  - replace tree models!
  - rename `SceneNode` to just `Node`
  - logging
  - delayed textures loading
  - Add `prototype of strategic mode city-building modes` to roadmap
