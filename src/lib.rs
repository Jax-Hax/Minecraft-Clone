use engine::Mesh;
use winit::{event_loop::ControlFlow, event::{WindowEvent, VirtualKeyCode, ElementState, Event, KeyboardInput, DeviceEvent}};
use crate::engine::State;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod engine;
mod texture;
mod camera;
pub struct Block{
    block_type: BlockType,
}
pub enum BlockType{
    Air,
    Water,
    Grass,
    Stone
}
struct Chunk{
    blocks: Vec<Vec<Vec<Block>>>,
    mesh: Mesh
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {

    // State::new uses async code, so we're going to wait for it to finish
    let (mut state,event_loop) = State::new().await;
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
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
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
                state.update(dt);
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
    let default_chunk = chunk_gen(1);
    let mesh = state.build_chunk(&mut default_chunk);
    let mut chunks: [Chunk; 256] = [Chunk {blocks: chunk_gen(1), state.build_chunk(&mut default_chunk)}; 256];
    for i in 0..255{
        let mut blocks = chunk_gen(1);
        let mesh = state.build_chunk(&mut blocks);
        chunks[i] = Chunk {blocks: blocks, mesh};
    }
    chunks
}
fn chunk_gen(seed: u64) -> Vec<Vec<Vec<Block>>> {
    let mut test_blocks: Vec<Vec<Vec<Block>>> = vec![
        vec![
            vec![
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
        ],
        vec![
            vec![
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
        ],
        vec![
            vec![
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
        ],
        vec![
            vec![
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
            vec![
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
                Block {  block_type: BlockType::Grass },
                Block {block_type: BlockType::Grass },
            ],
        ],
    ];
    test_blocks
}