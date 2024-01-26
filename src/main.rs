use nalgebra as na;

use image::{ImageBuffer, ImageError, Rgb};
use serde::Deserialize;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

const UP: FVec = na::Vector3::new(0.0, 0.0, 1.0);
const MAX_BOUNCES: u8 = 10;

type Float = f64;
type FVec = na::Vector3<Float>;

#[derive(Debug)]
struct Ray {
    origin: FVec,
    direction: FVec,
}

impl Ray {
    fn extend(&self, t: Float) -> FVec {
        self.origin + t * self.direction
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct Material {
    colour: FVec,
    k_diffuse: Float,
    k_ambient: Float,
    k_specular: Float,
    k_reflect: Float,
    shine: Float,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct LightSource {
    colour: FVec,
    pos: FVec,
}

struct Intersection {
    t: Float,
    pos: FVec,
    normal: FVec,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "type")]
enum Shape {
    Sphere { centre: FVec, radius: Float },
    Plane { point: FVec, normal: FVec },
}

impl Shape {
    /*
    Return smallest t >= 0 such that P is on surface of self, where:
        P = ray.origin + ray.direction * t
    If no such t exists, return None
     */
    fn intersection(&self, ray: &Ray, min_distance: Float) -> Option<Intersection> {
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
                    .filter(|t| *t > min_distance)
                    .min_by(|a, b| a.partial_cmp(&b).unwrap())
                    .map(|t| {
                        let point = ray.extend(t);
                        let normal = (point - centre).normalize();
                        Intersection {
                            t: t,
                            pos: point,
                            normal: normal,
                        }
                    })
            }
            Shape::Plane { point, normal } => {
                let n_dot_d = normal.dot(&ray.direction);
                if n_dot_d == 0.0 {
                    return None;
                }
                let a_minus_p = point - ray.origin;
                let t = normal.dot(&a_minus_p) / n_dot_d;
                if t <= min_distance {
                    None
                } else {
                    Some(Intersection {
                        t: t,
                        pos: ray.extend(t),
                        normal: *normal,
                    })
                }
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

fn channel_float_to_int(value: Float) -> u8 {
    let integer = (value * 255.0) as i32;
    clamp(integer, 0, 255) as u8
}

impl SceneObject {
    fn intersect(&self, ray: &Ray, min_distance: Float) -> Option<Intersection> {
        self.shape
            .intersection(ray, min_distance)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Camera {
    position: FVec,
    direction: FVec,
    screen_distance: Float,
    screen_width: Float,
    screen_height: Float,
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
        let x_screen = ((x as i64) - (self.screen_columns as i64 / 2)) as Float
            / self.screen_columns as Float
            * self.screen_width
            * 0.5;
        let y_screen = ((y as i64) - (self.screen_rows as i64 / 2)) as Float
            / self.screen_rows as Float
            * self.screen_height
            * -0.5;
        let (u, v, w) = self.get_basis_vectors();
        Ray {
            origin: self.position,
            direction: (self.screen_distance * u) + (x_screen * v) + (y_screen * w),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Scene {
    camera: Camera,
    default_colour: FVec,
    ambient_light: FVec,
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

    fn _get_intersection(
        &self,
        ray: &Ray,
        min_distance: Float,
    ) -> Option<(Intersection, Material)> {
        self.objects
            .iter()
            .filter_map(|object| {
                object
                    .intersect(ray, min_distance)
                    .map(|x| (x, object.material))
            })
            .min_by(|a, b| a.0.t.partial_cmp(&b.0.t).unwrap())
    }

    fn _get_diffuse_lighting(
        &self,
        intersection: &Intersection,
        material: &Material,
        light: &LightSource,
        ray: &Ray,
    ) -> FVec {
        let coeff = clamp(intersection.normal.dot(&ray.direction), 0., 1.);
        coeff
            * light
                .colour
                .component_mul(&material.colour)
    }

    fn _get_specular_lighting(
        &self,
        intersection: &Intersection,
        material: &Material,
        light: &LightSource,
        ray: &Ray,
    ) -> FVec {
        let l = light.pos - intersection.pos;
        let v = ray.origin - intersection.pos;
        let h = (l + v).normalize();
        let coeff = h
            .dot(&intersection.normal)
            .powf(material.shine);
        clamp(coeff, 0.0, 1.0) * light.colour
    }

    fn _get_reflection(
        &self,
        intersection: &Intersection,
        material: &Material,
        ray: &Ray,
        num_bounces: u8,
    ) -> FVec {
        if num_bounces > MAX_BOUNCES || material.k_reflect == 0.0 {
            return FVec::zeros();
        }
        let ray_proj_normal = ray.direction.dot(&intersection.normal) * intersection.normal;
        let reflected_ray_direction = ray.direction - 2.0 * ray_proj_normal;
        let reflected_ray = Ray {
            origin: intersection.pos,
            direction: reflected_ray_direction,
        };
        let reflected_ray_colour = self._get_ray_colour(&reflected_ray, 0.0001, num_bounces + 1);
        material.k_reflect * &reflected_ray_colour
    }

    fn _get_surface_point_colour(&self, intersection: &Intersection, material: &Material) -> FVec {
        let ambient = material.k_ambient
            * self
                .ambient_light
                .component_mul(&material.colour);
        let light_dependent_colouring: FVec = self
            .lights
            .iter()
            .filter_map(|light| {
                let ray = Ray {
                    origin: intersection.pos,
                    direction: (light.pos - intersection.pos).normalize(),
                };
                let t = self._get_intersection(&ray, 0.1);
                if t.is_some() {
                    return None;
                }
                Some((light, ray))
            })
            .map(|(light, ray)| {
                let diffuse_light = material.k_diffuse
                    * self._get_diffuse_lighting(intersection, material, light, &ray);
                let specular_reflectance = material.k_specular
                    * self._get_specular_lighting(intersection, material, light, &ray);
                diffuse_light + specular_reflectance
            })
            .sum();
        ambient + light_dependent_colouring
    }

    fn _get_ray_colour(&self, ray: &Ray, min_distance: Float, num_bounces: u8) -> FVec {
        self._get_intersection(&ray, min_distance)
            .map(|(i, m)| {
                let object_colour = self._get_surface_point_colour(&i, &m);
                let reflection = self._get_reflection(&i, &m, &ray, num_bounces);
                object_colour + reflection
            })
            .unwrap_or(self.default_colour)
    }

    fn render_to_file(&self, path: &str) -> Result<(), ImageError> {
        let image: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::from_par_fn(
            self.camera.screen_columns,
            self.camera.screen_rows,
            |x, y| {
                let ray = self.camera.get_ray(x, y);
                let rgb = self
                    ._get_ray_colour(&ray, 0.0, 0)
                    .map(channel_float_to_int)
                    .into();
                Rgb(rgb)
            },
        );
        image.save(path)
    }
}

fn main() {
    let scene = Scene::from_file("scene.json").unwrap();
    println!("{:?}", scene);
    scene
        .render_to_file("output.png")
        .unwrap();
}
