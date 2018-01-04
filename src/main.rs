extern crate generic_array;
extern crate nalgebra as na;
extern crate typenum;
extern crate alga;
extern crate libc;
extern crate ansi_term;
extern crate time;



use na::{Vector2,Vector3,Point2,Point3,Vector4};
use na::geometry::{Similarity,Similarity2,Translation2};

mod graphics;
mod graphics_util;
mod renderer;
mod math;
mod voxel_renderer;
mod dc;

use graphics::*;
use std::ptr;
use std::fs;
use std::fs::File;
use std::vec::*;
use std::collections::HashMap;
use graphics_util::*;
use std::io::Read;
use renderer::*;
use math::*;
use voxel_renderer::*;
use std::ops::*;

use time::precise_time_ns;

fn timed<T>(str_fn: &(Fn(u64) -> String), f : &mut (FnMut() -> T)) -> T{
    let t1 = precise_time_ns();
    let ret = f();
    let t2 = precise_time_ns();

    let dt = t2 - t1;

    println!("{}", str_fn(dt));

    ret
}

//F3 : FnMut(A) -> C
fn compose<'l, A, B, C, F1, F2>(f1 : & 'l Box<F1>, f2 : &'l Box<F2>) -> Box<Fn(A) -> C + 'l>
    where F1 : 'l + Fn(A) -> B,
          F2 : 'l + Fn(B) -> C,
          {
    Box::new(move |a : A| {(*f2)((*f1)(a))})
}



extern fn framebuf_sz_cb(win : *mut GlfwWindow, w : isize, h : isize){
    gl_viewport(0,0,w,h);
}

extern fn error_cb(n : isize, er : &str){
    println!("{}", er);
}

fn check_for_gl_errors(){
    let mut er: usize = gl_get_error();

    while er != GL_NO_ERROR{
        eprintln!("GL error: {}", er);
        er = gl_get_error();
    }
}

fn update_win_dim_info(info: &mut WindowInfo){
    let mut w: usize = 0;
    let mut h: usize = 0;

    glfw_get_window_size(info.handle, &mut w, &mut h);
    info.width = w;
    info.height = h;
}

fn process_input(win : *mut GlfwWindow){
    if glfw_get_key(win, GLFW_KEY_ESCAPE) == GLFW_PRESS{
        glfw_set_window_should_close(win, true);
    }
    else if glfw_get_key(win, GLFW_KEY_TAB) == GLFW_PRESS{
        //debug

        let mut w : usize = 0;
        let mut h : usize = 0;

        glfw_get_window_size(win, &mut w, &mut h);

        println!("({}, {})", w, h);

        let mon = glfw_get_primary_monitor();
        let vid_mode = glfw_get_video_mode(mon);
        unsafe{
            println!("{:?}", *vid_mode)
        }
    }
}


fn load_shaders_vf() -> HashMap<String, Program>{
    let dir : &str = "./assets/shaders/";
    let paths = fs::read_dir(dir).unwrap();
    let mut map : HashMap<String, Program> = HashMap::new();
    
    for entry in paths{
        let name : String = String::from(entry
            .unwrap()
            .path()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap());

        if !map.contains_key(&name){
            let mut file_vert = File::open(
                dir.to_string() + &name + ".vert").unwrap();
            let mut source_vert = String::new();
            file_vert.read_to_string(&mut source_vert).unwrap();

            let mut file_frag = File::open(
                dir.to_string() + &name + ".frag").unwrap();
            let mut source_frag = String::new();
            file_frag.read_to_string(&mut source_frag).unwrap();

            let prog = create_program_vf(
                &source_vert,
                &source_frag);
            
            
            map.insert(name, Program{id: prog});
        }
    }
    
    map
}

fn main() {
    let def_width: usize = 800;
    let def_height: usize = 600;

    //TODO check if it works
    glfw_set_error_callback(error_cb);
    glfw_init();
    glfw_window_hint(GLFW_CONTEXT_VERSION_MAJOR, 3);
    glfw_window_hint(GLFW_CONTEXT_VERSION_MINOR, 3);
    glfw_window_hint(GLFW_OPENGL_PROFILE, GLFW_OPENGL_CORE_PROFILE);

    let win = glfw_create_window(def_width, def_height, "Voxelized2D");

    if win == ptr::null_mut(){
        glfw_terminate();
        panic!("Failed to create GLFW window !");
    }

    glfw_make_context_current(win);
    glad_load_gl_loader();

    println!("Using GL version: {}", gl_get_string(GL_VERSION));
    
    glfw_set_framebuffer_size_callback(win, framebuf_sz_cb);


    let shaders = load_shaders_vf();
    let mut voxel_renderer = VoxelRenderer::new(&shaders);
    let mut win_info = WindowInfo{width: def_width, height: def_height, handle: win}; //will be updated each frame

    let test_tr = Triangle3{p1: Vector3::new(0.0,0.0, 0.0),
                           p2: Vector3::new(16.0, 0.0, 0.0),
                           p3: Vector3::new(8.0, 16.0, 0.0)};

    let mut renderer = RendererVertFragDef::make(
        VERTEX_SIZE_COLOR,
        set_attrib_ptrs_color,
        GL_TRIANGLES,
        String::from("color"));

    //add_tringle_color(&mut renderer, test_tr, Vector3::new(1.0,0.0,0.0));


    let BLOCK_SIZE : f32 = 0.125;
    let CHUNK_SIZE : usize = 128;

    let mut grid = VoxelGrid2::new(BLOCK_SIZE, CHUNK_SIZE, CHUNK_SIZE);

    let offset = Vector2::new(0.1, 0.1);

    let circle1 = mk_circle2(Vector2::new(4.0 as f32,8.0) + offset, 2.0);
    let circle2 = mk_circle2(Vector2::new(8.0 as f32,8.0) + offset, 5.0);
    let circle3 = mk_circle2(Vector2::new(4.0 as f32,4.0) + offset, 2.0);
    let circle4 = mk_circle2(Vector2::new(8.0 as f32,12.0) + offset, 4.0);
    let circle5 = mk_circle2(Vector2::new(8.0 as f32,6.0) + offset, 1.1);

    let rec = mk_rectangle2(Vector2::new(8.0 as f32, 10.8) + offset, Vector2::new(1.0, 3.0));

    let i1 = union(circle1, circle2);
    let i2 = union(i1, rec);
    let i3 = difference(i2, circle3);
    let i4 = difference(i3, circle4);
    let i5 = difference(i4, circle5);
    //let i6 = union(i5, rec);

    /*let contour_data = timed(&|dt| format!("op took {} ms", dt / 1000000), &mut ||{
        fill_in_grid(&mut grid, &i5, Vector2::new(0.0, 0.0));
        make_contour(&grid, &i5, 32)
    });

    for tr in &contour_data.triangles{
        add_triangle_color(&mut renderer, &Triangle3{p1 : Vector3::new(tr.p1.x, tr.p1.y, 0.0), p2 : Vector3::new(tr.p2.x, tr.p2.y, 0.0), p3 : Vector3::new(tr.p3.x, tr.p3.y, 0.0)}, Vector3::new(1.0,1.0,0.0))
    }
    */


    //dc::test_sample_normal();


    fn shader_data(shader: &Program, win: &WindowInfo){
        let aspect = win.width as f32 / win.height as f32;
        let height = 16.0;
        let width = height;
        let id_mat = [
            1.0,0.0,0.0,0.0,
            0.0,1.0,0.0,0.0,
            0.0,0.0,1.0,0.0,
            0.0,0.0,0.0,1.0];

        let cam_world_pos = Vector3::new(0.0, 0.0, 0.0);
        let m = na::Translation::from_vector(-cam_world_pos);

        shader.set_float4x4("P", false, na::geometry::Orthographic3::new(0.0, width, 0.0, height, -1.0, 1.0).to_homogeneous().as_slice());
        shader.set_float4x4("V", false, m.to_homogeneous().as_slice());

    }

    let provider = RenderDataProvider{pre_render_state: None, post_render_state: None, shader_data: Some(Box::new(shader_data))};



    let mut render_info = RenderInfo{renderer: Box::new(renderer), provider};//moved


    let id = voxel_renderer.push(RenderLifetime::Manual, RenderTransform::None, render_info).unwrap();



    voxel_renderer.manual_mut(&id).construct();

    while !glfw_window_should_close(win){
        update_win_dim_info(&mut win_info);
        process_input(win);

        gl_clear_color(0.2, 0.3, 0.3, 1.0);
        gl_clear(GL_COLOR_BUFFER_BIT);

        voxel_renderer.draw(&win_info);


        glfw_swap_buffers(win);
        glfw_poll_events();

        check_for_gl_errors();
    }

    voxel_renderer.manual_mut(&id).deconstruct();
    voxel_renderer.manual_mut(&id).reset();

    glfw_terminate();
}

fn calc_qef(point : &Vector2<f32>, lines : &Vec<Line2<f32>>) -> f32{
    let mut qef : f32 = 0.0;
    for line in lines{
        let dist = distance_point2_line2(point, line);
        qef += dist * dist;
    }

    qef
}

fn const_sign(a : f32, b : f32) -> bool {
    if a > 0.0 { b > 0.0} else {b <= 0.0}
}

fn sample_qef_brute(square : Square2<f32>, n : usize, lines : &Vec<Line2<f32>>) -> Vector2<f32> {
    let ext = Vector2::new(square.extent, square.extent);
    let min = square.center - ext;

    let mut best_qef = 100000000000.0; //TODO placeholder
    let mut best_point = min;

    for i in 0..n{
        for j in 0..n{
            let point = min + Vector2::new(ext.x * (2.0 * (i as f32) + 1.0) / (n as f32), ext.y * (2.0 * (j as f32) + 1.0) / (n as f32));
            let qef = calc_qef(&point, &lines);

            if qef < best_qef{
                best_qef = qef;
                best_point = point;
            }
        }
    }

    best_point
}


fn sample_intersection_brute(line : Line2<f32>, n : usize, f : &DenFn2<f32>) -> Vector2<f32>{
    let ext = line.end - line.start;

    let mut best_abs = 1000000000.0; //TODO placeholder
    let mut best_point : Option<Vector2<f32>> = None;

    for i in 0..n {
        let point = line.start + ext * (i as f32 / n as f32);
        let den = f(point);
        let abs = den.abs();

        if abs < best_abs {
            best_abs = abs;
            best_point = Some(point);
        }
    }

    best_point.unwrap()
}

fn sample_tangent(square : Square2<f32>, n : usize, f : &DenFn2<f32>) -> Vector2<f32>{
    let ext = Vector2::new(square.extent, square.extent);
    let min = square.center - ext;

    let den_at_center = f(square.center);

    let mut closest = den_at_center + 100000000.0; //TODO placeholder\
    let mut closest_point = square.center;

    for i in 0..n{
        for j in 0..n{
            let point = min + Vector2::new(ext.x * (2.0 * i as f32) / n as f32,
                ext.y * (2.0 * j as f32) / n as f32);
            let den = f(point);
            let attempt = (den - den_at_center).abs();
            if attempt < closest && (point - square.center).norm() != 0.0{
                closest = attempt;
                closest_point = point;
            }
        }
    }

    closest_point - square.center
}

fn ext_for_normal(block_size : f32) -> f32 {block_size / 100.0} //TODO why so ?


fn make_lines(vg : &VoxelGrid2<f32>, features : &Vec<Option<Vector2<f32>>>) -> Vec<Line2<f32>>{
    let mut ret = Vec::<Line2<f32>>::new();

    for y in 0..vg.size_y - 1{
        for x in 0..vg.size_x - 1{
            let feature = features[y * vg.size_x + x];
            if feature.is_some(){
                let p1 = vg.get(x + 1, y);
                let p2 = vg.get(x, y + 1);
                let p3 = vg.get(x + 1, y + 1);

                let mut vert1 : Option<Vector2<f32>> = None;
                let mut vert2 : Option<Vector2<f32>> = None;

                if !const_sign(p1,p3){
                    vert1 = features[y * vg.size_x + (x + 1)];
                }
                if !const_sign(p3,p2){
                    vert2 = features[(y+1) * vg.size_x + x];
                }

                if vert1.is_some(){
                    ret.push(Line2{start : feature.unwrap(), end : vert1.unwrap()});
                }
            }
        }
    }

    ret
}

fn make_triangles(vg : &VoxelGrid2<f32>, features : &Vec<Option<Vector2<f32>>>, intersections : &Vec<Option<Vec<Vector2<f32>>>>,
    extra : &Vec<Option<Vec<Vector2<f32>>>>) -> Vec<Triangle2<f32>>{
    let mut ret = Vec::<Triangle2<f32>>::new();

    for y in 0..vg.size_y{
        for x in 0.. vg.size_x{
            let t = y * vg.size_x + x;
            let cur_intersections = &intersections[t];
            let cur_extras = &extra[t];

            let p0 = vg.get(x, y);
            let p1 = vg.get(x + 1, y);
            let p2 = vg.get(x, y + 1);
            let p3 = vg.get(x + 1, y + 1);

            let v0 = vg.get_point(x,y);
            let v1 = vg.get_point(x + 1, y);
            let v2 = vg.get_point(x, y + 1);
            let v3 = vg.get_point(x + 1, y + 1);

            let mut sit = 0;

            if !const_sign(p0, p1){sit |= 1;}
            if !const_sign(p1, p3){sit |= 2;}
            if !const_sign(p3, p2){sit |= 4;}
            if !const_sign(p2, p0){sit |= 8;}

            if sit == 0{ //fully inside or fully outside
                let negative = p0 < 0.0;

                if negative{ //render if it is inside
                    let tr1 = Triangle2{p1: v0, p2 : v1, p3 : v3};
                    let tr2 = Triangle2{p1: v0, p2 : v3, p3 : v2};

                    ret.push(tr1);
                    ret.push(tr2);
                }

            }else{ //contains surface
                if cur_intersections.is_some() && features[t].is_some(){
                    let len = cur_intersections.as_ref().unwrap().len();
                    for i in 0..len{
                        ret.push(Triangle2{p1 : features[t].as_ref().unwrap().clone(), p2 : cur_intersections.as_ref().unwrap()[i].clone(), p3 : cur_extras.as_ref().unwrap()[i].clone()});
                    }
                }
            }
        }
    }

    ret
}


fn make_vertex(vg : &VoxelGrid2<f32>, tr : &mut Vec<Triangle2<f32>>, x : usize, y : usize,
    f : &DenFn2<f32>, accuracy : usize, features : &mut Vec<Option<Vector2<f32>>>, out_intersections : &mut Vec<Vector2<f32>>, out_extra : &mut Vec<Vector2<f32>>) -> Option<Vector2<f32>>{
    let epsilon = vg.a / accuracy as f32;

    let p0 = vg.get(x, y);
    let p1 = vg.get(x + 1, y);
    let p2 = vg.get(x, y + 1);
    let p3 = vg.get(x + 1, y + 1);

    let v0 = vg.get_point(x,y);
    let v1 = vg.get_point(x + 1, y);
    let v2 = vg.get_point(x, y + 1);
    let v3 = vg.get_point(x + 1, y + 1);

    let mut sit = 0;

    if !const_sign(p0, p1){sit |= 1;}
    if !const_sign(p1, p3){sit |= 2;}
    if !const_sign(p3, p2){sit |= 4;}
    if !const_sign(p2, p0){sit |= 8;}

    let ext_for_normal = ext_for_normal(vg.a);

    if sit > 0{
        let mut tangents = Vec::<Line2<f32>>::new();

        let mut vert1 : Option<Vector2<f32>> = None;
        let mut vert2 : Option<Vector2<f32>> = None;

        {
            let mut worker = |and : usize, v_a : Vector2<f32>, v_b : Vector2<f32>, p_a : f32, p_b : f32|{
                if (sit & and) > 0{
                    let ip = sample_intersection_brute(Line2{start : v_a, end : v_b}, accuracy, f);
                    let full = if p_a <= 0.0 {v_a} else {v_b};
                    let dir = sample_tangent(Square2{center : ip, extent : ext_for_normal}, accuracy, f);
                    let line = Line2{start : ip - dir * (1.0 / ext_for_normal), end : ip + dir * (1.0 / ext_for_normal)};
                    tangents.push(line);

                    out_intersections.push(ip);
                    out_extra.push(full);

                }else{
                    let negative = p_a < 0.0;
                    if negative{
                        out_intersections.push(v_a);
                        out_extra.push(v_b);
                    }
                }
            };

            worker(1, v0, v1, p0, p1);
            worker(2, v1, v3, p1, p3);
            worker(4, v3, v2, p3, p2);
            worker(8, v2, v0, p2, p0);
        }

        let interpolated_vertex = sample_qef_brute(vg.square2(x,y), accuracy, &tangents);

        for i in 0..out_intersections.len(){
            tr.push(Triangle2{p1 : interpolated_vertex, p2 : out_intersections[i], p3 : out_extra[i]});
        }

        features[y * vg.size_x + x] = Some(interpolated_vertex);

        Some(interpolated_vertex)
    }else{
        None
    }
}

struct ContourData{
    pub lines : Vec<Line2<f32>>,
    pub triangles : Vec<Triangle2<f32>>,
    pub features : Vec<Option<Vector2<f32>>>,
    pub intersections : Vec<Option<Vec<Vector2<f32>>>>,
    pub extras : Vec<Option<Vec<Vector2<f32>>>>,
}

fn make_contour(vg : &VoxelGrid2<f32>, f : &DenFn2<f32>, accuracy : usize) -> ContourData{
    let mut res1 = Vec::<Line2<f32>>::new();
    let mut res2 = Vec::<Triangle2<f32>>::new();

    let mut features : Vec<Option<Vector2<f32>>> = vec![None;vg.size_x * vg.size_y];
    let mut intersections : Vec<Option<Vec<Vector2<f32>>>> = vec![None;vg.size_x * vg.size_y];
    let mut extras : Vec<Option<Vec<Vector2<f32>>>> = vec![None;vg.size_x * vg.size_y];

    {
        let mut cached_make = |x: usize, y: usize, res2: &mut Vec<Triangle2<f32>>| -> Option<Vector2<f32>>{
            let t = y * vg.size_x + x;
            let possible = features[t];
            if possible.is_none() {
                intersections[t] = Some(Vec::with_capacity(4));//TODO extra mem usage
                extras[t] = Some(Vec::with_capacity(4));

                let ret = make_vertex(vg, res2, x, y, f, accuracy, &mut features, &mut intersections[t].as_mut().unwrap(), &mut extras[t].as_mut().unwrap());
                if ret.is_none() {
                    intersections[t] = None;
                    extras[t] = None;
                }

                ret
            } else {
                possible
            }
        };

        for y in 0..vg.size_y {
            for x in 0..vg.size_x {
                let p0 = vg.get(x, y);
                let p1 = vg.get(x + 1, y);
                let p2 = vg.get(x, y + 1);
                let p3 = vg.get(x + 1, y + 1);

                let v0 = vg.get_point(x, y);
                let v1 = vg.get_point(x + 1, y);
                let v2 = vg.get_point(x, y + 1);
                let v3 = vg.get_point(x + 1, y + 1);

                let mut sit = 0;

                if !const_sign(p0, p1) { sit |= 1; }
                if !const_sign(p1, p3) { sit |= 2; }
                if !const_sign(p3, p2) { sit |= 4; }
                if !const_sign(p2, p0) { sit |= 8; }

                if sit > 0 {
                    let interpolated_vertex = cached_make(x, y, &mut res2).unwrap(); //it is 'some' here

                    let mut vert1: Option<Vector2<f32>> = None;
                    let mut vert2: Option<Vector2<f32>> = None;

                    if (sit & 2) > 0 {
                        if x + 1 < vg.size_x {
                            vert1 = cached_make(x + 1, y, &mut res2);
                        }
                    }
                    if (sit & 4) > 0 {
                        if y + 1 < vg.size_y {
                            vert2 = cached_make(x, y + 1, &mut res2);
                        }
                    }

                    if vert1.is_some() {
                        res1.push(Line2 { start: interpolated_vertex, end: vert1.unwrap() });
                    }
                    if vert2.is_some() {
                        res1.push(Line2 { start: interpolated_vertex, end: vert2.unwrap() });
                    }
                } else {
                    let negative = p0 < 0.0;

                    if negative {
                        let tr1 = Triangle2 { p1: v0, p2: v1, p3: v3 };
                        let tr2 = Triangle2 { p1: v0, p2: v3, p3: v2 };

                        res2.push(tr1);
                        res2.push(tr2);
                    }
                }
            }
        }
    }

    ContourData{lines : res1, triangles : res2, features, intersections, extras}

}

fn fill_in_grid(vg : &mut VoxelGrid2<f32>, f : &DenFn2<f32>, point : Vector2<f32>){
    for y in 0..vg.vertices_y(){
        for x in 0..vg.vertices_x(){
            let vx = vg.vertices_x();
            vg.grid[y * vx + x] = f(point + Vector2::new(vg.a * (x as f32), vg.a * (y as f32)));
        }
    }
}

