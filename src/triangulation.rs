    // module for triangulating a simple polygon using ear clipping

use std::collections::HashSet;
use std::fmt;
use std::ops::{Mul, Sub};

pub trait Zeroable {
    fn zero() -> Self;
}

impl Zeroable for f32 {
    fn zero() -> f32 { 0.0f32 }
}

impl Zeroable for f64 {
    fn zero() -> f64 { 0.0f64 }
}

impl Zeroable for i32 {
    fn zero() -> i32 { 0i32 }
}

impl Zeroable for i16 {
    fn zero() -> i16 { 0i16 }
}

impl Zeroable for isize {
    fn zero() -> isize { 0isize }
}

pub trait Point {
    type T: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display;
    fn get_x(&self) -> Self::T;
    fn get_y(&self) -> Self::T;
}

impl<U> fmt::Display for Point<T=U> 
        where U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.get_x(), self.get_y())
    }
}

impl<U> Point for (U, U) 
        where U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display + Copy {
    type T = U;
    fn get_x(&self) -> U { self.0 }
    fn get_y(&self) -> U { self.1 }
}


#[derive(Debug, PartialEq)]
struct Vertex {
    index: usize,
    prev_index: usize,
    next_index: usize,
    is_convex: bool,
    is_ear: bool
}
impl Vertex {
    fn new(index: usize, prev_index: usize, next_index: usize) -> Vertex {
        Vertex { index: index, prev_index: prev_index, next_index: next_index, is_convex: false, is_ear: false }
    }
}

#[derive(Debug, PartialEq)]
enum LineCompare {
    Left, Right, On
}

fn compare_to_line<P, U>(v_test: &P, v_prev: &P, v_next: &P) -> LineCompare 
    where P :Point<T=U>, U: Mul<Output=U> + Sub<Output=U> + Zeroable + PartialOrd + PartialEq + fmt::Display {
    let val = (v_test.get_x() - v_prev.get_x())*(v_next.get_y() - v_prev.get_y()) - 
              (v_test.get_y() - v_prev.get_y())*(v_next.get_x() - v_prev.get_x());
    let zero = U::zero();
    if val < zero { LineCompare::Left } else if val > zero { LineCompare::Right } else { LineCompare::On }
}

fn is_convex<P, U>(v_test: &P, v_prev: &P, v_next: &P) -> bool 
        where P: Point<T=U>, U: Mul<Output=U>, U: Sub<Output=U>, U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    // point is convex if right of line made by prev->next
    compare_to_line(v_test, v_prev, v_next) == LineCompare::Right
}

fn is_in_triangle<P, U>(v_test: &P, v0: &P, v1: &P, v2: &P) -> bool 
        where P: Point<T=U>, U: Mul<Output=U>, U: Sub<Output=U>, U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    if compare_to_line(v_test, v0, v1) != LineCompare::Left { return false; }
    if compare_to_line(v_test, v1, v2) != LineCompare::Left { return false; }
    if compare_to_line(v_test, v2, v0) != LineCompare::Left { return false; }
    true
}

// note: this function assumes v_test is convex!
fn is_ear<P, U>(points: &Vec<P>, reflex_set: &HashSet<usize>, v_test: &Vertex) -> bool 
        where P: Point<T=U>, U: Mul<Output=U>, U: Sub<Output=U>, U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    for r in reflex_set {
        if *r == v_test.prev_index || *r == v_test.next_index {
            continue;
        }
        if is_in_triangle(&points[*r], 
                          &points[v_test.prev_index], 
                          &points[v_test.index], 
                          &points[v_test.next_index]) {
            return false;
        }
    }
    true
}

fn make_vertex_vec(n: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(n);
    vertices.push(Vertex::new(0, n-1, 1));
    for i in 1..(n-1) {
        vertices.push(Vertex::new(i, i-1, i+1));
    }
    vertices.push(Vertex::new(n-1, n-2, 0));
    vertices
}

#[derive(Debug, Eq, PartialEq)]
enum VertexType {
    Reflex,
    Convex,
    Ear
}

fn classify_vertex<P, U>(points: &Vec<P>, v_test: &mut Vertex, reflex_set: &HashSet<usize>) -> VertexType
        where P: Point<T=U>, U: Mul<Output=U>, U: Sub<Output=U>, U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    if is_convex(&points[v_test.index], &points[v_test.prev_index], &points[v_test.next_index]) {
        if is_ear(&points, reflex_set, &v_test) {
             return VertexType::Ear;
        } else {
            return VertexType::Convex;
        }
    } else {
        return VertexType::Reflex;
    }
}

fn fill_sets<P, U>(points: &Vec<P>, vertices: &mut Vec<Vertex>) -> (HashSet<usize>, HashSet<usize>)
        where P: Point<T=U>, U: Mul<Output=U>, U: Sub<Output=U>, U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    let mut ear_set = HashSet::new();
    let mut reflex_set = HashSet::new();

    for v in vertices.iter_mut() {
        if classify_vertex(points, v, &reflex_set) == VertexType::Reflex {
            reflex_set.insert(v.index);
        }
    }

    for mut v in vertices.iter_mut() {
        match classify_vertex(points, v, &reflex_set) {
            VertexType::Reflex => (),
            VertexType::Convex => {
                v.is_convex = true;
            }
            VertexType::Ear => {
                println!("adding {} to ear list", v.index);
                v.is_convex = true;
                v.is_ear = true;
                ear_set.insert(v.index);
            }
        }
    }
    
    (ear_set, reflex_set)
}

fn remove_vertex(vertices: &mut Vec<Vertex>, i_test: usize) {

    let prev_index = vertices[i_test].prev_index;
    let next_index = vertices[i_test].next_index;
    vertices[prev_index].next_index = next_index;
    vertices[next_index].prev_index = prev_index;
}

fn push_triangle(triangles: &mut Vec<usize>, i_test: usize, i_prev: usize, i_next: usize) {
    triangles.push(i_prev);
    triangles.push(i_test);
    triangles.push(i_next);
}

pub fn triangulate<P, U>(points: &Vec<P>) -> Result<Vec<usize>, &'static str>
         where P: Point<T=U>, U: Mul<Output=U>, U: Sub<Output=U>, U: PartialOrd + PartialEq + Sub + Mul + Zeroable + fmt::Display {
    let mut n = points.len();
    if n < 4 {
        if n == 3 {
            return Ok(vec![0, 1, 2]);
        }  else {
            return Err("Not enough vertices to triangulate");
        }
    }

    let mut vertices = make_vertex_vec(n);
    let (mut ear_set, mut reflex_set) = fill_sets(points, &mut vertices);

    let mut triangles = Vec::with_capacity(3 * (n - 2));
    
    loop {
        let ear_index = match ear_set.iter().next() {
            Some(i) => *i,
            None => return Err("Expected an ear in the ear set but found None")
        };

        ear_set.remove(&ear_index);
        println!("removing {} from ear list", ear_index);

        let prev_index;
        let next_index;
        {
            let vertex = &vertices[ear_index];
            prev_index = vertex.prev_index;
            next_index = vertex.next_index;
        }
        push_triangle(&mut triangles, ear_index, prev_index, next_index);
        remove_vertex(&mut vertices, ear_index);
        n -= 1;

        if n == 3 {
            let ear_index = match ear_set.iter().next() {
                Some(i) => *i,
                None => return Err("Expected an ear in the ear set but found None")
            };
            let prev_index;
            let next_index;
            {
                println!("removing last ear {}", ear_index);
                let vertex = &vertices[ear_index];
                prev_index = vertex.prev_index;
                next_index = vertex.next_index;
            }
            push_triangle(&mut triangles, ear_index, prev_index, next_index);
            return Ok(triangles);
        }

        {
            let ref mut v_prev = vertices[prev_index];
            if v_prev.is_ear {
                if !is_ear(&points, &reflex_set, v_prev) {
                    v_prev.is_ear = false;
                    ear_set.remove(&prev_index);
                    println!("{} is no longer an ear", prev_index);
                }
            } else {
                if is_convex(&points[prev_index], &points[v_prev.prev_index], &points[v_prev.next_index]) {
                    if !v_prev.is_convex {
                        v_prev.is_convex = true;
                        reflex_set.remove(&prev_index);
                    }
                    
                    if is_ear(&points, &reflex_set, v_prev) {
                        ear_set.insert(prev_index);
                        println!("{} is now an ear", prev_index);
                    }
                }
            }
        }

        {
            let ref mut v_next = vertices[next_index];
            if v_next.is_ear {
                if !is_ear(&points, &reflex_set, v_next) {
                    v_next.is_ear = false;
                    ear_set.remove(&next_index);
                    println!("{} is no longer an ear", next_index);
                }
            } else {
                if is_convex(&points[next_index], &points[v_next.prev_index], &points[v_next.next_index]) {
                    if !v_next.is_convex {
                        v_next.is_convex = true;
                        reflex_set.remove(&next_index);
                    }
                    
                    if is_ear(&points, &reflex_set, v_next) {
                        ear_set.insert(next_index);
                        println!("{} is now an ear", next_index);
                    }
                }
            }
        }
    }
}

fn triangle_edges(v0: usize, v1: usize, v2: usize, max: usize) -> (bool, bool, bool) { 
    let e0 = v1 == 0 && v0 == max || (v1 > v0 && v1 - v0 == 1);
    println!("{} -> {}: {}", v0, v1, e0);
    let e1 = v2 == 0 && v1 == max || (v2 > v1 && v2 - v1 == 1);
    println!("{} -> {}: {}", v1, v2, e1);
    let e2 = v0 == 0 && v2 == max || (v0 > v2 && v0 - v2 == 1);
    println!("{} -> {}: {}", v2, v0, e2);
    (e0, e1, e2)
}
 
pub fn find_edges(triangles: &Vec<usize>, max: usize) -> Vec<bool> {
    let n = triangles.len();
    let mut edges = Vec::with_capacity(n);

    for i in 0..(n/3) {
        let e = triangle_edges(triangles[i*3], triangles[i*3+1], triangles[i*3+2], max);
        edges.push(e.0);
        edges.push(e.1);
        edges.push(e.2);
    }
    edges
}

// extern crate libc;
// 
// use libc::size_t;
// use libc::c_float;
// use libc::c_uchar;
// use std::ptr;
// 
// impl Zeroable for f32 {
//     fn zero() -> f32 { 0.0f32 }
// }
// 
// #[no_mangle]
// pub extern "C" fn c_triangulate(num_points: size_t, c_points: *const c_float, 
//                                 num_triangle_indices: size_t, c_triangle_indices: *mut size_t,
//                                 num_edges: size_t, c_edges: *mut c_uchar) -> i32 {
//     if num_points % 3usize != 0usize {
//         return -1i32;
//     }
//     if num_triangle_indices < 3usize * (num_points - 2usize) {
//         return -1i32;
//     }
//     if num_edges != num_triangle_indices {
//         return -1i32;
//     }
//     if c_points == ptr::null() || c_triangle_indices == ptr::null_mut() || c_edges == ptr::null_mut() {
//         return -2i32;
//     }
// 
//     unsafe {
//         let points_slice = std::slice::from_raw_parts(c_points, num_points*2);
//         let mut points = Vec::with_capacity(num_points);
//         for i in 0..num_points {
//             points.push((points_slice[2*i], points_slice[2*i+1]));
//         }
// 
//         match triangulate(&points) {
//             Ok(triangle_indices) => {
//                 let edges = find_edges(&triangle_indices, num_points);
//                 let triangle_slice = std::slice::from_raw_parts_mut(c_triangle_indices, num_triangle_indices);
//                 let edge_slice = std::slice::from_raw_parts_mut(c_edges, num_edges);
//                 for i in 0..num_triangle_indices {
//                     triangle_slice[i] = triangle_indices[i];
//                     edge_slice[i] = if edges[i] { 1u8 } else { 08 }
//                 }
//                 0i32
//             },
//             Err(_) => -3i32
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::Zeroable;
    use super::compare_to_line;
    use super::is_convex;
    use super::is_in_triangle;
    use super::triangulate;
    use super::LineCompare;
    use super::find_edges;

    //#[test]
    fn test_compare_to_line() {
        let v0 = (0.1f32,  0.1f32);
        let v1 = (0.5f32, 0.5f32);

        let test = (0.2f32, 1.2f32);
        assert_eq!(compare_to_line(&test, &v0, &v1), LineCompare::Left);
        
        let test = (0.2f32, -1.0f32);
        assert_eq!(compare_to_line(&test, &v0, &v1), LineCompare::Right);
        
        let test = (v0.0 + (v1.0 - v0.0) * 0.2f32, v0.1 + (v1.1 - v0.1) * 0.2f32);      
        assert_eq!(compare_to_line(&test, &v0, &v1), LineCompare::On);
    }
    
    #[test]
    fn test_is_convex() {
        let v0 = (0.1f32, 0.1f32);
        let v1 = (0.5f32, 0.5f32);

        let test = (0.2f32, 1.2f32);
        assert!(!is_convex(&test, &v0, &v1));
        
        let test = (0.2f32, -1.0f32);
        assert!(is_convex(&test, &v0, &v1));
        
        let test = (v0.0 + (v1.0 - v0.0) * 0.2f32, v0.1 + (v1.1 - v0.1) * 0.2f32);
        assert!(!is_convex(&test, &v0, &v1));
    }
    
    #[test]
    fn test_is_in_triangle() {
        let v0 = (0.0f32, 0.0f32);
        let v1 = (1.0f32, 0.0f32);
        let v2 = (0.0f32, 1.0f32);

        let test = (0.75f32, 0.75f32);
        assert!(!is_in_triangle(&test, &v0, &v1, &v2));
        
        let test = (0.25f32, 0.25f32);
        assert!(is_in_triangle(&test, &v0, &v1, &v2));
        
        let test = (0.5f32, 0.5f32);
        assert!(!is_in_triangle(&test, &v0, &v1, &v2));
    }

    fn is_expected_triangle(v0: &usize, v1: &usize, v2: &usize, expected: &(usize, usize, usize)) -> bool {
        if *v0 == expected.0 && *v1 == expected.1 && *v2 == expected.2 {
            println!("{}, {}, {} is expected triangle", *v0, *v1, *v2);
            return true;
        }
        if *v1 == expected.0 && *v2 == expected.1 && *v0 == expected.2 {
            println!("{}, {}, {} is expected triangle", *v0, *v1, *v2);
            return true;
        }
        if *v2 == expected.0 && *v0 == expected.1 && *v1 == expected.2 {
            println!("{}, {}, {} is expected triangle", *v0, *v1, *v2);
            return true;
        }
        println!("{}, {}, {}, did not match {}, {}, {}", *v0, *v1, *v2, expected.0, expected.1, expected.2);
        false
    }

    fn is_same_triangulation(triangles: &Vec<usize>, mut expected: Vec<(usize, usize, usize)>) -> bool {
        let n = triangles.len() / 3;
        for i in 0..n {
            let mut matched = false;
            let mut match_index = 0;
            for (index, ex) in expected.iter().enumerate() {
                if is_expected_triangle(&triangles[i*3], &triangles[i*3+1], &triangles[i*3+2], ex) {
                    matched = true;
                    match_index = index;
                    break;
                }
            }
            if matched {
                expected.remove(match_index);
            } else {
                return false;
            }
        }
        true
    }

    #[test]
    fn test_triangulate_square() {
        let points = vec![ (0.0f32, 0.0f32),
                           (1.0f32, 0.0f32), 
                           (1.0f32, 1.0f32), 
                           (0.0f32, 1.0f32) ];

        let triangles = triangulate(&points).unwrap();

        for i in 0..(triangles.len() / 3) {
            println!("{}, {}, {}", triangles[i*3], triangles[i*3+1], triangles[i*3+2]);
        }

        // there are two valid triangulations of a square, up-right diagonal
        // and down-right diagonal, so we try both
        assert!(is_same_triangulation(&triangles, vec![(0, 1, 2), (0, 2, 3)]) ||
                is_same_triangulation(&triangles, vec![(0, 1, 3), (3, 1, 2)]));
    }

    #[test]
    fn test_triangulate_reflex() {
        let points = vec![ (0.0f32, 0.0f32),
                           (5.0f32, 0.0f32), 
                           (2.0f32, 2.0f32), 
                           (5.0f32, 4.0f32),
                           (0.0f32, 4.0f32) ];

        let triangles = triangulate(&points).unwrap();

        for i in 0..(triangles.len() / 3) {
            println!("{}, {}, {}", triangles[i*3], triangles[i*3+1], triangles[i*3+2]);
        }

        assert!(is_same_triangulation(&triangles, vec![(0, 1, 2), (0, 2, 4), (4, 2, 3)]));
    }

    #[test]
    fn test_find_edges() {
        // for now I will just inspect the output
        let points = vec![ (0.0f32, 0.0f32),
                           (5.0f32, 0.0f32), 
                           (2.0f32, 2.0f32), 
                           (5.0f32, 4.0f32),
                           (0.0f32, 4.0f32) ];

        let triangles = triangulate(&points).unwrap();
        let edges = find_edges(&triangles, 4);

        for i in 0..(triangles.len() / 3) {
            println!("T {}, {}, {}", triangles[i*3], triangles[i*3+1], triangles[i*3+2]);
            println!("E {}, {}, {}", edges[i*3], edges[i*3+1], edges[i*3+2]);
        }


    }
}