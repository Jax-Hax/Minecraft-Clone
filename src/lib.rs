use crate::engine::State;
use engine::Mesh;
use noise::{NoiseFn, Perlin};
use std::convert::TryInto;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::ControlFlow,
};
mod camera;
mod engine;
mod texture;
mod player;
#[derive(Copy, Clone, Default,Debug)]
pub struct Block {
    block_type: BlockType,
    is_solid: bool
}
impl Block{
    pub fn new(block_type: BlockType) -> Self {
        let mut is_solid;
        match block_type {
            BlockType::Grass => is_solid = true,
            _ => is_solid = false
        }
        Block { block_type, is_solid }
    }
}
#[derive(Copy, Clone, Default,Debug)]
pub enum BlockType {
    #[default]
    Air,
    Water,
    Grass,
    Stone,
}
pub struct Chunk {
    blocks: Vec<Vec<Vec<Block>>>,
    mesh: Mesh,
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // State::new uses async code, so we're going to wait for it to finish
    let (mut state, event_loop) = State::new().await;
    let mut chunks = create_terrain(&state);

    let mut last_render_time = instant::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => state.window().request_redraw(),
            // NEW!
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => 
                state.player.process_mouse(delta.0, delta.1),
            // UPDATED!
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                let now = instant::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt, &mut chunks);
                match state.render(&chunks) {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            _ => {}
        }
    });
}
fn create_terrain(state: &State) -> [Chunk; 256] {
    let mut chunks = vec![];
    let mut chunk_blocks_vec = vec![];
    let mut chunk_mesh_vec = vec![];
    //gen chunks
    for i in 0..256 {
        let row = (i / 16) * 16;
        let col = (i % 16) * 16;
        chunk_blocks_vec.push(chunk_gen(1, row, col));
    }
    //gen meshes
    for i in 0..256 {
        let row = (i / 16) * 16;
        let col = (i % 16) * 16;
        let blocks = &chunk_blocks_vec[i];
        let mesh = state.build_chunk(
            blocks,
            row as f32,
            col as f32,
            match i.checked_sub(16) {
                //actually front
                Some(j) => chunk_blocks_vec.get(j),
                None => None,
            },
            match i.checked_add(16) {
                //actually back
                Some(j) => chunk_blocks_vec.get(j),
                None => None,
            },
            if (i + 1) % 16 == 0 {
                //actually left
                None
            } else {
                chunk_blocks_vec.get(i + 1)
            },
            match i.checked_sub(1) {
                Some(j) => {
                    if j % 16 == 15 {
                        //actually right
                        None
                    } else {
                        chunk_blocks_vec.get(j)
                    }
                }
                None => None,
            },
        );
        chunk_mesh_vec.push(mesh);
    }
    for _ in 0..256 {
        chunks.push(Chunk {
            blocks: chunk_blocks_vec.remove(0),
            mesh: chunk_mesh_vec.remove(0),
        }) //always takes out the first element
    }
    chunks.try_into().unwrap_or_else(|v: Vec<Chunk>| {
        panic!("Expected a Vec of length 256 but it was {}", v.len())
    })
}
fn chunk_gen(seed: u32, row: i32, col: i32) -> Vec<Vec<Vec<Block>>> {
    let mut test_blocks = vec![];
    let perlin = Perlin::new(seed);
    let x_scale = 0.03;
    let z_scale = 0.03;
    for x in 0..16 {
        //front back
        let mut vec1 = vec![];
        for z in 0..16 {
            //left right
            let mut vec2 = vec![];
            let noise_value =
                (perlin.get([(x + row) as f64 * x_scale, (z + col) as f64 * z_scale]) + 2.0) * 10.0;
            for y in 0..30 {
                //up down
                let block_type = if y < (noise_value) as usize {
                    BlockType::Grass
                } else {
                    BlockType::Air
                };

                vec2.push(Block::new(block_type));
            }
            vec1.push(vec2);
        }

        test_blocks.push(flip_2d_vector(vec1));
    }
    test_blocks
}
fn flip_2d_vector(input: Vec<Vec<Block>>) -> Vec<Vec<Block>> {
    if input.is_empty() {
        return Vec::new();
    }

    let num_rows = input.len();
    let num_cols = input[0].len();

    let mut flipped = vec![
        vec![
            Block::default();
            num_rows
        ];
        num_cols
    ];

    for i in 0..num_rows {
        for j in 0..num_cols {
            flipped[j][i] = input[i][j];
        }
    }

    flipped
}
