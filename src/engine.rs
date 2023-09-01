use std::iter;

use cgmath::prelude::*;
use wgpu::{util::DeviceExt, Buffer};
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    window::{Fullscreen, Window, WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::{camera, texture, Block, BlockType, Chunk};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
}
pub struct Mesh {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    num_elements: u32,
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
pub struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    camera: camera::Camera,
    projection: camera::Projection,
    pub camera_controller: camera::CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture: texture::Texture,
    window: Window,
    texture_bind_group: wgpu::BindGroup,
    pub mouse_pressed: bool,
}

impl State {
    pub async fn new() -> (Self, EventLoop<()>) {
        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
            } else {
                env_logger::init();
            }
        }

        let event_loop = EventLoop::new();
        let title = env!("CARGO_PKG_NAME");
        let monitor = event_loop.primary_monitor().unwrap();
        let video_mode = monitor.video_modes().next();
        let size = video_mode
            .clone()
            .map_or(PhysicalSize::new(800, 600), |vm| vm.size());
        let window = WindowBuilder::new()
            .with_visible(false)
            .with_title(title)
            .with_fullscreen(video_mode.map(|vm| Fullscreen::Exclusive(vm)))
            .build(&event_loop)
            .unwrap();
        if window.fullscreen().is_none() {
            window.set_inner_size(PhysicalSize::new(512, 512));
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowExtWebSys;
            web_sys::window()
                .and_then(|win| win.document())
                .and_then(|doc| {
                    let dst = doc.get_element_by_id("wasm-example")?;
                    let canvas = web_sys::Element::from(window.canvas());
                    dst.append_child(&canvas).ok()?;

                    // Request fullscreen, if denied, continue as normal
                    match canvas.request_fullscreen() {
                        Ok(_) => {}
                        Err(_) => (),
                    }

                    Some(())
                })
                .expect("Couldn't append canvas to document body.");
        }

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        log::warn!("WGPU setup");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        log::warn!("device and queue");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                // Some(&std::path::Path::new("trace")), // Trace path
                None, // Trace path
            )
            .await
            .unwrap();

        log::warn!("Surface");
        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        let diffuse_bytes = include_bytes!("texture_atlas.png");
        let diffuse_texture =
            texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "texture_atlas.png")
                .unwrap();

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(30.0, 1.0);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        log::warn!("Load model");
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });
        window.set_visible(true);
        (
            Self {
                surface,
                device,
                queue,
                config,
                size,
                render_pipeline,
                camera,
                projection,
                camera_controller,
                camera_buffer,
                camera_bind_group,
                camera_uniform,
                depth_texture,
                window,
                texture_bind_group: diffuse_bind_group,
                mouse_pressed: false,
            },
            event_loop,
        )
    }
    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }
    pub fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
    pub fn render(&mut self, chunks: &[Chunk; 256]) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            for chunk in chunks {
                render_pass.set_vertex_buffer(0, chunk.mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(chunk.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..chunk.mesh.num_elements, 0, 0..1);
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
    pub fn build_mesh(&self, vertices: Vec<Vertex>, indices: Vec<u32>) -> Mesh {
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX,
            });
        Mesh {
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
        }
    }
    pub fn build_chunk(
        &self,
        blocks: &Vec<Vec<Vec<Block>>>,
        x_offset: f32,
        z_offset: f32,
        left_chunk: Option<&Vec<Vec<Vec<Block>>>>,
        right_chunk: Option<&Vec<Vec<Vec<Block>>>>,
        front_chunk: Option<&Vec<Vec<Vec<Block>>>>,
        back_chunk: Option<&Vec<Vec<Vec<Block>>>>,
    ) -> Mesh {
        let mut vertices: Vec<Vertex> = vec![];
        let mut indices: Vec<u32> = vec![];
        
        //vars in for loop code, preinitialized
        let mut grass_above;
        let mut neighbor_chunk_block_option;
        let mut base_index;
        let mut face;
        let mut neighbor;
        for (x, column) in blocks.iter().enumerate() {
            for (y, row) in column.iter().enumerate() {
                for (z, block) in row.iter().enumerate() {
                    //init code
                    if let BlockType::Air = block.block_type {
                        continue;
                    }
                    let pos = [x as f32 + x_offset, y as f32, z as f32 + z_offset];
                    grass_above = y + 1 < column.len() && matches!(blocks[x][y + 1][z].block_type, BlockType::Grass);

                    //block rendering
                    base_index = vertices.len() as u32;
                    face = Face::Top;
                    neighbor = if y + 1 < column.len() {Some(&blocks[x][y + 1][z])} else {None};
                    get_block_face(base_index,face, neighbor, block, pos, &mut vertices, &mut indices,false, None);

                    base_index = vertices.len() as u32;
                    face = Face::Bottom;
                    neighbor = if y > 0 {Some(&blocks[x][y - 1][z])} else {None};
                    get_block_face(base_index,face, neighbor, block, pos, &mut vertices, &mut indices,false, None);

                    base_index = vertices.len() as u32;
                    face = Face::Left; //this is actually front i think
                    neighbor = if x > 0 {Some(&blocks[x - 1][y][z])} else {None};
                    neighbor_chunk_block_option = left_chunk.map_or(None, |chunk| Some(&chunk[15][y][z]));
                    get_block_face(base_index,face, neighbor, block, pos, &mut vertices, &mut indices,grass_above, neighbor_chunk_block_option);
                    
                    base_index = vertices.len() as u32;
                    face = Face::Right;
                    neighbor = if x + 1 < blocks.len() {Some(&blocks[x + 1][y][z])} else {None};
                    neighbor_chunk_block_option = right_chunk.map_or(None, |chunk| Some(&chunk[0][y][z]));
                    get_block_face(base_index,face, neighbor, block, pos, &mut vertices, &mut indices,grass_above, neighbor_chunk_block_option);

                    base_index = vertices.len() as u32;
                    face = Face::Front;
                    neighbor = if z + 1 < row.len() {Some(&blocks[x][y][z + 1])} else {None};
                    neighbor_chunk_block_option = front_chunk.map_or(None, |chunk| Some(&chunk[x][y][0]));
                    get_block_face(base_index,face, neighbor, block, pos, &mut vertices, &mut indices,grass_above, neighbor_chunk_block_option);

                    base_index = vertices.len() as u32;
                    face = Face::Back;
                    neighbor = if z > 0 {Some(&blocks[x][y][z - 1])} else {None};
                    neighbor_chunk_block_option = back_chunk.map_or(None, |chunk| Some(&chunk[x][y][15]));
                    get_block_face(base_index,face, neighbor, block, pos, &mut vertices, &mut indices,grass_above, neighbor_chunk_block_option);
                }
            }
        }
        self.build_mesh(vertices, indices)
        //better technique, start in the middle and work your way out?
    }
}
fn get_block_face(base_index: u32, face: Face, neighbor_block_option: Option<&Block>, block: &Block, pos: [f32; 3], vertices: &mut Vec<Vertex>, indices: &mut Vec<u32>, grass_above: bool, neighbor_chunk_block_option: Option<&Block>){
    let mut render = false;
    match neighbor_block_option {
        Some(neighbor_block) => {
            if let BlockType::Air = neighbor_block.block_type {
                vertices.extend_from_slice(&get_mesh_texture_and_pos(
                    face,
                    &block.block_type,
                    pos,
                    grass_above,
                ));
                render = true;
            }
            //otherwise the neighboring block is a solid block so you don't need to render
        }
        None => {
            match neighbor_chunk_block_option {
                Some(neighbor_chunk_block) => {
                    if let BlockType::Air = neighbor_chunk_block.block_type {
                        vertices.extend_from_slice(&get_mesh_texture_and_pos(
                            face,
                            &block.block_type,
                            pos,
                            grass_above,
                        ));
                        render = true;
                    }
                    //otherwise the neighboring chunk's block is a solid block so you don't need to render
                }
                None => {}
            }
        }
    }
    if render {
        indices.push(base_index + 3);
        indices.push(base_index + 2);
        indices.push(base_index);
        indices.push(base_index + 1);
        indices.push(base_index + 2);
        indices.push(base_index + 3);
    }
}
fn get_mesh_texture_and_pos(
    face: Face,
    block_type: &BlockType,
    pos: [f32; 3],
    grass_above: bool,
) -> Vec<Vertex> {
    let vertices = match face {
        Face::Top => [
            [pos[0] - 0.5, pos[1] + 0.5, pos[2] - 0.5],
            [pos[0] + 0.5, pos[1] + 0.5, pos[2] + 0.5],
            [pos[0] + 0.5, pos[1] + 0.5, pos[2] - 0.5],
            [pos[0] - 0.5, pos[1] + 0.5, pos[2] + 0.5],
        ],
        Face::Bottom => [
            [pos[0] + 0.5, pos[1] - 0.5, pos[2] - 0.5],
            [pos[0] - 0.5, pos[1] - 0.5, pos[2] + 0.5],
            [pos[0] - 0.5, pos[1] - 0.5, pos[2] - 0.5],
            [pos[0] + 0.5, pos[1] - 0.5, pos[2] + 0.5],
        ],
        Face::Left => [
            [pos[0] - 0.5, pos[1] - 0.5, pos[2] + 0.5],
            [pos[0] - 0.5, pos[1] + 0.5, pos[2] - 0.5],
            [pos[0] - 0.5, pos[1] - 0.5, pos[2] - 0.5],
            [pos[0] - 0.5, pos[1] + 0.5, pos[2] + 0.5],
        ],
        Face::Right => [
            [pos[0] + 0.5, pos[1] - 0.5, pos[2] - 0.5],
            [pos[0] + 0.5, pos[1] + 0.5, pos[2] + 0.5],
            [pos[0] + 0.5, pos[1] - 0.5, pos[2] + 0.5],
            [pos[0] + 0.5, pos[1] + 0.5, pos[2] - 0.5],
        ],
        Face::Front => [
            [pos[0] + 0.5, pos[1] - 0.5, pos[2] + 0.5],
            [pos[0] - 0.5, pos[1] + 0.5, pos[2] + 0.5],
            [pos[0] - 0.5, pos[1] - 0.5, pos[2] + 0.5],
            [pos[0] + 0.5, pos[1] + 0.5, pos[2] + 0.5],
        ],
        Face::Back => [
            [pos[0] - 0.5, pos[1] - 0.5, pos[2] - 0.5],
            [pos[0] + 0.5, pos[1] + 0.5, pos[2] - 0.5],
            [pos[0] + 0.5, pos[1] - 0.5, pos[2] - 0.5],
            [pos[0] - 0.5, pos[1] + 0.5, pos[2] - 0.5],
        ],
    };
    let index = match block_type {
        BlockType::Grass => match face {
            Face::Left | Face::Right | Face::Back | Face::Front => {
                if grass_above {
                    1
                } else {
                    2
                }
            }
            Face::Top => 3,
            Face::Bottom => 1,
        },
        _ => todo!(),
    };

    let texture_coords = get_texture_coords(index);
    let mut vertices_array = vec![];
    for i in 0..4 {
        vertices_array.push(Vertex {
            position: vertices[i],
            tex_coords: texture_coords[i],
        })
    }

    vertices_array
}
fn get_texture_coords(index: usize) -> [[f32; 2]; 4] {
    const NUM_SPRITES_IN_TEXTURE: usize = 16; //must be perfect square
    const SPRITE_SIZE: f32 = 1.0 / (NUM_SPRITES_IN_TEXTURE as f32);

    let row = index / NUM_SPRITES_IN_TEXTURE;
    let col = index % NUM_SPRITES_IN_TEXTURE;

    let min_x = col as f32 * SPRITE_SIZE;
    let max_x = min_x + SPRITE_SIZE;
    let min_y = row as f32 * SPRITE_SIZE;
    let max_y = min_y + SPRITE_SIZE;
    [
        [min_x, min_y],
        [max_x, max_y],
        [min_x, max_y],
        [max_x, min_y],
    ]
}
enum Face {
    Top,
    Bottom,
    Left,
    Right,
    Back,
    Front,
}
