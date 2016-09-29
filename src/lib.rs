// module for triangulating a simple polygon using ear clipping

use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    x: f32,
    y: f32
}
impl Point {
    pub fn new(x: f32, y: f32) -> Point {
        Point { x: x, y: y }
    }
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

fn compare_to_line(v_test: &Point, v_prev: &Point, v_next: &Point) -> f32 {
    // returns negative if test is left of line, positive if right, 0 if colinear
    (v_test.x - v_prev.x)*(v_next.y - v_prev.y) - (v_test.y - v_prev.y)*(v_next.x - v_prev.x)
}

fn is_convex(v_test: &Point, v_prev: &Point, v_next: &Point) -> bool {
    // point is convex if right of line made by prev->next
    compare_to_line(v_test, v_prev, v_next) > 0.0f32
}

fn is_in_triangle(v_test: &Point, v0: &Point, v1: &Point, v2: &Point) -> bool {
    if compare_to_line(v_test, v0, v1) >= 0.0f32 { return false; }
    if compare_to_line(v_test, v1, v2) >= 0.0f32 { return false; }
    if compare_to_line(v_test, v2, v0) >= 0.0f32 { return false; }
    true
}

// note: this function assumes v_test is convex!
fn is_ear(points: &Vec<Point>, reflex_set: &HashSet<usize>, v_test: &Vertex) -> bool {
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

fn classify_vertex(points: &Vec<Point>, v_test: &mut Vertex,
        ear_vec: &mut HashSet<usize>, reflex_set: &mut HashSet<usize>) {
    if is_convex(&points[v_test.index], &points[v_test.prev_index], &points[v_test.next_index]) {
        v_test.is_convex = true;
        if is_ear(&points, reflex_set, &v_test) {
            v_test.is_ear = true;
            ear_vec.insert(v_test.index);
        }
    } else {
        reflex_set.insert(v_test.index);
    }
}

fn fill_sets(points: &Vec<Point>, vertices: &mut Vec<Vertex>) -> (HashSet<usize>, HashSet<usize>) {
    let mut ear_set = HashSet::new();
    let mut reflex_set = HashSet::new();
    
    for v in vertices {
        classify_vertex(points, v, &mut ear_set, &mut reflex_set);
    }

    (ear_set, reflex_set)
}

fn remove_vertex(vertices: &mut Vec<Vertex>, i_test: usize) {
    let prev_index = vertices[i_test].prev_index;
    let next_index = vertices[i_test].next_index;
    vertices[prev_index].next_index = next_index;
    vertices[next_index].prev_index = prev_index;
}

fn push_triangle(triangles: &mut Vec<Point>, points: &Vec<Point>, i_test: usize, i_prev: usize, i_next: usize) {
    triangles.push(points[i_prev]);
    triangles.push(points[i_test]);
    triangles.push(points[i_next]);
}

pub fn triangulate(points: Vec<Point>) -> Result<Vec<Point>, &'static str> {
    let mut n = points.len();
    if n < 4 {
        if n == 3 {
            return Ok(points);
        }  else {
            return Err("Not enough vertices to triangulate")
        }
    }

    let mut vertices = make_vertex_vec(n);
    let (mut ear_set, mut reflex_set) = fill_sets(&points, &mut vertices);

    let mut triangles = Vec::with_capacity(3 * (n - 1));
    
    loop {
        let ear_index = match ear_set.iter().next() {
            Some(i) => *i,
            None => return Err("Expected an ear in the ear set but found None")
        };
        ear_set.remove(&ear_index);
        let index;
        let prev_index;
        let next_index;
         {
            let vertex = &vertices[ear_index];
            index = vertex.index;
            prev_index = vertex.prev_index;
            next_index = vertex.next_index;
        }
        push_triangle(&mut triangles, &points, ear_index, prev_index, next_index);
        remove_vertex(&mut vertices, index);
        n -= 1;

        if n == 3 {
            let ear_index = match ear_set.iter().next() {
                Some(i) => *i,
                None => return Err("Expected an ear in the ear set but found None")
            };
            let vertex = &vertices[ear_index];
            push_triangle(&mut triangles, &points, ear_index, vertex.index, vertex.next_index);
            return Ok(triangles);
        }

        {
            let ref mut v_prev = vertices[prev_index];
            if v_prev.is_ear {
                if !is_ear(&points, &reflex_set, v_prev) {
                    v_prev.is_ear = false;
                    ear_set.remove(&prev_index);
                }
            } else {
                if is_convex(&points[prev_index], &points[v_prev.prev_index], &points[v_prev.next_index]) {
                    if !v_prev.is_convex {
                        v_prev.is_convex = true;
                        reflex_set.remove(&prev_index);
                    }
                    if is_ear(&points, &reflex_set, v_prev) {
                        ear_set.insert(prev_index);
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
                }
            } else {
                if is_convex(&points[next_index], &points[v_next.prev_index],& points[v_next.next_index]) {
                    if !v_next.is_convex {
                        v_next.is_convex = true;
                        reflex_set.remove(&next_index);
                    }
                    if is_ear(&points, &reflex_set, v_next) {
                        ear_set.insert(next_index);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Point;
    use super::compare_to_line;
    use super::is_convex;
    use super::is_in_triangle;

    #[test]
    fn test_compare_to_line() {
        let test = Point::new(0.2f32, 1.2f32);
        let v0 = Point::new(0.1f32,  0.1f32);
        let v1 = Point::new(0.5f32, 0.5f32);

        assert!(compare_to_line(&test, &v0, &v1) < 0.0f32);
        
        let test = Point::new(0.2f32, -1.0f32);

        assert!(compare_to_line(&test, &v0, &v1) > 0.0f32);
        
        let test = Point::new(v0.x + (v1.x - v0.x) * 0.2f32, v0.y + (v1.y - v0.y) * 0.2f32);
        
        assert_eq!(compare_to_line(&test, &v0, &v1), 0.0f32);
    }
    
    #[test]
    fn test_is_convex() {
        let test = Point::new(0.2f32, 1.2f32);
        let v0 = Point::new(0.1f32, 0.1f32);
        let v1 = Point::new(0.5f32, 0.5f32);

        assert_eq!(is_convex(&test, &v0, &v1), false);
        
        let test = Point::new(0.2f32, -1.0f32);

        assert!(is_convex(&test, &v0, &v1));
        
        let test = Point::new(v0.x + (v1.x - v0.x) * 0.2f32, v0.y + (v1.y - v0.y) * 0.2f32);
        
        assert_eq!(is_convex(&test, &v0, &v1), false);
    }
    
    #[test]
    fn test_is_in_triangle() {
        let v0 = Point::new(0.1f32, 0.1f32);
        let v1 = Point::new(0.7f32, -0.2f32);
        let v2 = Point::new(0.5f32, 0.5f32);

        let test = Point::new(0.2f32, 1.2f32);
        assert_eq!(is_in_triangle(&test, &v0, &v1, &v2), false);
        
        let test = Point::new(0.4f32, 0.2f32);
        assert!(is_in_triangle(&test, &v0, &v1, &v2));
        
        let test = Point::new(v2.x + (v0.x - v2.x) * 0.25f32, v2.y + (v0.y - v2.y) * 0.25f32);
        assert_eq!(is_in_triangle(&test, &v0, &v1, &v2), false);
    }
}