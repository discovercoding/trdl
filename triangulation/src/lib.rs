// module for triangulating a simple polygon
#![feature(btree_range, collections_bound)]

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::BTreeSet;
use std::collections::Bound::{Included, Unbounded};

#[derive(Debug)]
struct Point {
    x: f32,
    y: f32
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Point) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Point) -> Ordering {
        match self.y.partial_cmp(&other.y) {
            // flipped x and y because a point is considered lower 
            // if the y's are the same and the x is HIGHER
            Some(Ordering::Equal) => match other.x.partial_cmp(&self.x) {
                Some(ordering) => ordering,
                None => Ordering::Equal
            },
            // if y's not equal, lower y is lower point
            Some(ordering) => ordering,
            // if nan or something just say they are equal
            None => Ordering::Equal
        }
    }
}
impl PartialEq for Point {
    fn eq(&self, other: &Point) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Point { }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct HalfEdge<'a> {
    origin: &'a Point,
    next_index: usize,
    prev_index: usize
}

fn make_edge_list(points: &Vec<Point>) -> Vec<HalfEdge> {
    let n = points.len();
    let mut edge_list = Vec::with_capacity(n);
    edge_list.push(HalfEdge { origin: &points[0], prev_index: n-1, next_index: 1});
    for (i, p) in points.iter().enumerate().skip(1) {
        edge_list.push(HalfEdge { origin: p, prev_index: i-1, next_index: i+1 })
    }
    edge_list[n-1].next_index = 0;

    edge_list
}

fn insert_edge<'a, 'b>(edge_list: &'b mut Vec<HalfEdge<'a>>, start_index: usize, end_index: usize) {
    let new_index = edge_list.len();
    let twin_index = new_index + 1;
    
    let new_edge = HalfEdge { origin: edge_list[start_index].origin, 
                              prev_index: edge_list[start_index].prev_index, 
                              next_index: end_index };

    let twin_edge = HalfEdge { origin: edge_list[end_index].origin,
                               prev_index: edge_list[end_index].prev_index,
                               next_index: start_index };
      
    edge_list[new_edge.prev_index].next_index = new_index;
    edge_list[start_index].prev_index = twin_index;

    edge_list[twin_edge.prev_index].next_index = twin_index;
    edge_list[end_index].prev_index = new_index;

    edge_list.push(new_edge);
    edge_list.push(twin_edge);
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum VertexType {
    Start,
    End,
    Split,
    Merge,
    Regular,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Vertex<'a> {
    point: &'a Point,
    vertex_type: VertexType,
    edge_index: usize
}

impl<'a> Vertex<'a> {
    fn new_regular(point: &'a Point, edge_index: usize) -> Vertex<'a> {
        Vertex { point: point, vertex_type: VertexType::Regular, edge_index: edge_index }
    }

    fn new(point: &'a Point, vertex_type: VertexType, edge_index: usize) -> Vertex<'a> {
        Vertex { point: point, vertex_type: vertex_type, edge_index: edge_index }
    }
 }

fn classify_vertex<'a>(edge_list: &'a Vec<HalfEdge<'a>>, index: usize) -> Vertex<'a> {
    let ref edge = edge_list[index];
    let current = edge.origin;
    let prev = edge_list[edge.prev_index].origin;
    let next = edge_list[edge.next_index].origin;

    let mut vertex = Vertex::new_regular(current, index);

    let prev_below = prev < current;
    let next_below = next < current;

    if prev_below && next_below {
        if next.x > current.x {
            // both below, interior below
            vertex.vertex_type = VertexType::Start;
        } else {
            // both below, interior above
            vertex.vertex_type = VertexType::Split;
        }
    } else if !prev_below && !next_below { 
        if next.x > current.x {
            // both above, interior below
            vertex.vertex_type = VertexType::Merge;
        } else {
            // both above, interior above
            vertex.vertex_type =  VertexType::End;
        }
    }
    vertex
}

fn build_vertex_queue<'a>(edge_list: &'a Vec<HalfEdge<'a>>) -> BinaryHeap<Vertex<'a>> {
    let mut vertex_queue = BinaryHeap::new();
    for index  in 0..edge_list.len() {
        vertex_queue.push(classify_vertex(edge_list, index));
    }
    vertex_queue
}

struct TEdge<'a> {
    edge: &'a HalfEdge<'a>,
    helper: &'a Vertex<'a>
}

impl<'a> PartialOrd for TEdge<'a> {
    fn partial_cmp(&self, other: &TEdge<'a>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for TEdge<'a> {
    fn cmp(&self, other: &TEdge<'a>) -> Ordering {
        // left to right ordering based on x coord of origin of edge
        match self.edge.origin.x.partial_cmp(&other.edge.origin.x) {
            Some(ordering) => ordering,
            // if nan or something just say they are equal
            None => Ordering::Equal
        }
    }
}

impl<'a> PartialEq for TEdge<'a> {
    fn eq(&self, other: &TEdge<'a>) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl<'a> Eq for TEdge<'a> { }

fn get_left<'a>(btree: &'a BTreeSet<TEdge>, val: &'a TEdge<'a>) -> &'a TEdge<'a> {
    let mut subset = btree.range(Unbounded, Included(val));
    subset.next_back(); // why do we have to walk back 1?
    subset.next_back().unwrap() // TODO: Robust error handling
}

fn  handle_start_vertex<'a>(btree: &'a mut BTreeSet<TEdge<'a>>,
                                vertex: &'a Vertex<'a>,
                                edge_list: &'a Vec<HalfEdge<'a>>) {
    btree.insert(TEdge { edge: &edge_list[vertex.edge_index], helper: vertex });
}

fn handle_end_vertex<'a>(btree: &'a mut BTreeSet<TEdge<'a>>,
                                vertex: &'a Vertex<'a>,
                                edge_list: &'a Vec<HalfEdge<'a>>) {

                                }

/*

fn make_monotone(half_edge_list: &mut Vec<HalfEdge>, vertex_queue: &mut BinaryHeap<Vertex>) {
    let mut btree = BTreeSet::new();
    while !vertex_queue.is_empty() {
        let vertex = vertex_queue.pop().unwrap(); // TODO: Robuts error handling
        btree.
    }

}
*/
#[cfg(test)]
mod tests {
    use super::Point;
    use super::HalfEdge;
    use super::make_edge_list;
    use super::VertexType;
    use super::Vertex;
    use super::classify_vertex;
    use super::build_vertex_queue;
    use super::insert_edge;

    #[test]
    fn different_y() {
        let p1 = Point { x: 0.1, y: 0.2 };
        let p2 = Point { x: 0.0, y: 0.3 };
        assert!(p1 < p2);
    }
    #[test]
    fn bigger_x_and_y() {
        let p1 = Point { x: 0.1, y: 0.2 };
        let p2 = Point { x: 0.5, y: 0.3 };
        assert!(p1 < p2);
    }
    #[test]
    fn same_y() {
        let p1 = Point { x: 0.5, y: 0.2 };
        let p2 = Point { x: 0.4, y: 0.2 };
        assert!(p1 < p2);
    }

    #[test]
    fn test_make_edge_list() {
        let n = 9;
        let mut vec = Vec::with_capacity(n);
        vec.push(Point { x: 0.5f32, y: 2.5f32 });
        vec.push(Point { x: 1.0f32, y: 3.0f32 });
        vec.push(Point { x: 1.5f32, y: 2.5f32 });
        vec.push(Point { x: 2.0f32, y: 3.0f32 });
        vec.push(Point { x: 3.0f32, y: 2.0f32 });
        vec.push(Point { x: 3.0f32, y: 1.0f32 });
        vec.push(Point { x: 2.5f32, y: 1.5f32 });
        vec.push(Point { x: 1.5f32, y: 0.5f32 });
        vec.push(Point { x: 0.5f32, y: 1.0f32 });

        let edge_list = make_edge_list(&vec);

        assert_eq!(edge_list[0].origin, &vec[0]);
        assert_eq!(edge_list[edge_list[0].prev_index].origin, &vec[n-1]);
        assert_eq!(edge_list[edge_list[0].next_index].origin, &vec[1]);

        assert_eq!(edge_list[1].origin, &vec[1]);
        assert_eq!(edge_list[edge_list[1].prev_index].origin, &vec[0]);
        assert_eq!(edge_list[edge_list[1].next_index].origin, &vec[2]);

        assert_eq!(edge_list[2].origin, &vec[2]);
        assert_eq!(edge_list[edge_list[2].prev_index].origin, &vec[1]);
        assert_eq!(edge_list[edge_list[2].next_index].origin, &vec[3]);


        assert_eq!(edge_list[3].origin, &vec[3]);
        assert_eq!(edge_list[edge_list[3].prev_index].origin, &vec[2]);
        assert_eq!(edge_list[edge_list[3].next_index].origin, &vec[4]);

        assert_eq!(edge_list[4].origin, &vec[4]);
        assert_eq!(edge_list[edge_list[4].prev_index].origin, &vec[3]);
        assert_eq!(edge_list[edge_list[4].next_index].origin, &vec[5]);

        assert_eq!(edge_list[5].origin, &vec[5]);
        assert_eq!(edge_list[edge_list[5].prev_index].origin, &vec[4]);
        assert_eq!(edge_list[edge_list[5].next_index].origin, &vec[6]);

        assert_eq!(edge_list[6].origin, &vec[6]);
        assert_eq!(edge_list[edge_list[6].prev_index].origin, &vec[5]);
        assert_eq!(edge_list[edge_list[6].next_index].origin, &vec[7]);

        assert_eq!(edge_list[7].origin, &vec[7]);
        assert_eq!(edge_list[edge_list[7].prev_index].origin, &vec[6]);
        assert_eq!(edge_list[edge_list[7].next_index].origin, &vec[8]);

        assert_eq!(edge_list[8].origin, &vec[8]);
        assert_eq!(edge_list[edge_list[8].prev_index].origin, &vec[7]);
        assert_eq!(edge_list[edge_list[8].next_index].origin, &vec[0]);
        
    }

    #[test]
    fn test_classify_vertex() {

        let n = 9;
        let mut vec = Vec::with_capacity(n);
        vec.push(Point { x: 0.5f32, y: 2.5f32 });
        vec.push(Point { x: 1.0f32, y: 3.0f32 });
        vec.push(Point { x: 1.5f32, y: 2.5f32 });
        vec.push(Point { x: 2.0f32, y: 3.0f32 });
        vec.push(Point { x: 3.0f32, y: 2.0f32 });
        vec.push(Point { x: 3.0f32, y: 1.0f32 });
        vec.push(Point { x: 2.5f32, y: 1.5f32 });
        vec.push(Point { x: 1.5f32, y: 0.5f32 });
        vec.push(Point { x: 0.5f32, y: 1.0f32 });

        let edge_list = make_edge_list(&vec);

        assert_eq!(classify_vertex(&edge_list, 0), Vertex::new(&vec[0], VertexType::Regular, 0));
        assert_eq!(classify_vertex(&edge_list, 1), Vertex::new(&vec[1], VertexType::Start,   1));
        assert_eq!(classify_vertex(&edge_list, 2), Vertex::new(&vec[2], VertexType::Merge,   2));
        assert_eq!(classify_vertex(&edge_list, 3), Vertex::new(&vec[3], VertexType::Start,   3));
        assert_eq!(classify_vertex(&edge_list, 4), Vertex::new(&vec[4], VertexType::Regular, 4));
        assert_eq!(classify_vertex(&edge_list, 5), Vertex::new(&vec[5], VertexType::End,     5));
        assert_eq!(classify_vertex(&edge_list, 6), Vertex::new(&vec[6], VertexType::Split,   6));
        assert_eq!(classify_vertex(&edge_list, 7), Vertex::new(&vec[7], VertexType::End,     7));
        assert_eq!(classify_vertex(&edge_list, 8), Vertex::new(&vec[8], VertexType::Regular, 8));
    }                                        
                                             
    #[test]                                  
    fn test_build_vertex_queue() {           
                                             
        let n = 9;
        let mut vec = Vec::with_capacity(n);
        vec.push(Point { x: 0.5f32, y: 2.5f32 }); // 2
        vec.push(Point { x: 1.0f32, y: 3.0f32 }); // 0
        vec.push(Point { x: 1.5f32, y: 2.5f32 }); // 3
        vec.push(Point { x: 2.0f32, y: 3.0f32 }); // 1
        vec.push(Point { x: 3.0f32, y: 2.0f32 }); // 4
        vec.push(Point { x: 3.0f32, y: 1.0f32 }); // 7
        vec.push(Point { x: 2.5f32, y: 1.5f32 }); // 5
        vec.push(Point { x: 1.5f32, y: 0.5f32 }); // 8
        vec.push(Point { x: 0.5f32, y: 1.0f32 }); // 6

        let edge_list = make_edge_list(&vec);

        let mut vertex_queue = build_vertex_queue(&edge_list);

        // vertices should now be sorted
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[1], VertexType::Start,   1)); // 0
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[3], VertexType::Start,   3)); // 1
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[0], VertexType::Regular, 0)); // 2
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[2], VertexType::Merge,   2)); // 3
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[4], VertexType::Regular, 4)); // 4
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[6], VertexType::Split,   6)); // 5
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[8], VertexType::Regular, 8)); // 6
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[5], VertexType::End,     5)); // 7
        assert_eq!(vertex_queue.pop().unwrap(), Vertex::new(&vec[7], VertexType::End,     7)); // 8
    }
    
    #[test]
    fn test_insert_edge() {
        let n = 4;
        let mut vec = Vec::with_capacity(n);
        vec.push(Point { x: 0.0f32, y: 1.0f32 });
        vec.push(Point { x: 0.0f32, y: 0.0f32 });
        vec.push(Point { x: 1.0f32, y: 0.0f32 });
        vec.push(Point { x: 1.0f32, y: 1.0f32 });

         let mut edge_list = make_edge_list(&vec);
         {
             let mut edge_ref = &mut edge_list;
             insert_edge(&mut edge_ref, 0, 2);
         }
         assert_eq!(edge_list[0], HalfEdge { origin: &vec[0], prev_index: 5, next_index: 1 });
         assert_eq!(edge_list[1], HalfEdge { origin: &vec[1], prev_index: 0, next_index: 5 });
         assert_eq!(edge_list[5], HalfEdge { origin: &vec[2], prev_index: 1, next_index: 0 });

         assert_eq!(edge_list[3], HalfEdge { origin: &vec[3], prev_index: 2, next_index: 4 });
         assert_eq!(edge_list[4], HalfEdge { origin: &vec[0], prev_index: 3, next_index: 2 });
         assert_eq!(edge_list[2], HalfEdge { origin: &vec[2], prev_index: 4, next_index: 3 });
         
    }
    
}

