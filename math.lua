#!/usr/bin/lua

--[[
Если разбивать юниты на классы (технику и пехоту, как минимум),
то встает вопрос о том, как мне это хранить в игре.
Ведь тогда я не смогу иметь просто массив Vec<Unit>, потому что
в таком массиве у структур должны бы быть поля разных типов.

Выходом тут может послужить компонентный подход.
Или тупо хранение структур в нескольких массивах
- Vec<InfantryUnits>, Vec<VehicleUnits>.
Хотя это довольно близкие штуки.

BA2:

бронирвание:
- т34-41 - [75, 50, ?, 30]
- т34-43 - [85, 60, 60, 30]
- т70 - [65, 50, 50, 20]
- пантера - [150, 60, 60, 30]
- тигр - [120, 85, 85, 30] ??
- нашхорн - [20, 20, 5, 30]
- pz4 g - [80, 50, 40, 20]

- т34-41 -> бронетранспортер
  бронетранспортер двигался
  маленькая цель
  - далеко - 9%
  - средне
    - 16%
    - 26% (второй выстрел)

- т34-41 -> pz4g
  в зад
  ловкая цель
  - средне (5-7 клетокна глаз)
  - шанс попасть = 23%
  - шанс пробить  = 79%
  - итого шанс убить = 18%

- т34 -> тяж брон машина разведки (sd.kfz-231-8-rad)
  ...

- т70 -> разведчики
  на равнине
  he effectiveness = 9

- т70 -> бронетранспортер
  7 клеток
  после быстрого движения
  шанс попасть
      - 12%
      - 22% (повторно)
  шанс пробить - 95%

- су-152 -> тигр
  в лоб
  6 клеток
  шанс попасть - 40%
  шанс пробить - 36%

- су-152 -> pz4g
  в лоб
  6 клеток
  шанс попасть - 37%
  шанс пробить - 64%

- су-76 -> пехотинцы
  равнина
  с закрытой позиции
  далеко
  сила воздействия - 35%
  убило двоих

- т34-41 -> тигра
  далеко (7+ клеток)
  бонс за большую цель
  ловкая цель
  шанс попасть - 17%
  шанс пробить - 0%

- пт-ружье -> бронетранспортер
  маленькая и ловкая цель
  - 3 клетки
    шанс попасть - 24%
    шанс пробить - 100%
  - 2 клетки
    - из засады
      шанс попасть - 28%
      шанс пробить - 100%
    - повторный выстрел, обнаружен
      шанс попасть - 18%
      шанс пробить - 100%

- стационарный пулемет -> пехота
  у пехоты 4 из 5ти человек
  быстро бегут
  дорога
  3 клетки
  шанс первого убийства - 60% (т.е. ~25% для каждого в среднем)

- стрелки(винтовки) -> пехота
  у пехоты 4 из 5ти человек
  быстро бегут
  дорога
  3 клетки
  шанс первого убийства - 25%  (т.е. ~11% для каждого в среднем)

- т34-41 -> nashhorn
  в бок
  большая цель
  ловкая(?) цель
  - средне (5-7 на глаз)
    - 22%
    - 32% - второй выстрел

- миномет-82мм -> пехота
  в чистом поле
  убил двоих и подавил отряд
]]

unit_classes = {
    'vehicle',
    'infantry',
}


terrain_types = {
    forest = {moving = 40, not_moving = 70},
    open_ground = {moving = 0, not_moving = 60},
    road = {moving = 0, not_moving = 30},

    -- можно спрятаться
    craters = {moving = 0, not_moving = 55},

    rough_ground = {moving = 25, not_moving = 70},

    -- Трава не блокирует видимость!
    high_vegetation = {moving = 0, not_moving = 70},

    light_fortifications = {moving = 60, not_moving = 75},
    heavy_fortifications = {moving = 75, not_moving = 85},
    light_building = {moving = 50, not_moving = 75},
    building = {moving = 50, not_moving = 75},

    -- дает доп. укрытие в направлении
    trench = {
        moving = 0,
        not_moving = 70,
        directional_cover = 35, -- !
    },

    -- непроходимо для пехоты
    barbed_wires = {moving = 0, not_moving = 60},
}


--[[
    вот пушка может стрелять разными снарядами - как это моделировать?
    для начала, наверное, пускай в свойствах пушки прям все сразу и будет напихано:
    будет два поля - ap и he. Или soft и hard, как в той стратежке в андроиде.
    
    как минимум, остается вопрос про кинетические AP снаряды и кумулятивные
    последних, наверное, всего парочка должна быть

    а в серии wargame у оружия есть:
    - range
      - ground
      - helicopters
      - airplanes
    - accuracy
    - ap_power
    - he_power
    - suppression
    - rate_of_fire - это меня мало интересует))
    
    какое-то количество тэгов, меняющих огику работы оружия;
    тип снарядов;
    
    тэги:

    - [AoE]
      Area of Effect - This weapon fires anti-personnel explosive rounds.
      Its HE value applies over an area of effect.
    
    - [CQC]
      Denotes an infantry machine gun that can be used in close-quarters
      combat and on the move, unlike most infantry machine guns,
      which have the [STAT] tag.
    
    - [CORR]
      Corrected Shot - This weapon may provide indirect fire above
      obstacles. It may improve accuracy if a friendly unit has a
      direct line of sight on the target. Only artillery units have this tag.
    
    - [DEF]
      Anti-missile Defense - This weapon will target enemy missiles within
      a limited range. It will automatically fire and attempt to destroy
      them in flight.
    
    - [F&F]
      Fire & Forget - Once fired, this missile doesn’t require any more
      action from the operator. Note that all guns and unguided shells
      are fire-and-forget, so this tag only applies to missiles. This
      is as opposed to [GUID].
    
    - [GUID]
      Guided - This missile is guided. Its operator needs to stand still
      and aim at the target until the impact. This is as opposed to [F&F].
    
    - [HEAT]
      High Explosive Anti-Tank - This weapon fires anti-armor chemical
      rounds. Its AP value will remain the same whatever the range
      to the target. This is as opposed to [KE].
    
    - [NPLM]
      Napalm - This weapon uses napalm. It is likely to start fires
      in woods or buildings, but is also a terror weapon affecting the
      target’s morale within an area of effect. Note that this tag is
      generic to all incendiary weapons even if they are not
      technically napalm-based in reality.
    
    - [KE]
      Kinetic - This weapon fires anti-armor kinetic rounds. The closer
      it gets to its target, the higher its AP value will rise. This
      is very important to know - weapons with the [KE] tag, including
      most autocannons and tank cannons, will do significantly higher
      damage close to the target. This means that even very weak guns
      will do good damage in tight quarters.
    
    - [RAD]
      Radar - This weapon uses radar guidance, making it vulnerable to
      anti-radar missiles. Turning the weapon off will avoid this threat.
    
    - [SA]
      Semi-Active - This missile is guided but can be fired on the move.
      Its operator needs to aim at the target until the impact. This is
      sort of a halfway between the [GUID] and [F&F] tags. 
    
    - [SEAD]
      Suppression of Enemy Air Defenses - This anti-radiation missile will
      lock on enemy radar and be guided on them as long as they remain
      active. These weapons tend to fire automatically at the first radar
      they see.
    
    - [SHIP]
      Anti-ship - This weapon may only target naval units. It is designed
      to effectively destroy ships, and only ships.
    
    - [SMK]
      Smoke - This weapon fires smoke rounds. Smoke screens don’t deal any
      damage, but block any ground unit’s line of sight. Only artillery
      units have this ability.
    
    - [STAT]
      Stationary - This weapon can’t be fired on the move.
]]--


-- про урон морали: в ba2 у отрядов есть параметр he_suppression
weapon_types = {
    light_cannon = {
        accuracy = {9, 8, 7, 6, 5, 4, 3, 2, 1},
        he = 8,
        ap = {8, 8, 8, 7, 7, 6, 6, 5, 5},
        max_distance = 9, -- это поле вообще нужно? его можно заменить на #accuracy
    },
    cannon = {
        ap = 11,
        he = 11,
        accuracy = {9, 8, 7, 6, 5, 4, 3, 2, 1},
        max_distance = 9,
    },
    heavy_cannon = {
        ap = 15,
        he = 15,
        accuracy = {9, 8, 7, 6, 5, 4, 3, 2, 1},
        max_distance = 9,
    },
    rifle = {
        ap = 1,
        he = 2,
        accuracy = {9, 8, 4, 2, 1},
        max_distance = 5,
    },
    heavy_machine_gun = {
        ap = 1,
        he = 4,
        accuracy = {9, 8, 6, 4, 3, 2, 1},
        max_distance = 7,
    },
    -- легкий пулемет
    -- огнемет
    -- пистолет-пулемет
    -- штурмовая винтовка
}


-- разброс количества очков атаки у танков - 1-3.
-- отражает их скорострельность.
-- Тащемта, аналогично с BA, только у быстрых танков три очка атаки.
-- а у прокачанных, получается, будет аж четыре + пассивное.

-- как моделировать пехоту с базукой? так же, видимо, как танк с пушкой
-- и пулеметами?

-- подумать над юнитами с несколькими оружиями, которые могут вести огонь
-- одновременно. Типа т-35.

-- armor: {'frontal', 'side', 'rear', 'top'}
-- хотя пока забиваю на верхнюю броню

unit_types = {
    heavy_tank = {
        class = 'vehicle',
        weapon_type_id = 'heavy_cannon',
        size = 12, -- target_size_accuracy_factor
        armor = {15, 10, 7},
        moving_accuracy_penalty = 40,
    },
    tank = {
        class = 'vehicle',
        weapon_type_id = 'cannon',
        size = 10,
        armor = {11, 8, 5},
        moving_accuracy_penalty = 50,
    },
    light_tank = {
        class = 'vehicle',
        weapon_type_id = 'light_cannon',
        size = 8,
        armor = {7, 5, 3},
        moving_accuracy_penalty = 70,
    },
    soldier = {
        class = 'infantry',
        weapon_type_id = 'rifle',
        size = 4, -- уменьшить до 3 при залегании и до 1-2 при наличии хороших укрытий
        armor = 1,

        moving_accuracy_penalty = 50,

        -- индивидуальные свойства пехоты:
        count = 4,
        toughness = 2,
    },
}


-- какие вообще могут быть последствия стрельбы? т.е. что может храниться в событии Attack?
--
-- для артиллерии - надо сделать так, что бы можно было убивать отдельных
-- членов рассчета и это накладывало какие-то там ограничения на ее работу.
-- типа, убиваешь наводчика - уменьшается точность.
--
-- для техники:
--   - 1 - промах
--   - 2 - не пробил, без последствий
--   - 3 - легкие повреждения
--     - замедлен
--     - сбита гусеница
--     - уменьшилась видимость
--   - 4 - тяжелые повреждения
--     - пушка сломалась
--     - уменьшилось количество очков атаки (считай контузия экипажа или подобное)
--     - заклинило башню
--   - 5 - уничтожен
--   
--   вероятность последствий выводится из вероятности пробития.
--   типа, если шанс уничтожить - 20%, то тяж повреждения - 30%, легкие - 40%
--
-- для пехоты:
--   - сколько человек убито
--   - ээээ, хз что еще)

-- TODO: переименовать. это же не только тест на поадание, тут много всего
-- calculate_chances?
function hit_test(cfg)
    local target = cfg.target
    local attacker = cfg.attacker
    local world = cfg.world
    local target_type = unit_types[target.type_id]
    local attacker_type = unit_types[attacker.type_id]
    local target_class = target_type.class
    local weapon = weapon_types[attacker_type.weapon_type_id]
    if target_class == 'vehicle' then
        -- TODO: учесть размер
        
        --[[
            такс, вот как это происходит в ba2:
            - находится точность AP для дальности
            - затем из нее вычетается процент защиты от местности, с учетом
              того, двигалась ли цель или нет (тут надо немного прояснить механику ba).
            - если есть дым, то тут он снижает вероятность попадения
            - вероятность попадания домнажается на размер цели
            - если атакующий двигался, то вероятность домножается на moving_accuracy_penalty 
        ]]--

        local accuracy = weapon.accuracy[world.distance]

        local penetrate_chance =
            20
            + (weapon.ap - target_type.armor[world.angle_index]) * 10
        return {
            hit_chance = accuracy * 10 / 2,
            kill_chance = penetrate_chance,
        }
    elseif target_class == 'infantry' then

        --[[
            такс, вот как это происходит в ba2:
            - находится точность HE для дальности
            - добавляется бонус за стояние на месте, если можно
            - находится среднее из HE_attack
            - начальный шанс убить = средняя мощность атаки умножить на точность
            - умножается но силу атакующих (процент живых)
            - умножается на модификатор защиты местности
              (в зависимости от того, двигался враг или нет)
            - домножается на сил упротивника (чем больше мертво, тем больше 100%)
            - еще чего-то про combined modifiers, о это я уже не понимаю
        ]]--

        return {
            hit_chance = weapon.accuracy[world.distance] * 10 / 2,
            kill_chance = 100 -- жалкие человечишки всегда умирают
            -- хотяяяя, у меня ж не только обычные люди могут быть
        }
    else
        sys.exit('bad target class: {}' % target_class)
    end
end

function run_test(cfg)
    local result = hit_test(cfg)
    print(
        '[' ..
        cfg.name ..
        '] hit: ' ..
        result.hit_chance ..
        '%, kill: ' ..
        result.kill_chance ..
        '%'
    )
end

run_test {
    name = 'tank -> tank, простой',
    attacker = {type_id = 'tank'},
    target = {type_id = 'tank'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 1,
    },
}


run_test {
    name = 'tank -> tank, сбоку',
    attacker = {type_id = 'tank'},
    target = {type_id = 'tank'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 2,
    },
}


run_test {
    name = 'tank -> heavy_tank, простой',
    attacker = {type_id = 'tank'},
    target = {type_id = 'heavy_tank'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 1,
    },
}


run_test {
    name = 'tank -> heavy_tank, в борт',
    attacker = {type_id = 'tank'},
    target = {type_id = 'heavy_tank'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 2,
    },
}


run_test {
    name = 'tank -> heavy_tank, в зад',
    attacker = {type_id = 'tank'},
    target = {type_id = 'heavy_tank'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 3,
    },
}


run_test {
    name = 'heavy_tank -> tank, простой',
    attacker = {type_id = 'heavy_tank'},
    target = {type_id = 'tank'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 1,
    },
}


run_test {
    name = 'tank -> soldier, простой',
    attacker = {type_id = 'tank'},
    target = {type_id = 'soldier'},
    world = {
        distance = 1,
        terrain = 'open_ground',
        angle_index = 1,
    },
}


-- function main()
--     test_01()
--     test_02()
-- end


-- main()

-- vim: set tabstop=4 shiftwidth=4 softtabstop=4 expandtab:
