use std::collections::{HashMap, BinaryHeap};
use csv::ReaderBuilder; 
use std::error::Error;
use std::fs::File;
use csv::Writer;
use serde::Deserialize; 
use std::cmp::Ordering;
use std::env;
mod analysis;

// these are the 4 columns from the dataset I choose
#[derive(Debug, Deserialize, Clone)]
struct Record {
    source: i32,
    target: i32,
    rating: i32,
    time: i64,
}

// define the structure of an edge
#[derive(Debug)]
struct Edge {
    target: i32,
    trust_score: i32,
}

// define the structure of the graph
struct Graph {
    nodes: HashMap<i32, Vec<Edge>>,
}

impl Graph {
    // Create a new empty graph
    fn new() -> Graph {
        Graph {
            nodes: HashMap::new(),
        }
    }

    // add an edge to the graph
    fn add_edge(&mut self, source: i32, target: i32, trust_score: i32) {
        let edge = Edge { target, trust_score };
        self.nodes.entry(source).or_insert_with(Vec::new).push(edge);
    }
}

#[derive(Copy, Clone)]
struct State {
    cost: f64, // the total distance taken to reach a node from the starting node
    position: i32, 
    node_count: i32, 
}

// checking if two values of this type are equal or not
impl Eq for State {}


impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool { // return true if the two instances being compared are equal
        self.position == other.position
    }
}

// implement ordering for State based on cost
// prioritize lower cost
impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // flip the ordering here, because for Dijkstra algorithm, I need smallest element at the top
        other.cost.partial_cmp(&self.cost).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &State) -> Option<Ordering> { // handle cases where elements are not comparable, such as NaN
        Some(self.cmp(other))
    }
}

// build a graph from Record structs, and also output different generation depends on the input command
fn construct_graph(records: &[Record], generation: usize) -> (Graph, i64) {
    // Check for empty records
    if records.is_empty() {
        return (Graph::new(), 0);
    }

    let len = records.len();
    let mut end_index = match generation {
        1 => len / 3,            // end at 1/3 for the first generation
        2 => (len * 2) / 3,      // end at 2/3 for the second generation
        _ => len,                // include all records for the third generation
    };

    // ensuring that the end_index does not exceed the length of records
    end_index = end_index.min(len);

    let filtered_records = &records[0..end_index];

    // build the graph
    let mut graph = Graph::new();
    for record in filtered_records {
        graph.add_edge(record.source, record.target, record.rating);
    }

    // tells me the time of the last transaction, the unit is still epoch since time
    let last_transac = if !filtered_records.is_empty() {
        filtered_records.last().unwrap().time
    } else {
        0
    };

    (graph, last_transac)
}

// takes the graph and output a hashmap where it records all the highes possible scores that the starting node can give to the rest
// finding the shortest paths (paths with the highest trust score)
fn modified_dijkstra(graph: &Graph, start_node: i32) -> HashMap<i32, f64> {
    let mut dist: HashMap<i32, (f64, i32)> = HashMap::new();
    let mut heap = BinaryHeap::new();

    // initialize start_node distance as 0 and node_count as 1 (the start node itself)
    dist.insert(start_node, (0.0, 1));
    heap.push(State { cost: 0.0, position: start_node, node_count: 1 });

    // if the cost of the popped state is greater than the recorded distance in dist for that position, 
    // then it continues to the next iteration
    while let Some(State { cost, position, node_count }) = heap.pop() {
        if let Some(&(current_dist, _)) = dist.get(&position) {
            if cost > current_dist {
                continue;
            }
        }
        
        // exploring the neighboring nodes of the current node
        if let Some(edges) = graph.nodes.get(&position) { // if there are any outgoing edges from the current node
            for edge in edges {
                let next_cost = cost + 1.0 / (edge.trust_score + 11) as f64; // this is where i do the conversion
                let next_position = edge.target; // determines the next node
                let next_node_count = node_count + 1;
                
                // check if the new path is shorter
                let is_better = dist
                    .entry(next_position) 
                    .or_insert((f64::INFINITY, 0)) //if no path to next_position was recorded before, it will write infinity
                    .0 > next_cost;
                
                // if true, update the path and node count
                if is_better {
                    dist.insert(next_position, (next_cost, next_node_count));
                    heap.push(State { cost: next_cost, position: next_position, node_count: next_node_count });
                }
            }
        }
    }

    // convert total cost to average cost by dividing by the node count
    let mut average_scores = HashMap::new();
    for (node, (total_cost, count)) in dist {
        if count > 0 {
            average_scores.insert(node, total_cost / count as f64);
        }
    }

    average_scores
}

fn main() -> Result<(), Box<dyn Error>> { // flexible way to handle errors

    let generation = 3; // can be any value from 1 to 3

    let path = "/Users/garrickzhang/Desktop/GZ/BU/Sophmore/DS 210/Final Project/soc-sign-bitcoinotc.csv";

    // create the CSV reader 
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_path(path)
        .expect("Cannot read CSV file");

    // I used deserialize based on research
    // it takes each row from the CSV file and automatically tries to fit it into the Record struct i defined earlier
    let records: Vec<Record> = rdr.deserialize()
        .map(|result| result.expect("Error parsing record"))
        .collect();

    let (graph, last_transac) = construct_graph(&records, generation);

    // read the command to see if I wanted to run analysis of the file or creating a file first
    // I put the code in here because I need the last_transac variable
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "analyze" => {
                analysis::main(generation, last_transac)?;
                return Ok(()); // exit after analysis
            },
            _ => println!("Unknown command"),
        }
    }

    let file = File::create("trust_scores.csv")?;
    let mut wtr = Writer::from_writer(file);

    // Iterate over each node, treated as the starting node
    for &start_node in graph.nodes.keys() {
        let trust_scores = modified_dijkstra(&graph, start_node);

        // prepare a record for each node
        let mut record = Vec::new();
        record.push(start_node.to_string()); // first column is the start node

        for &node in graph.nodes.keys() {
            let score = trust_scores.get(&node).unwrap_or(&f64::INFINITY);
            record.push(score.to_string());
        }

        // write the record to the CSV
        wtr.write_record(&record)?;
    }

    wtr.flush()?; // make sure it's saved
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // create a small test graph
    fn create_test_graph() -> Graph {
        let mut graph = Graph::new();
        // create a small graph with known trust scores
        graph.add_edge(0, 1, 5); // node 0 -> Node 1 with trust score 5
        graph.add_edge(0, 2, 10); 
        graph.add_edge(1, 2, 2); // node 1 -> Node 2 with trust score 2
        graph.add_edge(2, 0, 1); 
        graph
    }

    #[test]
    fn test_modified_dijkstra() {
        let graph = create_test_graph();

        // Test the algorithm starting from Node 0
        let start_node = 0;
        let trust_scores = modified_dijkstra(&graph, start_node);

        // Expected scores (calculated manually for the test graph)
        let expected_scores = vec![
            (0, 0.0), // distance to itself is 0
            (1, 0.03125), // score to Node 1, manually calculated
            (2, 0.02381), // score to Node 2, manually calculated
        ];

        // Check if the scores match the expected values
        for (node, expected_score) in &expected_scores {
            assert!(
                trust_scores.get(&node).is_some(),
                "Node {} should have a trust score", node
            );
            let score = trust_scores.get(&node).unwrap();
            assert!(
                format!("{:.5}", *score) == format!("{:.5}", expected_score),
                "Trust score for node {} is not as expected: got {:.5}, expected {:.5}", node, score, expected_score
            );
        }
    }

    #[test]
    fn test_graph_construction() {
        let records = vec![
            Record { source: 1, target: 2, rating: 3, time: 100 },
            Record { source: 2, target: 3, rating: 4, time: 200 },
        ];
        let (graph, _) = construct_graph(&records, 3); //generation 3 for the full graph

        assert_eq!(graph.nodes.len(), 2); // check if the graph has 2 nodes
        assert!(graph.nodes.get(&1).is_some()); // check if node 1 exists
        assert!(graph.nodes.get(&2).is_some()); // check if node 2 exists
        assert_eq!(graph.nodes.get(&1).unwrap().len(), 1); // check if node 1 has 1 edge
    }
}
