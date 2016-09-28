// module for triangulating a simple polygon
#![feature(btree_range, collections_bound)]

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::BTreeSet;
use std::collections::Bound::{Included, Excluded, Unbounded};

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

 #[derive(Debug, Clone, Copy)]
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

struct Triangulator<'a> {
    points: &'a Vec<Point>,
    edge_list: Vec<HalfEdge<'a>>,
    vertex_queue: BinaryHeap<Vertex<'a>>,
    btree: BTreeSet<TEdge<'a>>,
    prev_t_edge: Option<TEdge<'a>>
}

impl<'a> Triangulator<'a> {
    fn triangulate(points: &'a mut Vec<Point>) {
        let mut this = Triangulator { points: points, 
                                      edge_list: Vec::new(), 
                                      vertex_queue: BinaryHeap::new(), 
                                      btree: BTreeSet::new(), 
                                      prev_t_edge: None };
        this.build_edge_list();
        this.build_vertex_queue();

    }

    fn build_edge_list(&mut self) {
        let n = self.points.len();
        self.edge_list.push(HalfEdge { origin: &self.points[0], prev_index: n-1, next_index: 1});
        for (i, p) in self.points.iter().enumerate().skip(1) {
            self.edge_list.push(HalfEdge { origin: p, prev_index: i-1, next_index: i+1 })
        }
        self.edge_list[n-1].next_index = 0;    
    }

    fn classify_vertex(&self, index: usize) -> Vertex<'a> {
        let ref edge = self.edge_list[index];
        let current = edge.origin;
        let prev = self.edge_list[edge.prev_index].origin;
        let next = self.edge_list[edge.next_index].origin;

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

    fn build_vertex_queue(&mut self) {
        for index  in 0..self.edge_list.len() {
            let vertex = self.classify_vertex(index);
            self.vertex_queue.push(vertex);
        }
    }

    fn insert_edge(&mut self, start_index: usize, end_index: usize) {
        let new_index = self.edge_list.len();
        let twin_index = new_index + 1;
    
        let new_edge = HalfEdge { origin: self.edge_list[start_index].origin, 
                                  prev_index: self.edge_list[start_index].prev_index, 
                                  next_index: end_index };

        let twin_edge = HalfEdge { origin: self.edge_list[end_index].origin,
                                   prev_index: self.edge_list[end_index].prev_index,
                                   next_index: start_index };
      
        self.edge_list[new_edge.prev_index].next_index = new_index;
        self.edge_list[start_index].prev_index = twin_index;

        self.edge_list[twin_edge.prev_index].next_index = twin_index;
        self.edge_list[end_index].prev_index = new_index;

        self.edge_list.push(new_edge);
        self.edge_list.push(twin_edge);
    }

    fn  handle_start_vertex(&'a mut self, vertex: &'a Vertex) {
        self.prev_t_edge = Some(TEdge { edge: &self.edge_list[vertex.edge_index], helper: vertex });
        self.btree.insert(self.prev_t_edge.unwrap()); // TODO: Robust error handling
    }

    fn handle_end_vertex(&'a mut self, vertex: &Vertex) {
        let t_edge = self.prev_t_edge.unwrap();
        if t_edge.helper.vertex_type == VertexType::Merge { // TODO: Robust error handling
            self.insert_edge(vertex.edge_index, t_edge.helper.edge_index); // TODO: Robust error handling
        }
        self.btree.remove(&self.prev_t_edge.unwrap());  // TODO: Robust error handling
        self.prev_t_edge = None;
    }

    fn handle_split_vertex(&'a mut self, vertex: &'a Vertex<'a>) {
        let edge = TEdge { edge: &self.edge_list[vertex.edge_index], helper: vertex };
        self.prev_t_edge = Some(edge);
        self.btree.insert(edge);
 
        let mut left = *self.btree.range(Unbounded, Excluded(&edge)).next_back().unwrap();
        let helper_index = left.helper.edge_index;
        

        self.btree.remove(&left);
        left.helper = vertex;
        self.btree.insert(left);
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
}

#[cfg(test)]
mod tests {
    use std::collections::BinaryHeap;
    use std::collections::BTreeSet;
    use super::Point;
    use super::HalfEdge;
    use super::VertexType;
    use super::Vertex;
    use Triangulator;

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
        let mut points = Vec::with_capacity(n);
        points.push(Point { x: 0.5f32, y: 2.5f32 });
        points.push(Point { x: 1.0f32, y: 3.0f32 });
        points.push(Point { x: 1.5f32, y: 2.5f32 });
        points.push(Point { x: 2.0f32, y: 3.0f32 });
        points.push(Point { x: 3.0f32, y: 2.0f32 });
        points.push(Point { x: 3.0f32, y: 1.0f32 });
        points.push(Point { x: 2.5f32, y: 1.5f32 });
        points.push(Point { x: 1.5f32, y: 0.5f32 });
        points.push(Point { x: 0.5f32, y: 1.0f32 });

        let mut tri = Triangulator { points: &points, 
                                      edge_list: Vec::new(), 
                                      vertex_queue: BinaryHeap::new(), 
                                      btree: BTreeSet::new(), 
                                      prev_t_edge: None };

        tri.build_edge_list();

        assert_eq!(tri.edge_list[0].origin, &points[0]);
        assert_eq!(tri.edge_list[tri.edge_list[0].prev_index].origin, &points[n-1]);
        assert_eq!(tri.edge_list[tri.edge_list[0].next_index].origin, &points[1]);

        assert_eq!(tri.edge_list[1].origin, &points[1]);
        assert_eq!(tri.edge_list[tri.edge_list[1].prev_index].origin, &points[0]);
        assert_eq!(tri.edge_list[tri.edge_list[1].next_index].origin, &points[2]);

        assert_eq!(tri.edge_list[2].origin, &points[2]);
        assert_eq!(tri.edge_list[tri.edge_list[2].prev_index].origin, &points[1]);
        assert_eq!(tri.edge_list[tri.edge_list[2].next_index].origin, &points[3]);


        assert_eq!(tri.edge_list[3].origin, &points[3]);
        assert_eq!(tri.edge_list[tri.edge_list[3].prev_index].origin, &points[2]);
        assert_eq!(tri.edge_list[tri.edge_list[3].next_index].origin, &points[4]);

        assert_eq!(tri.edge_list[4].origin, &points[4]);
        assert_eq!(tri.edge_list[tri.edge_list[4].prev_index].origin, &points[3]);
        assert_eq!(tri.edge_list[tri.edge_list[4].next_index].origin, &points[5]);

        assert_eq!(tri.edge_list[5].origin, &points[5]);
        assert_eq!(tri.edge_list[tri.edge_list[5].prev_index].origin, &points[4]);
        assert_eq!(tri.edge_list[tri.edge_list[5].next_index].origin, &points[6]);

        assert_eq!(tri.edge_list[6].origin, &points[6]);
        assert_eq!(tri.edge_list[tri.edge_list[6].prev_index].origin, &points[5]);
        assert_eq!(tri.edge_list[tri.edge_list[6].next_index].origin, &points[7]);

        assert_eq!(tri.edge_list[7].origin, &points[7]);
        assert_eq!(tri.edge_list[tri.edge_list[7].prev_index].origin, &points[6]);
        assert_eq!(tri.edge_list[tri.edge_list[7].next_index].origin, &points[8]);

        assert_eq!(tri.edge_list[8].origin, &points[8]);
        assert_eq!(tri.edge_list[tri.edge_list[8].prev_index].origin, &points[7]);
        assert_eq!(tri.edge_list[tri.edge_list[8].next_index].origin, &points[0]);
        
    }

    #[test]
    fn test_classify_vertex() {

        let n = 9;
        let mut points = Vec::with_capacity(n);
        points.push(Point { x: 0.5f32, y: 2.5f32 });
        points.push(Point { x: 1.0f32, y: 3.0f32 });
        points.push(Point { x: 1.5f32, y: 2.5f32 });
        points.push(Point { x: 2.0f32, y: 3.0f32 });
        points.push(Point { x: 3.0f32, y: 2.0f32 });
        points.push(Point { x: 3.0f32, y: 1.0f32 });
        points.push(Point { x: 2.5f32, y: 1.5f32 });
        points.push(Point { x: 1.5f32, y: 0.5f32 });
        points.push(Point { x: 0.5f32, y: 1.0f32 });

        let mut tri = Triangulator { points: &points, 
                                      edge_list: Vec::new(), 
                                      vertex_queue: BinaryHeap::new(), 
                                      btree: BTreeSet::new(), 
                                      prev_t_edge: None };

        tri.build_edge_list();

        assert_eq!(tri.classify_vertex(0), Vertex::new(&points[0], VertexType::Regular, 0));
        assert_eq!(tri.classify_vertex(1), Vertex::new(&points[1], VertexType::Start,   1));
        assert_eq!(tri.classify_vertex(2), Vertex::new(&points[2], VertexType::Merge,   2));
        assert_eq!(tri.classify_vertex(3), Vertex::new(&points[3], VertexType::Start,   3));
        assert_eq!(tri.classify_vertex(4), Vertex::new(&points[4], VertexType::Regular, 4));
        assert_eq!(tri.classify_vertex(5), Vertex::new(&points[5], VertexType::End,     5));
        assert_eq!(tri.classify_vertex(6), Vertex::new(&points[6], VertexType::Split,   6));
        assert_eq!(tri.classify_vertex(7), Vertex::new(&points[7], VertexType::End,     7));
        assert_eq!(tri.classify_vertex(8), Vertex::new(&points[8], VertexType::Regular, 8));
    }                                        
                                             
    #[test]                                  
    fn test_build_vertex_queue() {           
                                             
        let n = 9;
        let mut points = Vec::with_capacity(n);
        points.push(Point { x: 0.5f32, y: 2.5f32 }); // 2
        points.push(Point { x: 1.0f32, y: 3.0f32 }); // 0
        points.push(Point { x: 1.5f32, y: 2.5f32 }); // 3
        points.push(Point { x: 2.0f32, y: 3.0f32 }); // 1
        points.push(Point { x: 3.0f32, y: 2.0f32 }); // 4
        points.push(Point { x: 3.0f32, y: 1.0f32 }); // 7
        points.push(Point { x: 2.5f32, y: 1.5f32 }); // 5
        points.push(Point { x: 1.5f32, y: 0.5f32 }); // 8
        points.push(Point { x: 0.5f32, y: 1.0f32 }); // 6

        let mut tri = Triangulator { points: &points, 
                                      edge_list: Vec::new(), 
                                      vertex_queue: BinaryHeap::new(), 
                                      btree: BTreeSet::new(), 
                                      prev_t_edge: None };

        tri.build_edge_list();

        tri.build_vertex_queue();

        // vertices should now be sorted
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[1], VertexType::Start,   1)); // 0
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[3], VertexType::Start,   3)); // 1
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[0], VertexType::Regular, 0)); // 2
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[2], VertexType::Merge,   2)); // 3
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[4], VertexType::Regular, 4)); // 4
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[6], VertexType::Split,   6)); // 5
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[8], VertexType::Regular, 8)); // 6
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[5], VertexType::End,     5)); // 7
        assert_eq!(tri.vertex_queue.pop().unwrap(), Vertex::new(&points[7], VertexType::End,     7)); // 8
    }
    
    #[test]
    fn test_insert_edge() {
        let n = 4;
        let mut points = Vec::with_capacity(n);
        points.push(Point { x: 0.0f32, y: 1.0f32 });
        points.push(Point { x: 0.0f32, y: 0.0f32 });
        points.push(Point { x: 1.0f32, y: 0.0f32 });
        points.push(Point { x: 1.0f32, y: 1.0f32 });

         let mut tri = Triangulator { points: &points, 
                                      edge_list: Vec::new(), 
                                      vertex_queue: BinaryHeap::new(), 
                                      btree: BTreeSet::new(), 
                                      prev_t_edge: None };

        tri.build_edge_list();
        tri.insert_edge(0, 2);

         assert_eq!(tri.edge_list[0], HalfEdge { origin: &points[0], prev_index: 5, next_index: 1 });
         assert_eq!(tri.edge_list[1], HalfEdge { origin: &points[1], prev_index: 0, next_index: 5 });
         assert_eq!(tri.edge_list[5], HalfEdge { origin: &points[2], prev_index: 1, next_index: 0 });

         assert_eq!(tri.edge_list[3], HalfEdge { origin: &points[3], prev_index: 2, next_index: 4 });
         assert_eq!(tri.edge_list[4], HalfEdge { origin: &points[0], prev_index: 3, next_index: 2 });
         assert_eq!(tri.edge_list[2], HalfEdge { origin: &points[2], prev_index: 4, next_index: 3 });
         
    }

}
