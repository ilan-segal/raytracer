use nalgebra as na;

use image::{ImageBuffer, Rgb};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

const WIDTH: u32 = 680;
const HEIGHT: u32 = 480;
const UP: na::Vector3<f32> = na::Vector3::new(0.0, 0.0, 1.0);

struct Ray {
    origin: na::Vector3<f32>,
    direction: na::Vector3<f32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Material {
    colour: na::Vector3<f32>,
    // TODO: BP and stuff
}

// trait Shape {
//     /*
//     Return smallest t >= 0 such that P is on surface of self, where:
//         P = ray.origin + ray.direction * t
//     If no such t exists, return None
//      */
//     fn intersection(&self, ray: &Ray) -> Option<f32>;
// }

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
enum Shape {
    Sphere {
        centre: na::Vector3<f32>,
        radius: f32,
    },
}

impl Shape {
    /*
    Return smallest t >= 0 such that P is on surface of self, where:
        P = ray.origin + ray.direction * t
    If no such t exists, return None
     */
    fn intersection(&self, ray: &Ray) -> Option<f32> {
        match self {
            Shape::Sphere { centre, radius } => None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SceneObject {
    material: Material,
    shape: Shape,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Camera {
    position: na::Vector3<f32>,
    direction: na::Vector3<f32>,
    screen_distance: f32,
}

impl Camera {
    fn get_ray(&self, x: u32, y: u32) {
        let x_centered = ((x as i64) - (WIDTH / 2) as i64) as i32;
        let y_centered = ((y as i64) - (HEIGHT / 2) as i64) as i32;
        let origin = 
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Scene {
    camera: Camera,
    objects: Vec<SceneObject>,
}

impl Scene {
    fn from_file(path: &str) -> Result<Scene, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let scene = serde_json::from_reader(reader)?;
        Ok(scene)
    }
}

fn get_colour(x: u32, y: u32, scene: &Scene) -> Rgb<u8> {
    let v = (y % 256) as u8;
    Rgb([v, v, v])
}

fn main() {
    let scene = Scene::from_file("scene.json").unwrap();
    println!("{:?}", scene);
    let image = ImageBuffer::from_fn(WIDTH, HEIGHT, |x, y| get_colour(x, y, &scene));
    image.save("output.png").unwrap();
}
