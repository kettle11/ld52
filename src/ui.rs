use koi3::{koi_graphics_context::FacesToRender, *};
use kui::{
    colored_rectangle, text, GetStandardInput, GetStandardStyle, StandardContext, StandardStyle,
};

pub struct UI {
    drawer: kui::Drawer,
    context: StandardContext<UIState>,
    root_widget: Box<dyn kui::Widget<UIState, StandardContext<UIState>>>,
    _ui_entity: Entity,
    ui_material: Handle<Material>,
    ui_mesh: Handle<Mesh>,
}

struct UIState {
    current_text: String,
}
impl UI {
    pub fn new(world: &mut World, resources: &mut Resources) -> Self {
        resources.add(UIState {
            current_text: "Welcome to the farm!".to_string(),
        });

        let projection_matrix =
            koi3::projection_matrices::orthographic_gl(-1.0, 1.0, -1.0, 1.0, 0.0, 1.0);
        world.spawn((
            Transform::new(),
            Camera {
                clear_color: None,
                projection_mode: ProjectionMode::Custom(projection_matrix),
                ..Default::default()
            },
            RenderFlags::USER_INTERFACE,
        ));
        let mut meshes = resources.get::<AssetStore<Mesh>>();
        let mut materials = resources.get::<AssetStore<Material>>();
        let mut graphics_context = &mut resources.get::<Renderer>().raw_graphics_context;

        let ui_shader = resources.get::<AssetStore<Shader>>().load(
            "assets/unlit_ui.glsl",
            ShaderSettings {
                faces_to_render: FacesToRender::FrontAndBack,
                blending: Some((
                    koi_graphics_context::BlendFactor::One,
                    koi_graphics_context::BlendFactor::OneMinusSourceAlpha,
                )),
                ..Default::default()
            },
        );

        let ui_mesh = meshes.add(koi3::Mesh::new(&mut graphics_context, MeshData::default()));
        let ui_material = materials.add(Material {
            shader: ui_shader,
            ..Default::default()
        });

        let ui_entity = world.spawn((
            Transform::new(),
            ui_mesh.clone(),
            ui_material.clone(),
            RenderFlags::USER_INTERFACE,
        ));

        let mut fonts = kui::Fonts::empty();
        let _ = fonts.new_font_from_bytes(include_bytes!("../assets/Signika-SemiBold.ttf"));
        let mut style = StandardStyle::default();
        style.primary_text_color = Color::from_srgb_hex(0x72503B, 1.0);
        style.primary_text_size = 24.0;
        style.rounding = 28.0;
        style.padding = 28.0;

        use kui::*;

        let ui = align(
            Alignment::End,
            Alignment::Start,
            row((
                colored_rectangle(Vec2::new(50.0, 0.0), |_, _, _| Color::TRANSPARENT),
                padding(fit(stack((
                    rounded_fill(
                        |_, _, c: &StandardContext<_>| Color::from_srgb_hex(0xD9D9D9, 1.0),
                        |_, c| c.standard_style().rounding,
                    ),
                    padding(text(|state: &mut UIState| state.current_text.clone())),
                )))),
            )),
        );

        Self {
            drawer: kui::Drawer::new(),
            context: StandardContext::new(style, Default::default(), fonts),
            root_widget: Box::new(ui),
            _ui_entity: ui_entity,
            ui_material,
            ui_mesh,
        }
    }

    pub fn run(&mut self, world: &mut World, resources: &mut Resources) {
        let mut ui_state = resources.get::<UIState>();
        let window = resources.get::<kapp::Window>();
        let (window_width, window_height) = window.size();
        let ui_scale = window.scale();

        let width = window_width as f32 / ui_scale as f32;
        let height = window_height as f32 / ui_scale as f32;

        self.context.standard_style_mut().ui_scale = ui_scale as _;
        self.context.standard_input_mut().view_size = Vec2::new(width, height);

        let constraints = kui::MinAndMaxSize {
            min: Vec3::ZERO,
            max: Vec3::new(width, height, 10_000.0),
        };

        self.root_widget
            .layout(&mut ui_state, &mut (), &mut self.context, constraints);
        self.drawer.reset();
        self.drawer.set_view_width_height(width, height);

        self.root_widget.draw(
            &mut ui_state,
            &mut (),
            &mut self.context,
            &mut self.drawer,
            Box3::new_with_min_corner_and_size(constraints.min, constraints.max),
        );

        let mut meshes = resources.get::<AssetStore<Mesh>>();
        let mut textures = resources.get::<AssetStore<Texture>>();
        let mut materials = resources.get::<AssetStore<Material>>();

        let mut graphics_context = &mut resources.get::<Renderer>().raw_graphics_context;

        let first_mesh_data = &self.drawer.first_mesh;
        let mesh_data = MeshData {
            positions: first_mesh_data.positions.clone(),
            indices: first_mesh_data.indices.clone(),
            colors: first_mesh_data.colors.clone(),
            texture_coordinates: first_mesh_data.texture_coordinates.clone(),
            ..Default::default()
        };

        *meshes.get_mut(&self.ui_mesh) = Mesh::new(&mut graphics_context, mesh_data);

        if self.drawer.texture_atlas.changed {
            self.drawer.texture_atlas.changed = false;

            unsafe {
                let new_texture = graphics_context.new_texture_with_bytes(
                    self.drawer.texture_atlas.width as u32,
                    self.drawer.texture_atlas.height as u32,
                    1,
                    &self.drawer.texture_atlas.data,
                    koi_graphics_context::PixelFormat::R8Unorm,
                    koi_graphics_context::TextureSettings {
                        srgb: false,
                        ..Default::default()
                    },
                );

                let new_texture_handle = textures.add(koi3::Texture(new_texture));
                materials.get_mut(&self.ui_material).base_color_texture = Some(new_texture_handle);
            }
        }
    }
}