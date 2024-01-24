use nalgebra as na;

use image::{ImageBuffer, Rgb};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

const UP: FVec = na::Vector3::new(0.0, 0.0, 1.0);

type FVec = na::Vector3<f32>;

#[derive(Debug)]
struct Ray {
    origin: FVec,
    direction: FVec,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Material {
    colour: FVec,
    // TODO: BP and stuff
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LightSource {
    colour: FVec,
    pos: FVec,
}

struct Intersection {
    t: f32,
    pos: FVec,
    normal: FVec,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
enum Shape {
    Sphere { centre: FVec, radius: f32 },
}

impl Shape {
    /*
    Return smallest t >= 0 such that P is on surface of self, where:
        P = ray.origin + ray.direction * t
    If no such t exists, return None
     */
    fn intersection(&self, ray: &Ray, min_distance: f32) -> Option<Intersection> {
        match self {
            Shape::Sphere { centre, radius } => {
                let a = ray.direction.norm_squared();
                let difference = ray.origin - centre;
                let b = 2.0 * ray.direction.dot(&difference);
                let c = difference.norm_squared() - (radius * radius);
                let discriminant = b * b - 4.0 * a * c;
                if discriminant < 0.0 {
                    return None;
                }
                let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
                let t2 = (-b - discriminant.sqrt()) / (2.0 * a);
                [t1, t2]
                    .iter()
                    .copied()
                    .filter(|t| *t >= min_distance)
                    .min_by(|a, b| a.partial_cmp(&b).unwrap())
                    .map(|t| {
                        let point = ray.origin + t * ray.direction;
                        let normal = (point - centre).normalize();
                        Intersection {
                            t: t,
                            pos: point,
                            normal: normal,
                        }
                    })
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SceneObject {
    material: Material,
    shape: Shape,
}

fn clamp<T: PartialOrd>(x: T, min: T, max: T) -> T {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

fn channel_float_to_int(value: f32) -> u8 {
    let integer = (value * 255.0) as i32;
    clamp(integer, 0, 255) as u8
}

impl SceneObject {
    fn intersect(&self, ray: &Ray, min_distance: f32) -> Option<(Intersection, Rgb<f32>)> {
        self.shape.intersection(ray, min_distance).map(|t| {
            let [r, g, b] = self.material.colour.as_slice() else {
                panic!("Unable to unpack RGB values from colour")
            };
            (t, Rgb([*r, *g, *b]))
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Camera {
    position: FVec,
    direction: FVec,
    screen_distance: f32,
    screen_width: f32,
    screen_height: f32,
    screen_columns: u32,
    screen_rows: u32,
}

impl Camera {
    fn get_basis_vectors(&self) -> (FVec, FVec, FVec) {
        let u = self.direction.normalize();
        let v = u.cross(&UP);
        let w = v.cross(&u);
        (u, v, w)
    }

    fn get_ray(&self, x: u32, y: u32) -> Ray {
        // Center of screen is origin
        let x_screen = ((x as i64) - (self.screen_columns as i64 / 2)) as f32
            / self.screen_columns as f32
            * self.screen_width
            * 0.5;
        let y_screen = ((y as i64) - (self.screen_rows as i64 / 2)) as f32
            / self.screen_rows as f32
            * self.screen_height
            * -0.5;
        let (u, v, w) = self.get_basis_vectors();
        let s = (self.screen_distance * u) + (x_screen * v) + (y_screen * w);
        Ray {
            origin: self.position,
            direction: s - self.position,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Scene {
    camera: Camera,
    global_illumination: FVec,
    lights: Vec<LightSource>,
    objects: Vec<SceneObject>,
}

impl Scene {
    fn from_file(path: &str) -> Result<Scene, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let scene = serde_json::from_reader(reader)?;
        Ok(scene)
    }

    fn get_illumination(&self, p: Intersection) -> Rgb<f32> {
        let mut value = self.global_illumination;
        for light in self.lights.iter() {
            let ray = Ray {
                origin: p.pos,
                direction: (light.pos - p.pos).normalize(),
            };
            let t = self.get_intersection(&ray, 0.1);
            if t.is_some() {
                continue;
            }
            let coeff = clamp(p.normal.angle(&ray.direction).cos(), 0., 1.);
            for i in 0..3 {
                value[i] += coeff * light.colour[i];
            }
        }
        Rgb(std::array::from_fn(|i| value[i]))
    }

    fn get_intersection(&self, ray: &Ray, min_distance: f32) -> Option<(Intersection, Rgb<f32>)> {
        self.objects
            .iter()
            .filter_map(|object| object.intersect(ray, min_distance))
            .min_by(|a, b| a.0.t.partial_cmp(&b.0.t).unwrap())
    }

    fn get_pixel(&self, x: u32, y: u32) -> Rgb<f32> {
        let ray = self.camera.get_ray(x, y);
        // println!("{:?}", ray);
        self.get_intersection(&ray, 0.0)
            .map(|p| (self.get_illumination(p.0), p.1))
            .map(|colour| {
                let a = colour.0;
                let b = std::array::from_fn(|i| a[i] * colour.1[i]);
                Rgb(b)
            })
            .unwrap_or(Rgb([0.0, 0.0, 0.0]))
    }
}

fn main() {
    let scene = Scene::from_file("scene.json").unwrap();
    println!("{:?}", scene);
    let image = ImageBuffer::from_fn(
        scene.camera.screen_columns,
        scene.camera.screen_rows,
        |x, y| {
            let rgb = scene.get_pixel(x, y).0.map(channel_float_to_int);
            Rgb(rgb)
        },
    );
    image.save("output.png").unwrap();
}
