// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::{mem, os::raw::c_void, ptr};

mod mesh;
mod shader;
mod util;

use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;
use itertools::izip;
use mesh::{Mesh, Terrain, Helicopter};

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  byte_size_of_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(val) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()

// This should:
// * Generate a VAO and bind it
// * Generate a VBO and bind it
// * Fill it with data
// * Configure a VAP for the data and enable it
// * Generate a IBO and bind it
// * Fill it with data
// * Return the ID of the VAO

unsafe fn create_vao(mesh: &Mesh) -> u32 {
    let mut data = Vec::new();
    izip!(
        mesh.vertices.chunks(3),
        mesh.colors.chunks(4),
        mesh.normals.chunks(3)
    )
    .for_each(|(vertex, color, normal)| {
        data.extend_from_slice(vertex);
        data.extend_from_slice(color);
        data.extend_from_slice(normal)
    });

    let mut array = 0;
    gl::GenVertexArrays(1, &mut array);
    gl::BindVertexArray(array);

    let mut buffer = 0;
    gl::GenBuffers(1, &mut buffer);
    gl::BindBuffer(gl::ARRAY_BUFFER, buffer);
    gl::BufferData(
        gl::ARRAY_BUFFER,
        byte_size_of_array(&data),
        pointer_to_array(&data),
        gl::STATIC_DRAW,
    );

    let vertex_index = 0;
    gl::VertexAttribPointer(
        vertex_index,
        3,
        gl::FLOAT,
        gl::FALSE,
        size_of::<f32>() * 10,
        offset::<f32>(0),
    );
    gl::EnableVertexAttribArray(vertex_index);

    let color_index = 1;
    gl::VertexAttribPointer(
        color_index,
        4,
        gl::FLOAT,
        gl::FALSE,
        size_of::<f32>() * 10,
        offset::<f32>(3),
    );
    gl::EnableVertexAttribArray(color_index);

    let normal_index = 2;
    gl::VertexAttribPointer(
        normal_index,
        3,
        gl::FLOAT,
        gl::FALSE,
        size_of::<f32>() * 10,
        offset::<f32>(7),
    );
    gl::EnableVertexAttribArray(normal_index);

    let mut index_buffer = 0;
    gl::GenBuffers(1, &mut index_buffer);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, index_buffer);
    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER,
        byte_size_of_array(&mesh.indices),
        pointer_to_array(&mesh.indices),
        gl::STATIC_DRAW,
    );

    array
}

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(
            INITIAL_SCREEN_W,
            INITIAL_SCREEN_H,
        ));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!(
                "{}: {}",
                util::get_gl_string(gl::VENDOR),
                util::get_gl_string(gl::RENDERER)
            );
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!(
                "GLSL\t: {}",
                util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
            );
        }

        // Position
        let mut pitch = 0_f32;
        let mut yaw = 0_f32;
        let mut x = 0_f32;
        let mut y = 0_f32;
        let mut z = -3_f32;

        // Load models

        let terrain_model = Terrain::load("resources/lunarsurface.obj");
        let terrain_vao = unsafe { create_vao(&terrain_model) };

        let helicopter_model = Helicopter::load("resources/helicopter.obj");

        let helicopter_body_vao = unsafe { create_vao(&helicopter_model.body) };

        let helicopter_door_vao = unsafe { create_vao(&helicopter_model.door) };

        let helicopter_tail_vao = unsafe { create_vao(&helicopter_model.tail_rotor) };

        let helicopter_main_rotor_vao = unsafe { create_vao(&helicopter_model.main_rotor)};



        // == // Set up your shaders here

        unsafe {
            shader::ShaderBuilder::new()
                .attach_file("./shaders/simple.frag")
                .attach_file("./shaders/simple.vert")
                .link()
                .activate();
        }

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;

            let projection = glm::perspective(window_aspect_ratio, 81_f32, 0.1_f32, 100_f32);

            let forward_t = glm::translate(&glm::identity::<f32, 4>(), &glm::vec3(0_f32, 0_f32, z));
            let sideways_t =
                glm::translate(&glm::identity::<f32, 4>(), &glm::vec3(x, 0_f32, 0_f32));
            let up_t = glm::translate(&glm::identity::<f32, 4>(), &glm::vec3(0_f32, y, 0_f32));
            let pitch_t = glm::rotation(pitch, &glm::vec3(1_f32, 0_f32, 0_f32));
            let yaw_t = glm::rotation(yaw, &glm::vec3(0_f32, 1_f32, 0_f32));
            let flip_z = glm::mat4(
                // To fix correct back face
                // culling.
                1_f32, 0_f32, 0_f32, 0_f32, 0_f32, 1_f32, 0_f32, 0_f32, 0_f32, 0_f32, -1_f32, 0_f32,
                0_f32, 0_f32, 0_f32, 1_f32,
            );
            let transformation =
                projection * pitch_t * yaw_t * forward_t * sideways_t * up_t * flip_z;
            unsafe { gl::UniformMatrix4fv(0, 1, gl::FALSE, transformation.as_ptr()) };

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Window was resized to {}x{}", new_size.0, new_size.1);
                    unsafe {
                        gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32);
                    }
                }
            }

            // Handle keyboard input
            let rotation_speed = 0.8_f32;
            let movement_speed = 2_f32;

            let view_rotation = glm::rotation(-yaw, &glm::vec3(0_f32, 1_f32, 0_f32))
                * glm::rotation(-pitch, &glm::vec3(1_f32, 0_f32, 0_f32));
            let forward = view_rotation * glm::vec4(0_f32, 0_f32, 1_f32, 1_f32);
            let right = view_rotation * glm::vec4(1_f32, 0_f32, 0_f32, 1_f32);
            if let Ok(keys) = pressed_keys.lock() {
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html
                        VirtualKeyCode::D => {
                            x += delta_time * movement_speed * right.x;
                            y += delta_time * movement_speed * right.y;
                            z += delta_time * movement_speed * right.z;
                        }
                        VirtualKeyCode::A => {
                            x -= delta_time * movement_speed * right.x;
                            y -= delta_time * movement_speed * right.y;
                            z -= delta_time * movement_speed * right.z;
                        }
                        VirtualKeyCode::W => {
                            x += delta_time * movement_speed * forward.x;
                            y += delta_time * movement_speed * forward.y;
                            z += delta_time * movement_speed * forward.z;
                        }
                        VirtualKeyCode::S => {
                            x -= delta_time * movement_speed * forward.x;
                            y -= delta_time * movement_speed * forward.y;
                            z -= delta_time * movement_speed * forward.z;
                        }
                        VirtualKeyCode::Space => y += delta_time * movement_speed,
                        VirtualKeyCode::LShift => y -= delta_time * movement_speed,
                        VirtualKeyCode::Left => yaw += delta_time * rotation_speed,
                        VirtualKeyCode::Right => yaw -= delta_time * rotation_speed,
                        VirtualKeyCode::Up => {
                            pitch = f32::min(
                                pitch + delta_time * rotation_speed,
                                glm::pi::<f32>() / 2_f32,
                            )
                        }
                        VirtualKeyCode::Down => {
                            pitch = f32::max(
                                pitch - delta_time * rotation_speed,
                                -glm::pi::<f32>() / 2_f32,
                            )
                        }

                        // default handler:
                        _ => {}
                    }
                }
            }
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                // == // Optionally access the accumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                // == // Issue the necessary gl:: commands to draw your scene here

                gl::BindVertexArray(terrain_vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    terrain_model.indices.len() as i32,
                    gl::UNSIGNED_INT,
                    ptr::null(),
                );

                gl::BindVertexArray(helicopter_body_vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    helicopter_model.body.indices.len() as i32,
                    gl::UNSIGNED_INT,
                    ptr::null(),
                );

                gl::BindVertexArray(helicopter_door_vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    helicopter_model.door.indices.len() as i32,
                    gl::UNSIGNED_INT,
                    ptr::null(),
                );

                gl::BindVertexArray(helicopter_tail_vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    helicopter_model.tail_rotor.indices.len() as i32,
                    gl::UNSIGNED_INT,
                    ptr::null(),
                );

                gl::BindVertexArray(helicopter_main_rotor_vao);
                gl::DrawElements(
                    gl::TRIANGLES,
                    helicopter_model.main_rotor.indices.len() as i32,
                    gl::UNSIGNED_INT,
                    ptr::null()
                );
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });

    // == //
    // == // From here on down there are only internals.
    // == //

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                println!(
                    "New window size received: {}x{}",
                    physical_size.width, physical_size.height
                );
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        }
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Q => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}
