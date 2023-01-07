use koi3::*;
mod rapier_integration;
use rapier2d::prelude::*;
use rapier_integration::*;
mod temporary;
use koi_graphics_context::BlendFactor;
use temporary::*;
mod ui;

struct LevelState {
    pitch_multiplier: f32,
    aiming: bool,
    ready_to_shoot: bool,
    collected_pegs: Vec<Entity>,
}

impl LevelState {
    pub fn new() -> Self {
        Self {
            pitch_multiplier: 1.0,
            aiming: true,
            ready_to_shoot: true,
            collected_pegs: Vec::new(),
        }
    }
    pub fn prepare_to_shoot(&mut self, world: &mut World) {
        self.ready_to_shoot = true;
        self.pitch_multiplier = 1.0;

        // TODO: Make this more satisfying
        let mut time_offset = 5;
        for entity in self.collected_pegs.drain(..) {
            let _ = world.insert_one(entity, Temporary(time_offset));
            time_offset += 5;

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
                            spawn_gold(world, resources, position, stem_direction * 8.0);
                        } else {
                            if gold_energy > 0 && Random::new().f32() > 0.8 {
                                spawn_gold(world, resources, position, stem_direction * 8.0);
                                gold_energy -= 1;
                            } else {
                                spawn_plant(world, resources, position, stem_direction * 8.0);
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
                                    let range = std::f32::consts::PI * 0.2;
                                    let rotation = range * -1.0 + Random::new().f32() * range * 2.0;

                                    let rotation = Quat::from_angle_axis(rotation, Vec3::Z);
                                    let new_random_dir =
                                        rotation.rotate_vector3(stem_direction.extend(0.0)).xy();

                                    let position = position + new_random_dir * 8.0;

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
                            0.01,
                        ),));
                    }
                }

                let range = std::f32::consts::PI * 0.5;
                let rotation = range * -1.0 + Random::new().f32() * range * 2.0;

                let rotation = Quat::from_angle_axis(rotation, Vec3::Z);
                let new_random_dir = rotation.rotate_vector3(Vec2::Y.extend(0.0)).xy();

                world.spawn((DelayedAction::new(
                    move |world, resources| {
                        plant_segment(world, resources, p, new_random_dir, 7, true, 1);
                    },
                    0.01,
                ),));
            }
        }
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
}

struct PegMaterial {
    base: Handle<Material>,
    glowing: Handle<Material>,
    shockwave: Handle<Material>,
}

const SHOT_POWER: f32 = 70.0;

struct EyeFocalPoint;

struct Plant {
    last_direction: usize,
}

struct PlantGrowth;

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

fn main() {
    App::default().setup_and_run(|world, resources| {
        let rapier_integration = rapier_integration::RapierIntegration::new();

        let view_height = 100.0;
        let camera_entity = world.spawn((
            Transform::new().with_position(Vec3::Z * 2.0),
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
        ));

        let top_of_screen = Vec3::new(0.0, view_height / 2.0 * 0.75, 0.0);

        // Spawn the background
        {
            let new_texture = resources.get::<AssetStore<Texture>>().load(
                "assets/fantasy_farm_background.png",
                koi_graphics_context::TextureSettings::default(),
            );

            let new_material = resources.get::<AssetStore<Material>>().add(Material {
                shader: Shader::UNLIT,
                base_color_texture: Some(new_texture),
                ..Default::default()
            });

            world.spawn((
                Transform::new().with_scale(Vec3::fill(100.0)),
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

        let mut spawn_witch = || {
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
                    .with_scale(Vec3::fill(25.0)),
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
        spawn_witch();

        let peg_color = Color::GREEN.with_lightness(0.8).with_chroma(0.4);

        let ball_material = get_texture_material(
            "assets/Ball.png",
            resources,
            recolor_shader.clone(),
            Color::RED,
        );

        let peg_hit_sound = resources
            .get::<AssetStore<Sound>>()
            .load("assets/marimba.wav", Default::default());

        let ball_bundle = (
            Transform::new().with_scale(Vec3::fill(3.5)),
            Mesh::VERTICAL_QUAD,
            ball_material,
        );

        let mouse_focal_point = world.spawn((Transform::new(), EyeFocalPoint));

        //world.spawn(ball_bundle);

        let mut random = Random::new();

        resources.add(rapier_integration);

        resources.add(LevelState::new());

        let mut pointer_position = Vec3::ZERO;

        let growable_plant_material = load_peg_material(resources, &recolor_shader, Color::ORANGE);
        let plant_material = load_peg_material(resources, &recolor_shader, Color::GREEN);
        let gold_material = load_peg_material(resources, &recolor_shader, Color::YELLOW);

        resources.add(GameAssets {
            stem_material,
            growable_plant_material,
            plant_material,
            gold_material,
        });

        for _ in 0..3 {
            spawn_growable_plant_peg(
                world,
                resources,
                Vec2::new(random.range_f32(-50.0..50.0), random.range_f32(-50.0..-20.)),
            );
        }

        let mut ui = ui::UI::new(world, resources);

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

                let mut rapier_integration =
                    resources.get::<rapier_integration::RapierIntegration>();

                rapier_integration.step(world);
            }

            Event::Draw => {
                ui.run(world, resources);

                run_eyes(world, resources);
                run_scale(world, resources);
                temporary::despawn_temporaries(world);

                let input = resources.get::<Input>();
                let (x, y) = input.pointer_position();

                pointer_position = {
                    let (camera_transform, camera) = world
                        .query_one_mut::<(&mut GlobalTransform, &Camera)>(camera_entity)
                        .unwrap();

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
                    .get::<&mut Transform>(mouse_focal_point)
                    .unwrap()
                    .position = pointer_position;

                let dir = (pointer_position - top_of_screen).normalized();

                let velocity = dir * SHOT_POWER;

                let mut v = velocity;
                let mut p = top_of_screen;

                let steps = 10;
                let step_length = 0.3 / steps as f32;

                for _ in 0..steps {
                    p += v * step_length;
                    v += Vec3::Y * 4.0 * -9.81 * step_length;

                    p.z = 0.2;

                    world.spawn((
                        Temporary(2),
                        Mesh::VERTICAL_CIRCLE,
                        Material::UNLIT,
                        Transform::new().with_position(p),
                    ));
                }
            }

            Event::KappEvent(KappEvent::PointerDown {
                x,
                y,
                button: PointerButton::Primary,
                ..
            }) => {
                let mut rapier_integration = resources.get::<RapierIntegration>();

                pointer_position = {
                    let (camera_transform, camera) = world
                        .query_one_mut::<(&mut GlobalTransform, &Camera)>(camera_entity)
                        .unwrap();

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

                let rapier_handle = rapier_integration.add_rigid_body_with_collider(
                    RigidBodyBuilder::dynamic()
                        .linvel([dir.x, dir.y].into())
                        .build(),
                    ColliderBuilder::ball(0.5 * 3.5).restitution(0.7).build(),
                );
                let mut p = top_of_screen;
                p.z = 0.3;
                let b = (
                    ball_bundle.0.with_position(p),
                    ball_bundle.1.clone(),
                    ball_bundle.2.clone(),
                    Ball,
                    EyeFocalPoint,
                    rapier_handle,
                );

                world.spawn(b);
            }
            _ => {}
        }
    });
}

#[derive(Clone)]
struct Ball;

#[derive(Clone)]
struct Peg {
    hit: bool,
    shockwave_child: Entity,
    glowing_material: Handle<Material>,
}

#[derive(Clone)]
struct Health(f32);

fn run_health(world: &mut World, resources: &mut Resources) {
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
    for (e, (transform, ball)) in world.query::<(&GlobalTransform, &mut Ball)>().iter() {
        if transform.position.y < world_bottom {
            to_despawn.push(e);
        } else {
            count += 1;
        }
    }

    for e in to_despawn {
        println!("DESPAWNING BALL");
        let _ = world.despawn(e);
    }

    if count == 0 {
        let mut level_state = resources.get::<LevelState>();
        level_state.prepare_to_shoot(world);
    }
}

fn run_pegs(world: &mut World, resources: &mut Resources, peg_hit_sound: &Handle<Sound>) {
    {
        let rapier_integration = resources.get::<RapierIntegration>();
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
                            audio_manager.play_one_shot_with_speed(
                                peg_hit_sound,
                                level_state.pitch_multiplier,
                            );
                            level_state.pitch_multiplier += 0.1;

                            world.get::<&mut Scale>(peg.shockwave_child).unwrap().t = 0.5;
                            level_state.collected_pegs.push(entity);
                        }

                        // Remove health as this ball touches the peg to prevent it from getting stuck.
                        world.get::<&mut Health>(entity).unwrap().0 -= 1.0 / 60.0;
                    }
                }
            }
        }
    }
}

enum PegType {
    GrowablePlant,
    Plant,
    Gold,
}
fn spawn_peg(
    world: &mut World,
    resources: &Resources,
    position: Vec2,
    peg_type: PegType,
) -> Entity {
    let game_assets = resources.get::<GameAssets>();

    let mut rapier_integration = resources.get::<rapier_integration::RapierIntegration>();

    let rapier_handle = rapier_integration.add_rigid_body_with_collider(
        RigidBodyBuilder::kinematic_position_based().build(),
        ColliderBuilder::ball(0.5 * 3.8).restitution(0.7).build(),
    );

    let PegMaterial {
        base,
        glowing,
        shockwave,
    } = match peg_type {
        PegType::GrowablePlant => &game_assets.growable_plant_material,
        PegType::Plant => &game_assets.plant_material,
        PegType::Gold => &game_assets.gold_material,
    };

    let child = world.spawn((
        Transform::new().with_position(Vec3::Z * -0.01),
        shockwave.clone(),
        Mesh::VERTICAL_QUAD,
        Scale {
            rate: 5.0,
            t: 2.0,
            max_scale: 1.5,
            t_max: 2.0,
        },
    ));

    let position = position.extend(0.3);

    let parent = world.spawn((
        Transform::new()
            .with_scale(Vec3::fill(4.0 * 2.4))
            .with_position(position),
        Mesh::VERTICAL_QUAD,
        Peg {
            hit: false,
            glowing_material: glowing.clone(),
            shockwave_child: child,
        },
        Health(1.0),
        base.clone(),
        rapier_handle,
    ));
    let _ = world.set_parent(parent, child);
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

fn spawn_growable_plant_peg(world: &mut World, resources: &Resources, position: Vec2) {
    let e = spawn_peg(world, resources, position, PegType::GrowablePlant);

    let _ = world.insert(e, (Plant { last_direction: 0 },));
}
