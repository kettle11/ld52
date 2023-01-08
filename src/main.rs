#![feature(drain_filter)]

use koi3::*;
mod rapier_integration;
use rapier2d::prelude::*;
use rapier_integration::*;
mod temporary;
use koi_graphics_context::BlendFactor;
use temporary::*;
mod ui;
use ui::*;

struct LevelState {
    pitch_multiplier: f32,
    aiming: bool,
    ready_to_shoot: bool,
    collected_pegs: Vec<Entity>,
    other_world: World,
    in_shop: bool,
    screen_shake_amount: f32,
    effects_to_apply_to_next_ball: Vec<Effects>,
    victory: bool,
    fired_once: bool,
    multiplier: f32,
}

#[derive(Clone, PartialEq, Debug)]
enum Effects {
    BigBall,
    RockStorm,
    RocksToGold,
    SeedStorm,
    MultiBallStorm,
    MultiplierStorm,
    RockWall,
}

const BIG_BALL: Powerup = Powerup {
    cost: 10,
    description: "2x Size",
    effect: Effects::BigBall,
};
const ROCK_STORM: Powerup = Powerup {
    cost: 10,
    description: "Rock Storm",
    effect: Effects::RockStorm,
};

const SEED_FRENZY: Powerup = Powerup {
    cost: 40,
    description: "Plant Seeds",
    effect: Effects::SeedStorm,
};
const ROCKS_TO_GOLD: Powerup = Powerup {
    cost: 100,
    description: "Rocks To Gold",
    effect: Effects::RocksToGold,
};

const MULTIBALL_STORM: Powerup = Powerup {
    cost: 20,
    description: "Multiballs!",
    effect: Effects::MultiBallStorm,
};

const MULTIPLIER_STORM: Powerup = Powerup {
    cost: 50,
    description: "Multipliers!",
    effect: Effects::MultiplierStorm,
};

const ROCK_WALL: Powerup = Powerup {
    cost: 20,
    description: "Rock Walls!",
    effect: Effects::RockWall,
};

const POWERUPS: [Powerup; 7] = [
    BIG_BALL,
    ROCK_STORM,
    ROCKS_TO_GOLD,
    SEED_FRENZY,
    MULTIBALL_STORM,
    MULTIPLIER_STORM,
    ROCK_WALL,
];

struct MouseFocalPoint;

fn apply_rock_storm(
    world: &mut World,
    resources: &mut Resources,
    peg_type: PegType,
    turn_rate: f32,
    radius_rate: f32,
    count: usize,
    center: Vec2,
    start_radius: f32,
) {
    let mut random = Random::new();
    let mut angle = random.f32() * std::f32::consts::TAU;
    let mut radius = start_radius;

    let mut time_offset = 0.1;

    for _ in 0..count {
        angle += std::f32::consts::TAU * turn_rate / (radius / 30.0);
        radius += radius_rate;

        let (sin, cos) = angle.sin_cos();
        let position = Vec2::new(cos, sin) * radius + center;

        let peg_type = peg_type.clone();
        world.spawn((DelayedAction::new(
            move |world, resources| {
                spawn_peg(world, resources, position.xy(), peg_type.clone());
            },
            time_offset,
        ),));
        time_offset += 0.05;
    }
}

pub fn apply_rocks_to_gold(world: &mut World, resources: &mut Resources) {
    let mut rocks = Vec::new();
    for (e, p) in world.query::<&Peg>().iter() {
        match p.peg_type {
            PegType::Stone => rocks.push(e),
            _ => {}
        }
    }

    let mut time_offset = 0.1;
    for rock in rocks {
        let position = world.get::<&Transform>(rock).unwrap().position;
        world.spawn((DelayedAction::new(
            move |world, resources| {
                let _ = world.despawn(rock);
            },
            time_offset,
        ),));
        world.spawn((DelayedAction::new(
            move |world, resources| {
                spawn_peg(world, resources, position.xy(), PegType::Gold);
            },
            time_offset + 0.02,
        ),));

        time_offset += 0.02;
    }
}

fn create_rock_wall(world: &mut World) {
    let mut time_offset = 0.1;

    for i in 0..30 {
        let position = Vec2::new(-65.0, -70.0 + i as f32 * 5.5);

        world.spawn((DelayedAction::new(
            move |world, resources| {
                spawn_peg(world, resources, position.xy(), PegType::Stone);
            },
            time_offset,
        ),));
        time_offset += 0.05;
    }

    for i in 0..30 {
        let position = Vec2::new(65.0, -70.0 + i as f32 * 5.5);

        world.spawn((DelayedAction::new(
            move |world, resources| {
                spawn_peg(world, resources, position.xy(), PegType::Stone);
            },
            time_offset,
        ),));
        time_offset += 0.05;
    }
}

const PLANT_SEGMENT_LENGTH: f32 = 8.0;
impl LevelState {
    pub fn new(other_world: World) -> Self {
        Self {
            pitch_multiplier: 1.0,
            aiming: true,
            ready_to_shoot: true,
            collected_pegs: Vec::new(),
            other_world,
            in_shop: false,
            screen_shake_amount: 0.0,
            effects_to_apply_to_next_ball: Vec::new(),
            victory: false,
            fired_once: false,
            multiplier: 1.0,
        }
    }

    pub fn toggle_shop(&mut self, world: &mut World, resources: &mut Resources) {
        self.in_shop = !self.in_shop;
        std::mem::swap(world, &mut self.other_world);

        if !self.in_shop {
            for effect in self
                .effects_to_apply_to_next_ball
                .drain_filter(|f| match f {
                    Effects::RockStorm
                    | Effects::RocksToGold
                    | Effects::SeedStorm
                    | Effects::MultiBallStorm
                    | Effects::MultiplierStorm
                    | Effects::RockWall => true,
                    _ => false,
                })
            {
                match effect {
                    Effects::BigBall => {}
                    Effects::RockWall => create_rock_wall(world),
                    Effects::RockStorm => {
                        let mut random = Random::new();
                        let center =
                            Vec2::new(random.range_f32(-30.0..30.0), random.range_f32(-40.0..30.));
                        apply_rock_storm(
                            world,
                            resources,
                            PegType::Stone,
                            0.05,
                            1.5,
                            20,
                            center,
                            10.0,
                        )
                    }
                    Effects::MultiBallStorm => {
                        let mut random = Random::new();
                        let center =
                            Vec2::new(random.range_f32(-30.0..30.0), random.range_f32(-40.0..30.));
                        apply_rock_storm(
                            world,
                            resources,
                            PegType::MultiBall,
                            Random::new().range_f32(0.02..0.1),
                            Random::new().range_f32(1.0..4.0),
                            Random::new().range_u32(3..10) as _,
                            center,
                            10.0,
                        )
                    }
                    Effects::RocksToGold => apply_rocks_to_gold(world, resources),
                    Effects::SeedStorm => {
                        let mut random = Random::new();
                        let center =
                            Vec2::new(random.range_f32(-30.0..30.0), random.range_f32(-40.0..0.));
                        apply_rock_storm(
                            world,
                            resources,
                            PegType::GrowablePlant,
                            0.2,
                            3.0,
                            2,
                            center,
                            10.0,
                        )
                    }
                    Effects::MultiplierStorm => {
                        let mut random = Random::new();
                        let center =
                            Vec2::new(random.range_f32(-30.0..30.0), random.range_f32(-40.0..40.0));
                        apply_rock_storm(
                            world,
                            resources,
                            PegType::Multiplier,
                            0.2,
                            3.0,
                            2,
                            center,
                            10.0,
                        )
                    }
                }
            }
        }
    }

    pub fn prepare_to_shoot(&mut self, world: &mut World) -> usize {
        let mut new_gold = 0;
        if !self.in_shop {
            self.ready_to_shoot = true;
            self.pitch_multiplier = 1.0;

            // TODO: Make this more satisfying
            let mut time_offset = 5;
            let mut len_remaining = self.collected_pegs.len();

            for entity in self.collected_pegs.drain(..) {
                if let Ok(peg) = world.get::<&Peg>(entity) {
                    match peg.peg_type {
                        PegType::Gold => new_gold += 20,
                        PegType::Plant => new_gold += 1,
                        _ => {}
                    }
                }

                len_remaining -= 1;
                // Destroy collected pegs.
                let _ = world.insert_one(entity, Temporary(time_offset));

                if len_remaining > 10 {
                    time_offset += 2;
                } else {
                    time_offset += 4;
                }

                // Grow a child plant
                let should_spawn_plant = world.get::<&Plant>(entity).map(|p| {
                    let peg_transform = world.get::<&Transform>(entity).unwrap();
                    *peg_transform
                });

                if let Ok(transform) = should_spawn_plant {
                    let p = transform.position.xy();

                    fn plant_segment(
                        world: &mut World,
                        resources: &mut Resources,
                        position: Vec2,
                        stem_direction: Vec2,
                        mut energy: usize,
                        skip_stem: bool,
                        mut gold_energy: usize,
                    ) {
                        if !skip_stem {
                            if energy == 1 && gold_energy == 1 {
                                spawn_gold(
                                    world,
                                    resources,
                                    position,
                                    stem_direction * PLANT_SEGMENT_LENGTH,
                                );
                            } else {
                                if gold_energy > 0 && Random::new().f32() > 0.8 {
                                    spawn_gold(
                                        world,
                                        resources,
                                        position,
                                        stem_direction * PLANT_SEGMENT_LENGTH,
                                    );
                                    gold_energy -= 1;
                                } else {
                                    spawn_plant(
                                        world,
                                        resources,
                                        position,
                                        stem_direction * PLANT_SEGMENT_LENGTH,
                                    );
                                }
                            }
                        }
                        energy = energy.saturating_sub(1);
                        if energy > 0 {
                            world.spawn((DelayedAction::new(
                                move |world, resources| {
                                    let mut segment_count = 1;
                                    let mut energy_in_segments = [energy, 0];

                                    if Random::new().f32() > 0.8 {
                                        segment_count += 1;
                                        let transfer = Random::new().range_u32(1..energy as _);
                                        energy_in_segments[0] -= transfer as usize;
                                        energy_in_segments[1] += transfer as usize;
                                    }

                                    for i in 0..segment_count {
                                        let range = std::f32::consts::PI * 0.4;
                                        let rotation =
                                            range * -1.0 + Random::new().f32() * range * 2.0;

                                        let rotation = Quat::from_angle_axis(rotation, Vec3::Z);
                                        let new_random_dir = rotation
                                            .rotate_vector3(stem_direction.extend(0.0))
                                            .xy();

                                        let position =
                                            position + new_random_dir * PLANT_SEGMENT_LENGTH;

                                        plant_segment(
                                            world,
                                            resources,
                                            position,
                                            new_random_dir,
                                            energy_in_segments[i],
                                            false,
                                            gold_energy,
                                        );
                                    }
                                },
                                0.2,
                            ),));
                        }
                    }

                    let range = std::f32::consts::PI * 0.5;
                    let rotation = range * -1.0 + Random::new().f32() * range * 2.0;

                    let rotation = Quat::from_angle_axis(rotation, Vec3::Z);
                    let new_random_dir = rotation.rotate_vector3(Vec2::Y.extend(0.0)).xy();

                    world.spawn((DelayedAction::new(
                        move |world, resources| {
                            plant_segment(
                                world,
                                resources,
                                p,
                                new_random_dir,
                                Random::new().range_u32(3..20) as _,
                                true,
                                3,
                            );
                        },
                        0.01,
                    ),));
                }
            }
        }
        new_gold
    }
}

struct Scale {
    rate: f32,
    t: f32,
    t_max: f32,
    max_scale: f32,
}

struct Eye {
    art: Entity,
    radius: f32,
    range: f32,
    other_eye: Option<Entity>,
}

struct GameAssets {
    stem_material: Handle<Material>,
    growable_plant_material: PegMaterial,
    plant_material: PegMaterial,
    gold_material: PegMaterial,
    stone_material: PegMaterial,
    multiball_material: PegMaterial,
    multiplier_material: PegMaterial,

    //
    brick_material: Handle<Material>,
    //
    ball_material: Handle<Material>,
    //
    plus_one: Handle<Material>,
    plus_twenty: Handle<Material>,
    x2: Handle<Material>,
}

struct PegMaterial {
    base: Handle<Material>,
    glowing: Handle<Material>,
    shockwave: Handle<Material>,
}

const SHOT_POWER: f32 = 80.0;

struct EyeFocalPoint;

struct Plant {
    last_direction: usize,
}

fn run_eyes(world: &mut World, resources: &Resources) {
    // TODO: The eyes wiggle because this isn't calculated from their center.
    // TODO: Make eyes look together.

    for (_, (eye_transform, eye)) in world.query::<(&GlobalTransform, &Eye)>().iter() {
        let mut closest_d = f32::MAX;
        let mut target_p = None;

        let mut center = eye_transform.position.xy();

        // Keep both eyes looking the same direction
        if let Some(other_eye) = eye.other_eye {
            let other_p = world
                .get::<&GlobalTransform>(other_eye)
                .unwrap()
                .position
                .xy();
            center = (center + other_p) / 2.0;
        }
        for (_, (ball_transform, ..)) in world
            .query::<(&mut GlobalTransform, &mut EyeFocalPoint)>()
            .iter()
        {
            let direction = ball_transform.position.xy() - center;
            let d = direction.length_squared();
            if d < closest_d {
                closest_d = d;
                target_p = Some(ball_transform.position.xy());
            }
        }

        if let Some(target_p) = target_p {
            let dir = (eye_transform.position.xy() - target_p).normalized();
            if closest_d < (eye.range * eye.range) {
                let mut t = world.get::<&mut Transform>(eye.art).unwrap();
                let z_before = t.position.z;

                let target = -dir.extend(z_before) * eye.radius;
                t.position = t.position.lerp(target, 0.3);
            }
        }
    }
}

fn run_scale(world: &mut World, resources: &Resources) {
    let time = resources.get::<Time>();
    let delta = time.draw_delta_seconds as f32;
    for (_, (transform, scale)) in world.query_mut::<(&mut Transform, &mut Scale)>() {
        scale.t += delta * scale.rate;
        scale.t = f32::clamp(scale.t, 0.0, scale.t_max);
        transform.scale = Vec3::fill(animation_curves::smooth_step(scale.t) * scale.max_scale);
    }
}

struct MainCamera;

fn main() {
    App::default()
        .with_resource(InitialSettings {
            window_width: 1600,
            window_height: 1200,
            ..Default::default()
        })
        .setup_and_run(|world, resources| {
            let rapier_integration = rapier_integration::RapierIntegration::new();

            let view_height = 150.0;

            let child_camera = world.spawn((
                Transform::new(),
                Camera {
                    clear_color: Some(Color::BLACK),
                    exposure: Exposure::EV100(6.0),
                    projection_mode: ProjectionMode::Orthographic {
                        height: view_height,
                        z_near: -2.0,
                        z_far: 2.0,
                    },
                    ..Default::default()
                },
                MainCamera,
            ));
            let parent = world.spawn((Transform::new().with_position(Vec3::Z * 2.0),));
            let _ = world.set_parent(parent, child_camera);

            let top_of_screen = Vec3::new(0.0, view_height / 2.0 * 0.75, 0.0);

            // Spawn the background
            {
                let new_texture = resources.get::<AssetStore<Texture>>().load(
                    "assets/BackgroundWide.png",
                    koi_graphics_context::TextureSettings::default(),
                );

                let new_material = resources.get::<AssetStore<Material>>().add(Material {
                    shader: Shader::UNLIT,
                    base_color_texture: Some(new_texture),
                    ..Default::default()
                });

                world.spawn((
                    Transform::new().with_scale(Vec3::new(160.0 * 2.0, 160.0, 1.0)),
                    Mesh::VERTICAL_QUAD,
                    new_material,
                ));
            }

            let stem_material = resources.get::<AssetStore<Material>>().add(Material {
                shader: Shader::UNLIT,
                base_color: Color::from_srgb_hex(0x489B41, 1.0)
                    .with_lightness(0.7)
                    .with_chroma(0.3),
                ..Default::default()
            });

            let recolor_shader = resources.get::<AssetStore<Shader>>().load(
                "assets/custom_shader.glsl",
                ShaderSettings {
                    blending: Some((BlendFactor::One, BlendFactor::OneMinusSourceAlpha)),
                    ..Default::default()
                },
            );

            fn get_texture_material(
                path: &str,
                resources: &Resources,
                shader: Handle<Shader>,
                color: Color,
            ) -> Handle<Material> {
                let texture = resources
                    .get::<AssetStore<Texture>>()
                    .load(path, koi_graphics_context::TextureSettings::default());
                resources.get::<AssetStore<Material>>().add(Material {
                    shader,
                    base_color_texture: Some(texture),
                    base_color: color,
                    ..Default::default()
                })
            }

            fn load_peg_material(
                resources: &mut Resources,
                shader: &Handle<Shader>,
                color: Color,
            ) -> PegMaterial {
                PegMaterial {
                    glowing: get_texture_material(
                        "assets/PegGlowing.png",
                        resources,
                        shader.clone(),
                        color,
                    ),
                    shockwave: get_texture_material(
                        "assets/PegShockWave.png",
                        resources,
                        shader.clone(),
                        color,
                    ),
                    base: get_texture_material("assets/Peg.png", resources, shader.clone(), color),
                }
            }

            let mut ui = ui::UI::new(world, resources);

            let spawn_witch = |world: &mut World| {
                let witch_material = get_texture_material(
                    "assets/Witch.png",
                    resources,
                    Shader::UNLIT_TRANSPARENT,
                    Color::WHITE,
                );
                let pupil_material = get_texture_material(
                    "assets/Pupil.png",
                    resources,
                    Shader::UNLIT_TRANSPARENT,
                    Color::WHITE,
                );

                let witch = world.spawn((
                    Transform::new()
                        .with_position(top_of_screen + Vec3::Z * 0.2)
                        .with_scale(Vec3::fill(35.0)),
                    witch_material,
                    Mesh::VERTICAL_QUAD,
                ));

                let witch_pupil_l = world.spawn((
                    Transform::new().with_scale(Vec3::fill(0.07)),
                    pupil_material.clone(),
                    Mesh::VERTICAL_QUAD,
                ));
                let witch_pupil_r = world.spawn((
                    Transform::new().with_scale(Vec3::fill(0.07)),
                    pupil_material,
                    Mesh::VERTICAL_QUAD,
                ));

                let witch_pupil_center_l = world.spawn((
                    Transform::new().with_position(Vec3::new(-0.08, -0.14, 0.0)),
                    Eye {
                        radius: 0.03,
                        range: f32::MAX,
                        art: witch_pupil_l,
                        other_eye: None,
                    },
                ));

                let witch_pupil_center_r = world.spawn((
                    Transform::new().with_position(Vec3::new(0.12, -0.14, 0.0)),
                    Eye {
                        radius: 0.03,
                        range: f32::MAX,
                        art: witch_pupil_r,
                        other_eye: Some(witch_pupil_center_l),
                    },
                ));
                world
                    .get::<&mut Eye>(witch_pupil_center_l)
                    .unwrap()
                    .other_eye = Some(witch_pupil_center_r);

                let _ = world.set_parent(witch, witch_pupil_center_l);
                let _ = world.set_parent(witch, witch_pupil_center_r);

                let _ = world.set_parent(witch_pupil_center_l, witch_pupil_l);
                let _ = world.set_parent(witch_pupil_center_r, witch_pupil_r);
            };
            spawn_witch(world);

            let mut shop_world = World::new();
            let camera_child = shop_world.spawn((
                Transform::new(),
                Camera {
                    clear_color: Some(Color::BLACK),
                    exposure: Exposure::EV100(6.0),
                    projection_mode: ProjectionMode::Orthographic {
                        height: view_height,
                        z_near: -2.0,
                        z_far: 2.0,
                    },
                    ..Default::default()
                },
                MainCamera,
            ));
            let camera_parent = shop_world.spawn((Transform::new().with_position(Vec3::Z * 2.0),));
            let _ = shop_world.set_parent(camera_parent, camera_child);

            shop_world.spawn((Transform::new(), EyeFocalPoint, MouseFocalPoint));
            shop_world.spawn((RapierIntegration::new(),));

            spawn_witch(&mut shop_world);
            ui.add_to_world(&mut shop_world);

            // Spawn the shop background
            {
                let new_texture = resources.get::<AssetStore<Texture>>().load(
                    "assets/ShopBackground.png",
                    koi_graphics_context::TextureSettings::default(),
                );

                let new_material = resources.get::<AssetStore<Material>>().add(Material {
                    shader: Shader::UNLIT,
                    base_color_texture: Some(new_texture),
                    ..Default::default()
                });

                shop_world.spawn((
                    Transform::new().with_scale(Vec3::new(160.0 * 2.0, 160.0, 1.0)),
                    Mesh::VERTICAL_QUAD,
                    new_material,
                ));
            }

            let ball_material = get_texture_material(
                "assets/Ball.png",
                resources,
                recolor_shader.clone(),
                Color::from_srgb_hex(0xd4af37, 1.0),
            );

            let peg_hit_sound = resources
                .get::<AssetStore<Sound>>()
                .load("assets/marimba.wav", Default::default());

            world.spawn((Transform::new(), EyeFocalPoint, MouseFocalPoint));

            //world.spawn(ball_bundle);

            let mut random = Random::new();

            world.spawn((rapier_integration,));

            let mut pointer_position = Vec3::ZERO;

            let growable_plant_material =
                load_peg_material(resources, &recolor_shader, Color::ORANGE);
            let plant_material = load_peg_material(resources, &recolor_shader, Color::GREEN);
            let gold_material = load_peg_material(resources, &recolor_shader, Color::YELLOW);
            let stone_material =
                load_peg_material(resources, &recolor_shader, Color::BLACK.with_lightness(0.6));
            let multiball_material = load_peg_material(
                resources,
                &recolor_shader,
                Color::PURPLE.with_lightness(0.6),
            );
            let multiplier_material = load_peg_material(
                resources,
                &recolor_shader,
                Color::ELECTRIC_INDIGO.with_lightness(0.9),
            );

            let brick_material = get_texture_material(
                "assets/Brick.png",
                resources,
                recolor_shader.clone(),
                Color::BLACK.with_lightness(0.6),
            );
            let plus_one = get_texture_material(
                "assets/+1.png",
                resources,
                Shader::UNLIT_TRANSPARENT,
                Color::WHITE,
            );
            let plus_twenty = get_texture_material(
                "assets/+20.png",
                resources,
                Shader::UNLIT_TRANSPARENT,
                Color::WHITE,
            );
            let x2 = get_texture_material(
                "assets/x2.png",
                resources,
                recolor_shader.clone(),
                Color::WHITE,
            );
            resources.add(GameAssets {
                stem_material,
                growable_plant_material,
                plant_material,
                gold_material,
                stone_material: stone_material,
                brick_material,
                multiball_material,
                ball_material,
                plus_one,
                plus_twenty,
                multiplier_material,
                x2,
            });

            // apply_rock_storm(world, resources, P);
            for _ in 0..2 {
                spawn_peg(
                    world,
                    resources,
                    Vec2::new(random.range_f32(-50.0..50.0), random.range_f32(-50.0..-20.)),
                    PegType::GrowablePlant,
                );
            }

            for _ in 0..2 {
                spawn_peg(
                    world,
                    resources,
                    Vec2::new(random.range_f32(-50.0..50.0), random.range_f32(-50.0..10.)),
                    PegType::MultiBall,
                );
            }

            for _ in 0..2 {
                spawn_peg(
                    world,
                    resources,
                    Vec2::new(random.range_f32(-50.0..50.0), random.range_f32(-50.0..10.)),
                    PegType::Multiplier,
                );
            }

            for _ in 0..5 {
                spawn_peg(
                    world,
                    resources,
                    Vec2::new(random.range_f32(-50.0..50.0), random.range_f32(-50.0..10.)),
                    PegType::Stone,
                );
            }

            for i in 0..3 {
                if let Some(p) = select_powerup(&mut shop_world) {
                    spawn_brick_with_powerup(
                        &mut shop_world,
                        resources,
                        Vec2::new(i as f32 * 65.0 - 60.0, -10.0),
                        p,
                    );
                }
            }

            for i in 0..2 {
                if let Some(p) = select_powerup(&mut shop_world) {
                    spawn_brick_with_powerup(
                        &mut shop_world,
                        resources,
                        Vec2::new(i as f32 * 70.0 - 40.0, -50.0),
                        p,
                    );
                }
            }

            apply_rock_storm(
                world,
                resources,
                PegType::Gold,
                0.4,
                0.0,
                4,
                Vec2::new(0.0, -20.0),
                60.0,
            );

            create_rock_wall(world);

            let level_state = LevelState::new(shop_world);
            resources.add(level_state);

            let mut subtract_gold_timer = 1.5;

            // This function will run for major events liked a FixedUpdate occuring
            // and for any input events from the application.
            // See [koi::Event]
            move |event, world, resources| match event {
                Event::FixedUpdate => {
                    temporary::run_delayed_actions(world, resources);

                    // Update the shot visual

                    run_balls(world, resources, -view_height / 2.0);
                    run_pegs(world, resources, &peg_hit_sound);
                    run_health(world, resources);

                    let rapier_integration = world
                        .query::<&mut RapierIntegration>()
                        .iter()
                        .next()
                        .unwrap()
                        .0;
                    let mut rapier_integration = world
                        .remove_one::<RapierIntegration>(rapier_integration)
                        .unwrap();

                    rapier_integration.step(world);
                    world.spawn((rapier_integration,));

                    let victory = resources.get::<LevelState>().victory;
                    if !victory && resources.get::<UIState>().gold >= 1000 {
                        resources.get::<UIState>().current_text =
                            "You've reached 1000 and won!".into();
                        resources.get::<LevelState>().victory = true;
                        apply_rock_storm(
                            world,
                            resources,
                            PegType::MultiBall,
                            0.04,
                            0.0,
                            60,
                            Vec2::ZERO,
                            15.0,
                        );

                        apply_rock_storm(
                            world,
                            resources,
                            PegType::Plant,
                            0.04,
                            0.0,
                            40,
                            Vec2::ZERO,
                            60.0,
                        );
                    }
                }
                Event::KappEvent(KappEvent::KeyDown { key: Key::S, .. }) => {
                    let mut level_state = resources.remove::<LevelState>().unwrap();
                    level_state.toggle_shop(world, resources);
                    resources.add(level_state);
                }
                Event::Draw => {
                    {
                        let mut level_state = resources.get::<LevelState>();

                        let mut ui_state = resources.get::<UIState>();
                        if ui_state.incoming_gold > 0 {
                            if subtract_gold_timer < 0.0 {
                                ui_state.incoming_gold -= 1;
                                ui_state.gold += 1;
                                subtract_gold_timer = 0.2;
                                level_state.screen_shake_amount += 0.05;
                            } else {
                                subtract_gold_timer -= 30.0 / 60.0;
                            }
                        } else {
                            subtract_gold_timer = 1.5;
                        }

                        let screen_shake_amount = &mut level_state.screen_shake_amount;
                        let screen_shake = Vec2::new(
                            random.range_f32(-*screen_shake_amount..*screen_shake_amount),
                            random.range_f32(-*screen_shake_amount..*screen_shake_amount),
                        );

                        let mut q = world.query::<(&mut Transform, &MainCamera)>();
                        let mut iter = q.iter();
                        let (camera_transform, ..) = iter.next().unwrap().1;
                        camera_transform.position =
                            screen_shake.extend(camera_transform.position.z);
                        *screen_shake_amount *= 0.94;
                    }

                    if !world
                        .query::<(&mut GlobalTransform, &Camera)>()
                        .iter()
                        .next()
                        .is_some()
                    {
                        return;
                    }
                    ui.run(world, resources);
                    draw_screen_space_uis(world, resources);

                    run_eyes(world, resources);
                    run_scale(world, resources);
                    temporary::despawn_temporaries(world);

                    let input = resources.get::<Input>();
                    let (x, y) = input.pointer_position();

                    pointer_position = {
                        let mut q = world.query::<(&mut GlobalTransform, &Camera, &MainCamera)>();
                        let mut i = q.iter();
                        let (_, (camera_transform, camera, _)) = i.next().unwrap();

                        let view_size = resources.get::<kapp::Window>().size();
                        let ray = camera.view_to_ray(
                            camera_transform,
                            x as _,
                            y as _,
                            view_size.0 as _,
                            view_size.1 as _,
                        );
                        ray.origin
                    };
                    pointer_position.z = 0.0;

                    world
                        .query::<With<&mut Transform, &MouseFocalPoint>>()
                        .iter()
                        .next()
                        .unwrap()
                        .1
                        .position = pointer_position;

                    let dir = (pointer_position - top_of_screen).normalized();

                    let velocity = dir * SHOT_POWER;

                    let mut v = velocity;
                    let mut p = top_of_screen;

                    let steps = 10;
                    let step_length = 0.3 / steps as f32;

                    for _ in 0..steps {
                        p += v * step_length;
                        v += Vec3::Y * GRAVITY * step_length;

                        p.z = 0.2;

                        world.spawn((
                            Temporary(2),
                            Mesh::VERTICAL_CIRCLE,
                            Material::UNLIT,
                            Transform::new().with_position(p),
                        ));
                    }
                }

                Event::KappEvent(KappEvent::PointerUp {
                    x,
                    y,
                    button: PointerButton::Primary,
                    ..
                }) => {
                    // Shoot one ball at a time
                    {
                        let mut level_state = resources.get::<LevelState>();
                        if !level_state.ready_to_shoot && !level_state.in_shop {
                            return;
                        }
                        level_state.ready_to_shoot = false;
                    }

                    {
                        if resources.get::<UIState>().incoming_gold > 0 {
                            return;
                        }
                    }
                    resources.get::<LevelState>().fired_once = true;
                    resources.get::<UIState>().gold -= 1;

                    pointer_position = {
                        let mut q = world.query::<(&mut GlobalTransform, &Camera, &MainCamera)>();
                        let mut i = q.iter();
                        let (_, (camera_transform, camera, _)) = i.next().unwrap();

                        let view_size = resources.get::<kapp::Window>().size();
                        let ray = camera.view_to_ray(
                            camera_transform,
                            *x as _,
                            *y as _,
                            view_size.0 as _,
                            view_size.1 as _,
                        );
                        ray.origin
                    };

                    let dir = (pointer_position - top_of_screen).normalized() * SHOT_POWER;

                    let mut ball_size = 3.5;
                    let mut health_subtract_rate = 1.0;

                    {
                        let mut level_state = resources.get::<LevelState>();

                        if !level_state.in_shop {
                            for effect in level_state.effects_to_apply_to_next_ball.drain(..) {
                                match effect {
                                    Effects::BigBall => {
                                        ball_size *= 2.0;
                                        health_subtract_rate *= 2.0;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    let p = top_of_screen;

                    spawn_ball(
                        world,
                        resources,
                        p.xy(),
                        dir.xy(),
                        health_subtract_rate,
                        ball_size,
                    );
                }
                _ => {}
            }
        });
}

fn spawn_ball(
    world: &mut World,
    resources: &mut Resources,
    position: Vec2,
    dir: Vec2,
    health_subtract_rate: f32,
    mut ball_size: f32,
) {
    let rapier_integration = world
        .query::<&mut RapierIntegration>()
        .iter()
        .next()
        .unwrap()
        .0;
    let mut rapier_integration = world
        .remove_one::<RapierIntegration>(rapier_integration)
        .unwrap();

    let mut assets = resources.get::<GameAssets>();
    let mut health_subtract_rate = 1.0;

    ball_size = ball_size.min(50.0);

    let rapier_handle = rapier_integration.add_rigid_body_with_collider(
        RigidBodyBuilder::dynamic()
            .linvel([dir.x, dir.y].into())
            .build(),
        ColliderBuilder::ball(ball_size / 2.0)
            .restitution(0.7)
            .build(),
    );
    let position = position.extend(0.3);

    let b = (
        Transform::new()
            .with_position(position)
            .with_scale(Vec3::fill(ball_size)),
        Mesh::VERTICAL_QUAD,
        assets.ball_material.clone(),
        Ball {
            health_subtract_rate,
        },
        EyeFocalPoint,
        rapier_handle,
    );

    world.spawn(b);
    world.spawn((rapier_integration,));
}

#[derive(Clone)]
struct Ball {
    health_subtract_rate: f32,
}

#[derive(Clone)]
struct Peg {
    hit: bool,
    shockwave_child: Entity,
    glowing_material: Handle<Material>,
    peg_type: PegType,
}

#[derive(Clone)]
struct Health(f32);

fn run_health(world: &mut World, _resources: &mut Resources) {
    let mut to_despawn = Vec::new();
    for (e, health) in world.query::<&Health>().iter() {
        if health.0 <= 0.0 {
            to_despawn.push(e);
        }
    }

    for e in to_despawn {
        let _ = world.despawn(e);
    }
}

fn run_balls(world: &mut World, resources: &mut Resources, world_bottom: f32) {
    let mut count = 0;
    let mut to_despawn = Vec::new();
    for (e, (transform, ball)) in world.query::<(&Transform, &mut Ball)>().iter() {
        if transform.position.y < world_bottom {
            to_despawn.push(e);
        } else {
            count += 1;
        }
    }

    for e in to_despawn {
        let _ = world.despawn(e);
    }

    if count == 0 {
        let mut level_state = resources.get::<LevelState>();
        let new_gold = level_state.prepare_to_shoot(world);

        if level_state.fired_once {
            level_state.fired_once = false;
            if new_gold == 0 {
                resources.get::<UIState>().current_text = ":(".into();
            }

            if new_gold > 10 {
                resources.get::<UIState>().current_text = "Nice shot!".into();
            }

            if new_gold > 50 {
                resources.get::<UIState>().current_text = "Great shot!".into();
            }

            if new_gold > 100 {
                resources.get::<UIState>().current_text = "Amazing!".into();
            }
            if new_gold > 200 {
                resources.get::<UIState>().current_text = ":O :O :O!!!!".into();
            }
        }
        if resources.get::<UIState>().gold >= 1000 {
            resources.get::<UIState>().current_text = "YOU WIN!!!".into();
        }

        resources.get::<UIState>().incoming_gold +=
            (new_gold as f32 * level_state.multiplier) as i32;
        level_state.multiplier = 1.0;
        level_state.ready_to_shoot = true;
    }
}

fn run_pegs(world: &mut World, resources: &mut Resources, peg_hit_sound: &Handle<Sound>) {
    let mut to_despawn = Vec::new();
    let mut deferred_actions = Vec::new();
    {
        {
            let rapier_integration = world
                .query::<&mut RapierIntegration>()
                .iter()
                .next()
                .unwrap()
                .0;
            let rapier_integration = world
                .remove_one::<RapierIntegration>(rapier_integration)
                .unwrap();

            let sounds = resources.get::<AssetStore<Sound>>();
            let mut audio_manager = resources.get::<AudioManager>();

            let peg_hit_sound = sounds.get(peg_hit_sound);

            let mut level_state = resources.get::<LevelState>();

            for (e, (transform, ball)) in world.query::<(&Transform, &mut Ball)>().iter() {
                let collider = world.get::<&RapierRigidBody>(e).unwrap();
                for contact_pair in rapier_integration
                    .narrow_phase
                    .contacts_with(collider.collider_handle)
                {
                    if !contact_pair.has_any_active_contact {
                        continue;
                    }

                    let other_collider = if contact_pair.collider1 == collider.collider_handle {
                        contact_pair.collider2
                    } else {
                        contact_pair.collider1
                    };

                    let user_data = rapier_integration
                        .collider_set
                        .get(other_collider)
                        .unwrap()
                        .user_data;

                    if user_data != 0 {
                        let entity = Entity::from_bits(user_data as _).unwrap();

                        if let Ok(mut peg) = world.get::<&mut Peg>(entity) {
                            if !peg.hit {
                                peg.hit = true;
                                *world.get::<&mut Handle<Material>>(entity).unwrap() =
                                    peg.glowing_material.clone();

                                level_state.pitch_multiplier += 0.1;
                                level_state.pitch_multiplier =
                                    level_state.pitch_multiplier.min(10.0);

                                world.get::<&mut Scale>(peg.shockwave_child).unwrap().t = 0.5;
                                level_state.collected_pegs.push(entity);

                                match peg.peg_type {
                                    PegType::Plant | PegType::Gold => audio_manager
                                        .play_one_shot_with_speed(
                                            peg_hit_sound,
                                            level_state.pitch_multiplier,
                                        ),
                                    PegType::Multiplier => {
                                        let position =
                                            world.get::<&GlobalTransform>(entity).unwrap().position;
                                        deferred_actions.push(DelayedAction::new(
                                            move |world, resources| {
                                                world.spawn((
                                                    Transform::new()
                                                        .with_position(
                                                            position - Vec2::fill(2.0).extend(0.0),
                                                        )
                                                        .with_scale(Vec3::fill(20.0)),
                                                    resources.get::<GameAssets>().x2.clone(),
                                                    Mesh::VERTICAL_QUAD,
                                                    Temporary(90),
                                                ));
                                            },
                                            0.01,
                                        ));
                                        level_state.multiplier *= 2.0;
                                        audio_manager.play_one_shot_with_speed(peg_hit_sound, 8.0);
                                    }
                                    _ => {
                                        audio_manager.play_one_shot_with_speed(peg_hit_sound, 0.7);
                                    }
                                }
                                match peg.peg_type {
                                    PegType::Gold => {
                                        let position =
                                            world.get::<&GlobalTransform>(entity).unwrap().position;
                                        deferred_actions.push(DelayedAction::new(
                                            move |world, resources| {
                                                world.spawn((
                                                    Transform::new()
                                                        .with_position(
                                                            position + Vec2::fill(2.0).extend(0.0),
                                                        )
                                                        .with_scale(Vec3::fill(15.0)),
                                                    resources
                                                        .get::<GameAssets>()
                                                        .plus_twenty
                                                        .clone(),
                                                    Mesh::VERTICAL_QUAD,
                                                    Temporary(50),
                                                ));
                                            },
                                            0.01,
                                        ));

                                        level_state.screen_shake_amount += 0.3;
                                    }
                                    PegType::Plant => {
                                        let position =
                                            world.get::<&GlobalTransform>(entity).unwrap().position;
                                        deferred_actions.push(DelayedAction::new(
                                            move |world, resources| {
                                                world.spawn((
                                                    Transform::new()
                                                        .with_position(
                                                            position + Vec2::fill(2.0).extend(0.0),
                                                        )
                                                        .with_scale(Vec3::fill(12.0)),
                                                    resources.get::<GameAssets>().plus_one.clone(),
                                                    Mesh::VERTICAL_QUAD,
                                                    Temporary(30),
                                                ));
                                            },
                                            0.01,
                                        ));

                                        level_state.screen_shake_amount += 0.05;
                                    }
                                    PegType::MultiBall => {
                                        level_state.screen_shake_amount += 0.1;
                                        let position =
                                            world.get::<&GlobalTransform>(entity).unwrap().position;

                                        deferred_actions.push(DelayedAction::new(
                                            move |world, resources| {
                                                let _ = world.despawn(entity);
                                                spawn_ball(
                                                    world,
                                                    resources,
                                                    position.xy(),
                                                    Vec2::ZERO,
                                                    1.0,
                                                    3.5,
                                                );
                                            },
                                            0.01,
                                        ))
                                    }
                                    _ => {}
                                }
                            }
                        }

                        if let Ok(mut powerup) = world.get::<&mut Powerup>(entity) {
                            level_state.screen_shake_amount += 0.8;

                            if powerup.cost < 0 {
                                resources.get::<UIState>().gold -= powerup.cost;
                            }
                            powerup.cost -= 1;

                            // Acquire power up
                            if powerup.cost <= 0 {
                                powerup.cost = 0;
                                to_despawn.push(entity);
                                level_state
                                    .effects_to_apply_to_next_ball
                                    .push(powerup.effect.clone());
                                // Quit shop
                            }
                            to_despawn.push(e);
                        }
                        // Remove health as this ball touches the peg to prevent it from getting stuck.
                        if let Ok(mut health) = world.get::<&mut Health>(entity) {
                            health.0 -= ball.health_subtract_rate / 60.0;
                        }
                    }
                }
            }

            world.spawn((rapier_integration,));
        }

        for deferred_action in deferred_actions {
            world.spawn((deferred_action,));
        }
        for e in to_despawn {
            if world.get::<&Powerup>(e).is_ok() {
                let t = world.get::<&Transform>(e).unwrap().position;

                world.spawn((DelayedAction::new(
                    Box::new(move |world: &mut World, resources: &mut Resources| {
                        // Replacement powerup
                        if let Some(p) = select_powerup(world) {
                            spawn_brick_with_powerup(world, resources, t.xy(), p);
                        }
                    }),
                    0.6,
                ),));
            }
            let _ = world.insert_one(e, Temporary(10));
        }
    }
}

fn select_powerup(world: &mut World) -> Option<Powerup> {
    let in_world: Vec<_> = world
        .query::<&Powerup>()
        .iter()
        .map(|i| i.1.effect.clone())
        .collect();

    println!("IN WORLD: {:?}", in_world);
    let mut random = Random::new();

    let mut replacement_type = random.select_from_slice(&POWERUPS).clone();

    let mut max_iterations = 10;
    while in_world.contains(&replacement_type.effect) {
        replacement_type = random.select_from_slice(&POWERUPS).clone();
        if max_iterations == 0 {
            return None;
        }
        max_iterations -= 1;
    }
    Some(replacement_type)
}

#[derive(Clone, PartialEq)]
enum PegType {
    GrowablePlant,
    Plant,
    Gold,
    Stone,
    MultiBall,
    Multiplier,
}
fn spawn_peg(
    world: &mut World,
    resources: &Resources,
    position: Vec2,
    peg_type: PegType,
) -> Entity {
    let game_assets = resources.get::<GameAssets>();

    let mut scale = 4.0 * 2.4;

    if peg_type == PegType::Gold {
        scale *= 1.4;
    }

    let rapier_handle = {
        let mut q = world.query::<&mut RapierIntegration>();
        let rapier_integration = q.iter().next().unwrap().1;
        rapier_integration.add_rigid_body_with_collider(
            RigidBodyBuilder::kinematic_position_based().build(),
            ColliderBuilder::ball(scale * 0.3).restitution(0.7).build(),
        )
    };

    let PegMaterial {
        base,
        glowing,
        shockwave,
    } = match peg_type {
        PegType::GrowablePlant => &game_assets.growable_plant_material,
        PegType::Plant => &game_assets.plant_material,
        PegType::Gold => &game_assets.gold_material,
        PegType::Stone => &game_assets.stone_material,
        PegType::MultiBall => &game_assets.multiball_material,
        PegType::Multiplier => &game_assets.multiplier_material,
    };

    let child = world.spawn((
        Transform::new().with_position(Vec3::Z * -0.01),
        shockwave.clone(),
        Mesh::VERTICAL_QUAD,
        Scale {
            rate: 5.0,
            t: 1.0,
            max_scale: 1.5,
            t_max: 2.0,
        },
    ));

    let position = position.extend(0.3);

    let parent = world.spawn((
        Transform::new()
            .with_scale(Vec3::fill(scale))
            .with_position(position),
        Mesh::VERTICAL_QUAD,
        Peg {
            hit: false,
            glowing_material: glowing.clone(),
            shockwave_child: child,
            peg_type: peg_type.clone(),
        },
        Health(1.0),
        base.clone(),
        rapier_handle,
    ));
    match peg_type {
        PegType::GrowablePlant => {
            let _ = world.insert(parent, (Plant { last_direction: 0 },));
        }
        _ => {}
    }
    let _ = world.set_parent(parent, child);
    parent
}

#[derive(Clone)]
struct Powerup {
    cost: i32,
    description: &'static str,
    effect: Effects,
}

fn spawn_brick(
    world: &mut World,
    resources: &mut Resources,
    position: Vec2,
    dimensions: Vec2,
) -> Entity {
    let game_assets = resources.get::<GameAssets>();

    let rapier_handle = {
        let mut q = world.query::<&mut RapierIntegration>();
        let rapier_integration = q.iter().next().unwrap().1;
        rapier_integration.add_rigid_body_with_collider(
            RigidBodyBuilder::kinematic_position_based().build(),
            ColliderBuilder::cuboid(dimensions.x / 2.0 * 0.98, dimensions.y / 2.0 * 0.98)
                .restitution(0.7)
                .build(),
        )
    };

    let position = position.extend(0.3);

    let parent = world.spawn((
        Transform::new()
            .with_scale(dimensions.extend(1.0))
            .with_position(position),
        Mesh::VERTICAL_QUAD,
        Health(1.0),
        game_assets.brick_material.clone(),
        rapier_handle,
    ));

    parent
}

fn spawn_brick_with_powerup(
    world: &mut World,
    resources: &mut Resources,
    position: Vec2,
    powerup: Powerup,
) -> Entity {
    let dimensions = Vec2::new(40.0, 20.0);

    let name = powerup.description;

    let screen_space_ui = ScreenSpaceUI::new(
        world,
        resources,
        kui::center(kui::text(move |state: &mut UIState| {
            let mut name = name.to_string().clone();
            name.push_str(&format!(": {:?}", state.hacky_remaining_health));
            name
        })),
    );
    let game_assets = resources.get::<GameAssets>();

    let rapier_handle = {
        let mut q = world.query::<&mut RapierIntegration>();
        let rapier_integration = q.iter().next().unwrap().1;
        rapier_integration.add_rigid_body_with_collider(
            RigidBodyBuilder::kinematic_position_based().build(),
            ColliderBuilder::cuboid(dimensions.x / 2.0 * 0.98, dimensions.y / 2.0 * 0.98)
                .restitution(0.7)
                .build(),
        )
    };

    let position = position.extend(0.3);

    let parent = world.spawn((
        Transform::new()
            .with_scale(dimensions.extend(1.0))
            .with_position(position),
        Mesh::VERTICAL_QUAD,
        powerup,
        game_assets.brick_material.clone(),
        rapier_handle,
    ));

    let _ = world.set_parent(parent, screen_space_ui);
    parent
}

fn spawn_gold(world: &mut World, resources: &Resources, position: Vec2, stem_direction: Vec2) {
    world.spawn((
        Mesh::VERTICAL_QUAD,
        Transform::new()
            .with_position((position - stem_direction / 2.0).extend(0.2))
            .with_rotation(Quat::from_forward_up(
                -Vec3::Z,
                stem_direction.normalized().extend(0.0),
            ))
            .with_scale(Vec3::new(2.0, stem_direction.length() * 1.1, 1.0)),
        resources.get::<GameAssets>().stem_material.clone(),
    ));
    spawn_peg(world, resources, position, PegType::Gold);
}

fn spawn_plant(world: &mut World, resources: &Resources, position: Vec2, stem_direction: Vec2) {
    spawn_peg(world, resources, position, PegType::Plant);
    world.spawn((
        Mesh::VERTICAL_QUAD,
        Transform::new()
            .with_position((position - stem_direction / 2.0).extend(0.2))
            .with_rotation(Quat::from_forward_up(
                -Vec3::Z,
                stem_direction.normalized().extend(0.0),
            ))
            .with_scale(Vec3::new(2.0, stem_direction.length() * 1.1, 1.0)),
        resources.get::<GameAssets>().stem_material.clone(),
    ));
}
