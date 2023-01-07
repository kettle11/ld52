use koi3::*;
mod rapier_integration;
use rapier2d::prelude::*;
use rapier_integration::*;
mod temporary;
use temporary::*;

struct LevelState {
    pitch_multiplier: f32,
    aiming: bool,
}

fn main() {
    App::default().setup_and_run(|world, resources| {
        let mut rapier_integration = rapier_integration::RapierIntegration::new();

        let view_height = 100.0;
        world.spawn((
            Transform::new().with_position(Vec3::Z * 2.0),
            Camera {
                clear_color: Some(Color::BLACK),
                exposure: Exposure::EV100(6.0),
                projection_mode: ProjectionMode::Orthographic {
                    height: 100.0,
                    z_near: -2.0,
                    z_far: 2.0,
                },
                ..Default::default()
            },
        ));

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

        fn get_texture_material(path: &str, resources: &Resources) -> Handle<Material> {
            let texture = resources
                .get::<AssetStore<Texture>>()
                .load(path, koi_graphics_context::TextureSettings::default());
            resources.get::<AssetStore<Material>>().add(Material {
                shader: Shader::UNLIT_TRANSPARENT,
                base_color_texture: Some(texture),
                ..Default::default()
            })
        }

        let peg_material = get_texture_material("assets/Peg.png", resources);
        let peg_glowing_material = get_texture_material("assets/PegGlowing.png", resources);

        let ball_material = get_texture_material("assets/Ball.png", resources);

        let peg_hit_sound = resources
            .get::<AssetStore<Sound>>()
            .load("assets/marimba.wav", Default::default());

        let peg_bundle = (
            Transform::new().with_scale(Vec3::fill(4.0 * 2.4)),
            Mesh::VERTICAL_QUAD,
            Peg { hit: false },
            peg_material,
        );

        let ball_bundle = (
            Transform::new().with_scale(Vec3::fill(3.5)),
            Mesh::VERTICAL_QUAD,
            ball_material,
        );

        //world.spawn(ball_bundle);

        let mut random = Random::new();

        for _ in 0..50 {
            let rapier_handle = rapier_integration.add_rigid_body_with_collider(
                RigidBodyBuilder::kinematic_position_based().build(),
                ColliderBuilder::ball(0.5 * 3.8).restitution(0.7).build(),
            );

            let mut p = peg_bundle.clone();
            p.0.position = Vec3::new(
                random.range_f32(-50.0..50.0),
                random.range_f32(-50.0..30.0),
                0.3,
            );
            world.spawn((p.0, p.1, p.2, p.3, rapier_handle));
        }

        resources.add(rapier_integration);

        let top_of_screen = Vec3::new(0.0, view_height / 2.0 * 0.75, 0.0);

        resources.add(LevelState {
            pitch_multiplier: 1.0,
            aiming: true,
        });

        let mut pointer_position = Vec3::ZERO;

        // This function will run for major events liked a FixedUpdate occuring
        // and for any input events from the application.
        // See [koi::Event]
        move |event, world, resources| match event {
            Event::FixedUpdate => {
                // Update the shot visual

                run_pegs(world, resources, &peg_glowing_material, &peg_hit_sound);

                let mut rapier_integration =
                    resources.get::<rapier_integration::RapierIntegration>();

                rapier_integration.step(world);
            }
            Event::Draw => {
                temporary::despawn_temporaries(world);

                let input = resources.get::<Input>();
                let (x, y) = input.pointer_position();

                pointer_position = {
                    let mut q = world.query::<(&mut GlobalTransform, &Camera)>();
                    let mut iter = q.iter();
                    let (camera_transform, camera) = iter.next().unwrap().1;

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
                let dir = (pointer_position - top_of_screen).normalized() * 40.0;

                let mut v = dir;
                let mut p = top_of_screen;

                let steps = 10;
                let step_length = 0.7 / steps as f32;

                for _ in 0..steps {
                    p += v * step_length;
                    v += Vec3::Y * 4.0 * -9.81 * step_length;

                    p.z = 0.2;

                    world.spawn((
                        Temporary(1),
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
                    let mut q = world.query::<(&mut GlobalTransform, &Camera)>();
                    let mut iter = q.iter();
                    let (camera_transform, camera) = iter.next().unwrap().1;

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

                let dir = (pointer_position - top_of_screen).normalized() * 40.0;

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
}

fn run_pegs(
    world: &mut World,
    resources: &mut Resources,
    peg_glowing_material: &Handle<Material>,
    peg_hit_sound: &Handle<Sound>,
) {
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

                println!("COLLIDED WITH");

                if let Ok(mut peg) = world.get::<&mut Peg>(entity) {
                    if !peg.hit {
                        peg.hit = true;
                        *world.get::<&mut Handle<Material>>(entity).unwrap() =
                            peg_glowing_material.clone();
                        audio_manager
                            .play_one_shot_with_speed(peg_hit_sound, level_state.pitch_multiplier);
                        level_state.pitch_multiplier += 0.1;
                    }
                }
            }
        }
    }
}
