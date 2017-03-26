use std::cmp::Ordering;
use std::collections::{HashMap,HashSet};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Node {
    Column(String),
    Constraint(String),
    Function(String),
    Table(String),
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub node: Node,
    pub weight: f32,
}

impl Edge {
    pub fn new(node: &Node, weight: f32) -> Self {
        Edge {
            node: node.clone(),
            weight: weight,
        }
    }
}

pub struct DependencyGraph {
    edges: HashMap<Node, Vec<Edge>>,
    unresolved: HashSet<Node>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ValidationResult {
    Valid,
    UnresolvedDependencies,
    CircularReference
}

struct DepthFirstSearchState {
    pre : Vec<usize>, // fixed size array
    post : Vec<usize>, // fixed size array
    pre_order: Vec<usize>, // queue
    post_order: Vec<usize>, // queue
    marked: Vec<bool>, // fixed size array
    pre_counter: usize, // Tracks current position in pre
    post_counter: usize,  // Tracks current position in post
}

impl DepthFirstSearchState {
    fn new(size: usize) -> Self {
        DepthFirstSearchState {
            pre : DepthFirstSearchState::fixed_len::<usize>(size, 0),
            post : DepthFirstSearchState::fixed_len::<usize>(size, 0),
            pre_order : Vec::new(),
            post_order : Vec::new(),
            marked : DepthFirstSearchState::fixed_len::<bool>(size, false),
            pre_counter : 0,
            post_counter : 0,
        }
    }

    fn fixed_len<T>(size: usize, def: T) -> Vec<T> where T: Copy {
        let mut zero_vec: Vec<T> = Vec::with_capacity(size);
        for _ in 0..size {
            zero_vec.push(def);
        }
        zero_vec
    }

    fn update_pre_order(&mut self, v: usize) {
        self.pre[v] = self.pre_counter;
        self.pre_counter = self.pre_counter + 1;
        self.pre_order.push(v);
    }

    fn update_post_order(&mut self, v: usize) {
        self.post_order.push(v);
        self.post[v] = self.post_counter;
        self.post_counter = self.post_counter + 1;
    }

    // check that preorder and postorder are consistent with pre[v] and post[v]
    fn validate(&self) -> bool {

        // Has everything been visited?
        for m in &self.marked {
            if !m {
                return false;
            }
        }

        // check that post[v] is consistent with post_order
        let mut r = 0;
        for v in self.post_order.iter() {
            if self.post[*v] != r {
                return false;
            }
            r = r + 1;
        }

        // check that pre[v] is consistent with pre_order
        r = 0;
        for v in self.pre_order.iter() {
            if self.pre[*v] != r {
                return false;
            }
            r = r + 1;
        }

        true
    }    
}

/*
 * Our dependency graph is edge weighted. This is a multiplier and bubbled up.
 * e.g. assume we have a FK constaint. This will weight the reference column over
 *      the FK as it is more important for that table to be deployed first.
 * This approach could back fire if we have circular references, however we try
 * to eliminate those during validation.
 */
impl DependencyGraph {

    pub fn new() -> Self {
        DependencyGraph {
            edges: HashMap::new(),
            unresolved: HashSet::new(),
        }
    }

    pub fn add_node(&mut self, node: &Node) {
        self.add_node_with_edges(node, Vec::new());
    }

    pub fn add_node_with_edges(&mut self, node: &Node, edges: Vec<Edge>) {
        // If it has already been added then panic
        if self.edges.contains_key(node) {
            panic!("Node has already been registered: {:?}", node);
        }
        // Remove it from unresolved if it exists
        self.unresolved.remove(node);
        self.edges.insert(node.clone(), edges);
    }

    pub fn add_edge(&mut self, node: &Node, edge: Edge) {
        let dependency_known = self.edges.contains_key(&edge.node);
        if let Some(n) = self.edges.get_mut(node) {
            // Only add the dependency if necessary
            if !dependency_known {
                self.unresolved.insert(edge.node.clone());
            }
            // Clone instead of copy, perhaps an optimization here later on
            n.push(edge);
        } else {
            panic!("Cannot add dependencies as node not known: {:?}", *node);
        }
    }

    pub fn validate(&self) -> ValidationResult {
        if !self.unresolved.is_empty() {
            return ValidationResult::UnresolvedDependencies;
        } 

        // Check for circular references. It's only circular if the root edge is seen twice.
        for node in self.edges.keys() {
            if self.visit_node(node, node) {
                return ValidationResult::CircularReference;
            }
        }

        ValidationResult::Valid
    }

    fn visit_node(&self, root: &Node, lookup: &Node) -> bool {
        if let Some(edges) = self.edges.get(lookup) {
            for edge in edges {
                if edge.node.eq(root) {
                    return true;
                }

                if self.visit_node(root, &edge.node) {
                    return true;
                }
            }
        }
        false
    }    

    pub fn unresolved(&self) -> Vec<Node> {
        self.unresolved.iter().map(|x| x.clone()).collect()
    }

    // TODO: Does not work for what I need, relies on reverse dependents
    pub fn topological_graph(&self) -> Vec<Node> {
        // Create a graph vec to track state for directed acyclic graph
        // First, we need to sort it according it edge weight
        // The order is relevant for the DFS algorithm!
        let mut table = HashMap::new();
        // Set up the hash map first 
        for node in self.edges.keys() {
            table.insert(node.clone(), 1.0);
        }
        // Now recursively do this again applying the weight rules
        for node in self.edges.keys() {
            self.calculate_weight(&mut table, node, 1.0);
        }

        let mut weighted_graph : Vec<(Node, f32)> = table.into_iter().collect();;
        weighted_graph.sort_by(|a,b| { 
            let c = (b.1 - a.1).abs();
            // Using an epsilon for equality
            if c <= 0.00000000000001 {
                a.0.cmp(&b.0)
            } else if b.1 < a.1 {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });
        for item in &weighted_graph {
            println!("{:?} {}", item.0, item.1);
        }
        let graph : Vec<Node> = weighted_graph.iter().map(|k| k.0.clone()).collect();

        // Create the state
        let mut state = DepthFirstSearchState::new(graph.len());

        // Do a DFS and compute preorder/postorder
        for v in 0..graph.len() {
            if !state.marked[v] {
                self.dfs(&graph, v, &mut state);
            }
        }

        if !state.validate() {
            panic!("Directed Acyclic Graph in invalid state");
        }

        // Finally, reverse the post order to build the topological graph
        let mut result = Vec::new();
        for v in state.post_order {
            result.push(graph[v].clone());
        }
        result
    }

    fn dfs(&self, graph: &Vec<Node>, v: usize, state: &mut DepthFirstSearchState) {
        state.marked[v] = true;
        state.update_pre_order(v);
        if let Some(edges) = self.edges.get(&graph[v]) {
            for edge in edges {
                // Look up location in graph
                if let Ok(w) = graph.binary_search(&edge.node) {
                    if !state.marked[w] {
                        self.dfs(graph, w, state);
                    }
                }
            }
        }
        state.update_post_order(v);
    }

    fn calculate_weight(&self, table: &mut HashMap<Node,f32>, current_node: &Node, weight: f32) {
        if let Some(edges) = self.edges.get(current_node) {
            for edge in edges {
                let new_weight = edge.weight * weight;
                if let Some(x) = table.get_mut(&edge.node) {
                    *x = *x * new_weight;
                }
                self.calculate_weight(table, &edge.node, new_weight);
            }
        }
    }
}

#[test]
#[should_panic]
fn it_panics_if_adding_a_duplicate_node() {
    let mut graph = DependencyGraph::new();
    let table = Node::Table("public.users".to_owned());
    graph.add_node(&table);
    // Clone just in case
    graph.add_node(&table.clone());
}

#[test]
#[should_panic]
fn it_panics_if_adding_a_dependency_to_a_node_that_doesnt_exist() {
    let mut graph = DependencyGraph::new();
    let table_org = Node::Table("public.org".to_owned());
    let col_id = Node::Column("public.org.id".to_owned());
    graph.add_node(&table_org);
    graph.add_edge(&col_id, Edge::new(&table_org, 1.0));    
}

#[test]
fn it_tracks_dependencies() {
    let mut graph = DependencyGraph::new();
    let table_user = Node::Table("public.user".to_owned());
    let col_id = Node::Column("public.user.id".to_owned());
    let col_name = Node::Column("public.user.name".to_owned());
    graph.add_node(&col_id);
    graph.add_node(&col_name);
    assert_eq!(ValidationResult::Valid, graph.validate());
    graph.add_edge(&col_id, Edge::new(&table_user, 1.0));
    graph.add_edge(&col_name, Edge::new(&table_user, 1.0));
    assert_eq!(ValidationResult::UnresolvedDependencies, graph.validate());
    let unresolved = graph.unresolved();
    assert_eq!(1, unresolved.len());
    assert_eq!(Node::Table("public.user".to_owned()), unresolved[0]);

    graph.add_node(&table_user);
    assert_eq!(ValidationResult::Valid, graph.validate());
}

#[test]
fn it_detects_circular_dependencies() {
    let mut graph = DependencyGraph::new();
    let table_user = Node::Table("public.user".to_owned());
    let col_id = Node::Column("public.user.id".to_owned());
    let col_name = Node::Column("public.user.name".to_owned());
    graph.add_node(&table_user);
    graph.add_node(&col_id);
    graph.add_node(&col_name);
    graph.add_edge(&col_id, Edge::new(&table_user, 1.0));
    graph.add_edge(&col_name, Edge::new(&table_user, 1.0));
    // Not realistic, but testing circular references nevertheless
    graph.add_edge(&col_id, Edge::new(&col_name, 1.0));
    graph.add_edge(&col_name, Edge::new(&col_id, 1.0));
    assert_eq!(ValidationResult::CircularReference, graph.validate());
}

#[test]
fn it_generates_a_topological_graph() {
    let mut graph = DependencyGraph::new();
    // This is a more complex test. It represents the following structures:
    //    CREATE TABLE data.versions(
    //        id serial NOT NULL
    //    );
    //    CREATE TABLE data.coefficients(
    //        id serial NOT NULL, 
    //        version_id int NOT NULL,
    //        CONSTRAINT fk_coefficients__version_id FOREIGN KEY (version_id) 
    //          REFERENCES data.versions (id) MATCH SIMPLE
    //          ON UPDATE NO ACTION ON DELETE NO ACTION
    //    );
    let table_versions = Node::Table("data.versions".to_owned());
    let table_coefficients = Node::Table("data.coefficients".to_owned());
    let column_versions_id = Node::Column("data.versions.id".to_owned());
    let column_coefficients_id = Node::Column("data.coefficients.id".to_owned());
    let column_coefficients_version_id = Node::Column("data.coefficients.version_id".to_owned());
    let constraint_coefficients_version_id = Node::Constraint("data.coefficients.fk_coefficients__version_id".to_owned());

    // Note: We do this in an unordered way purposely

    // Coefficients table
    graph.add_node(&table_coefficients);
    // Add each column for this table, each column needs the table to exist
    graph.add_node_with_edges(&column_coefficients_id, vec!(
        Edge::new(&table_coefficients, 1.0)
        )); 
    graph.add_node_with_edges(&column_coefficients_version_id, vec!(
        Edge::new(&table_coefficients, 1.0)
        )); 
    // Add constraint - the constraint needs the columns created
    graph.add_node_with_edges(&constraint_coefficients_version_id, vec!(
        Edge::new(&column_coefficients_version_id, 1.0), // FK
        Edge::new(&column_versions_id, 1.1), // Reference
        ));

    // Versions table
    graph.add_node(&table_versions);
    // The column needs the table
    graph.add_node_with_edges(&column_versions_id, vec!(
        Edge::new(&table_versions, 1.0)
        ));

    // Now, let's validate to make sure it's a valid graph
    assert_eq!(ValidationResult::Valid, graph.validate());

    // Now, order it.
    let ordered = graph.topological_graph();

    //TODO: Need to make this edge weighted as a constraint on a table defines that the other table has to exist first 
    // (i.e. versions before coefficient)
    // The expected order:
    let expected = [
        table_versions, // Versions must be first as coefficients has a constraint against it
        column_versions_id,
        table_coefficients,
        column_coefficients_id,
        column_coefficients_version_id,
        constraint_coefficients_version_id,
    ];
    assert_eq!(expected.len(), ordered.len());
    for i in 0..expected.len() {
        assert_eq!(expected[i], ordered[i]);
    }
}