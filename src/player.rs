use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::event::*;

use crate::camera::Camera;
use crate::{Block, BlockType, Chunk};
const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;
pub struct Player {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    speed: f32,
    fall_speed: f32,
    sensitivity: f32,
    jump: bool,
    jump_am: f32,
    local_pos: Point3<f32>,
    world_pos: Point3<usize>,
}
impl Player {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            speed,
            fall_speed: 30.0,
            sensitivity,
            jump: false,
            jump_am: 0.0,
            local_pos: (0.5, 0.5, 0.0).into(), //x, y, z
            world_pos: (30, 29, 30).into(), //x, y, z, actually an i32 but i cant represent it cus i need to add to local_pos
        }
    }
    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.amount_forward = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.amount_backward = amount;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.jump = true;
                self.jump_am = 1.0;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }
    pub fn update_player(&mut self, camera: &mut Camera, dt: Duration, chunks: &mut [Chunk; 256]) {
        self.update_camera(camera, dt, chunks);
    }
    fn update_camera(&mut self, camera: &mut Camera, dt: Duration, chunks: &mut [Chunk; 256]) {
        let dt = dt.as_secs_f32();

        // Move forward/backward and left/right
        let (yaw_sin, yaw_cos) = camera.yaw.0.sin_cos();
        let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        //get chunks
        let cur_chunk_index = (self.world_pos.z / 16) + (16 * (self.world_pos.x / 16));
        let cur_chunk = &chunks[cur_chunk_index];
        let front_chunk = &chunks[cur_chunk_index - 16];
        let back_chunk = &chunks[cur_chunk_index + 16];
        let left_chunk = &chunks[cur_chunk_index + 1];
        let right_chunk = &chunks[cur_chunk_index - 1];
        //get transforms
        let forward_am = forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        let right_am = right * (self.amount_right - self.amount_left) * self.speed * dt;
        let move_am = forward_am + right_am;
        //check if can move right
        let (block_right_bottom, block_right_top) = if (self.world_pos.z % 16) as isize - 1 < 0 {
            (
                left_chunk.blocks[self.world_pos.x % 16][self.world_pos.y - 1][15],
                left_chunk.blocks[self.world_pos.x % 16][self.world_pos.y][15],
            )
        } else {
            (
                cur_chunk.blocks[self.world_pos.x % 16][self.world_pos.y - 1]
                    [(self.world_pos.z % 16) - 1],
                left_chunk.blocks[self.world_pos.x % 16][self.world_pos.y]
                    [(self.world_pos.z % 16) - 1],
            )
        };
        println!(
            "{:#?}",
            self.local_pos.x < 0.1
            && (block_right_bottom.is_solid || block_right_top.is_solid)
        );
        if !(self.local_pos.x < 0.1
            && (block_right_bottom.is_solid || block_right_top.is_solid)
            && move_am.x > 0.01)
        {
            self.local_pos += move_am;
        }
        if self.local_pos.x > 1.0 {
            self.local_pos.x -= 1.0;
            self.world_pos.x += 1;
        }
        if self.local_pos.z > 1.0 {
            self.local_pos.z -= 1.0;
            self.world_pos.z += 1;
        }
        if self.local_pos.x < -1.0 {
            self.local_pos.x += 1.0;
            self.world_pos.x -= 1;
        }
        if self.local_pos.z < -1.0 {
            self.local_pos.z += 1.0;
            self.world_pos.z -= 1;
        }
        let block_bottom =
            cur_chunk.blocks[self.world_pos.x % 16][self.world_pos.y - 2][self.world_pos.z % 16];
        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.
        if let BlockType::Air = block_bottom.block_type {
            self.local_pos.y -= self.fall_speed * dt;
            if self.local_pos.y < -1.0 {
                self.local_pos.y += 1.0;
                self.world_pos.y -= 1;
            }
        }

        // Rotate
        camera.yaw += Rad(self.rotate_horizontal) * self.sensitivity * dt;
        camera.pitch += Rad(-self.rotate_vertical) * self.sensitivity * dt;

        // If process_mouse isn't called every frame, these values
        // will not get set to zero, and the camera will rotate
        // when moving in a non cardinal direction.
        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Keep the camera's angle from going too high/low.
        if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
            camera.pitch = -Rad(SAFE_FRAC_PI_2);
        } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
            camera.pitch = Rad(SAFE_FRAC_PI_2);
        }
        camera.position.x = self.local_pos.x + self.world_pos.x as f32;
        camera.position.y = self.local_pos.y + self.world_pos.y as f32;
        camera.position.z = self.local_pos.z + self.world_pos.z as f32;
    }
}
