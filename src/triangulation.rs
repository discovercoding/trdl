//! Module for triangulating a simple polygon using ear clipping.

use std::collections::HashSet;
use super::TrdlError;

// The vertex class holds the index of a vertex in the list of points. It also has the index of the
// previousand next vertex as well as a flag indicating if it is convex and if it is an ear.
// All of these attributes only makes sense as relationships between it and other vertices and are
// determined by its location in the polygon.
#[derive(Debug, PartialEq)]
struct Vertex {
    index: usize,
    prev_index: usize,
    next_index: usize,
    is_convex: bool,
    is_ear: bool
}
impl Vertex {
    // Contructor
    fn new(index: usize, prev_index: usize, next_index: usize) -> Vertex {
        Vertex { index: index, prev_index: prev_index, next_index: next_index,
                 is_convex: false, is_ear: false }
    }
}

// Enum indicating whether a point is left of, right of or on a line segment.
#[derive(Debug, PartialEq)]
enum LineCompare {
    Left, Right, On
}

// Determine if a point is left of, right of, or on a line segment determined by two points.
fn compare_to_line(v_test: &(f32, f32),
                   v_prev: &(f32, f32), v_next: &(f32, f32)) -> LineCompare {
    let val = (v_test.0 - v_prev.0)*(v_next.1 - v_prev.1) -
              (v_test.1 - v_prev.1)*(v_next.0 - v_prev.0);
    if val < 0.0f32 {
        LineCompare::Left
    } else if val > 0.0f32 {
        LineCompare::Right
    } else {
        LineCompare::On
    }
}

// Determine if a angle created by 3 points is convex or reflex.
fn is_convex(v_test: &(f32, f32),
             v_prev: &(f32, f32), v_next: &(f32, f32)) -> bool {
    // point is convex if right of line made by prev->next
    compare_to_line(v_test, v_prev, v_next) == LineCompare::Right
}

// Determine if a point is inside a triangle created by 3 other points.
fn is_in_triangle(v_test: &(f32, f32), v0: &(f32, f32),
                  v1: &(f32, f32), v2: &(f32, f32)) -> bool {
if compare_to_line(v_test, v0, v1) != LineCompare::Left { return false; }
    if compare_to_line(v_test, v1, v2) != LineCompare::Left { return false; }
    if compare_to_line(v_test, v2, v0) != LineCompare::Left { return false; }
    true
}

// Determine if a point is an ear tip.
// note: this function assumes v_test is convex!
fn is_ear(points: &Vec<(f32, f32)>, reflex_set: &HashSet<usize>, v_test: &Vertex) -> bool {
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

// Make a list of vectors from a vector of ordered points representing a polygon.
fn make_vertex_vec(n: usize) -> Vec<Vertex> {
    let mut vertices = Vec::with_capacity(n);
    vertices.push(Vertex::new(0, n-1, 1));
    for i in 1..(n-1) {
        vertices.push(Vertex::new(i, i-1, i+1));
    }
    vertices.push(Vertex::new(n-1, n-2, 0));
    vertices
}

// Enum representing the type of a vertex (reflex, convex or ear, ear implies convex)
#[derive(Debug, Eq, PartialEq)]
enum VertexType {
    Reflex,
    Convex,
    Ear
}

// Classify a vertex as reflex, convex or ear.
fn classify_vertex(points: &Vec<(f32, f32)>, v_test: &mut Vertex,
                   reflex_set: &HashSet<usize>) -> VertexType {
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

// Fill the ear set and the reflex set with the indices of the corresponding vertices.
fn fill_sets(points: &Vec<(f32, f32)>,
             vertices: &mut Vec<Vertex>) -> (HashSet<usize>, HashSet<usize>) {
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
                v.is_convex = true;
                v.is_ear = true;
                ear_set.insert(v.index);
            }
        }
    }
    (ear_set, reflex_set)
}

// Connect the 2 points on either side of a point, effectivly removing that point from the linked
// list.
fn remove_vertex(vertices: &mut Vec<Vertex>, i_test: usize) {
    let prev_index = vertices[i_test].prev_index;
    let next_index = vertices[i_test].next_index;
    vertices[prev_index].next_index = next_index;
    vertices[next_index].prev_index = prev_index;
}

// Add 3 points representing a triangle to the triangle list.
fn push_triangle(triangles: &mut Vec<usize>, i_test: usize, i_prev: usize, i_next: usize) {
    triangles.push(i_prev);
    triangles.push(i_test);
    triangles.push(i_next);
}

/// Accept a vector of points representing vertices of a polygon with counter-clockwise ordering.
/// Remove ear tips one at a time adding triangles to the triangle list until the last triangle
/// which is added to the triangle list, creating a triangulation of the polygon.
/// Return a list of indices into the original passed in list of vertices, every three indices is a
/// triangle. Or return an error if a problem occurred.
pub fn triangulate(points: &Vec<(f32, f32)>) -> Result<Vec<usize>, TrdlError> {
    let mut n = points.len();
    if n < 4 {
        if n == 3 {
            return Ok(vec![0, 1, 2]);
        }  else {
            return Err(TrdlError::NotEnoughVertices);
        }
    }

    let mut vertices = make_vertex_vec(n);
    let (mut ear_set, mut reflex_set) = fill_sets(points, &mut vertices);

    let mut triangles = Vec::with_capacity(3 * (n - 2));
    
    loop {
        let ear_index = match ear_set.iter().next() {
            Some(i) => *i,
            None => return Err(TrdlError::NonSimplePolygon)
        };

        ear_set.remove(&ear_index);

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
                None => return Err(TrdlError::NonSimplePolygon)
            };
            let prev_index;
            let next_index;
            {
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
                }
            } else {
                if is_convex(&points[prev_index], &points[v_prev.prev_index],
                             &points[v_prev.next_index]) {
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
                if !is_ear(points, &reflex_set, v_next) {
                    v_next.is_ear = false;
                    ear_set.remove(&next_index);
                }
            } else {
                if is_convex(&points[next_index], &points[v_next.prev_index],
                             &points[v_next.next_index]) {
                    if !v_next.is_convex {
                        v_next.is_convex = true;
                        reflex_set.remove(&next_index);
                    }
                    
                    if is_ear(points, &reflex_set, v_next) {
                        ear_set.insert(next_index);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compare_to_line;
    use super::is_convex;
    use super::is_in_triangle;
    use super::triangulate;
    use super::LineCompare;

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

    fn is_expected_triangle(v0: &usize, v1: &usize, v2: &usize,
                            expected: &(usize, usize, usize)) -> bool {
        if *v0 == expected.0 && *v1 == expected.1 && *v2 == expected.2 {
            return true;
        }
        if *v1 == expected.0 && *v2 == expected.1 && *v0 == expected.2 {
            return true;
        }
        if *v2 == expected.0 && *v0 == expected.1 && *v1 == expected.2 {
            return true;
        }
        false
    }

    fn is_same_triangulation(triangles: &Vec<usize>,
                             mut expected: Vec<(usize, usize, usize)>) -> bool {
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
}