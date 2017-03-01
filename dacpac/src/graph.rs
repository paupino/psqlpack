use std::collections::{HashMap,HashSet};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Node {
    Schema(String),
    Table(String),
    Column(String),
    Constraint(String),
    Function(String),
}

pub struct DependencyGraph {
    edges: HashMap<Node, Vec<Node>>,
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
            pre : DependencyGraph::fixed_len::<usize>(size, 0),
            post : DependencyGraph::fixed_len::<usize>(size, 0),
            pre_order : Vec::new(),
            post_order : Vec::new(),
            marked : DependencyGraph::fixed_len::<bool>(size, false),
            pre_counter : 0,
            post_counter : 0,
        }
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

impl DependencyGraph {

    pub fn new() -> Self {
        DependencyGraph {
            edges: HashMap::new(),
            unresolved: HashSet::new(),
        }
    }

    pub fn add_node(&mut self, node: &Node) {
        self.add_node_with_dependencies(node, Vec::new());
    }

    pub fn add_node_with_dependencies(&mut self, node: &Node, dependencies: Vec<Node>) {
        // If it has already been added then panic
        if self.edges.contains_key(node) {
            panic!("Node has already been registered: {:?}", node);
        }
        // Remove it from unresolved if it exists
        self.unresolved.remove(node);
        self.edges.insert(node.clone(), dependencies);
    }

    pub fn add_dependency(&mut self, node: &Node, dependency: &Node) {
        let dependency_known = self.edges.contains_key(dependency);
        if let Some(n) = self.edges.get_mut(node) {
            // Clone instead of copy, perhaps an optimization here later on
            n.push(dependency.clone());
            // Only add the dependency if necessary
            if !dependency_known {
                self.unresolved.insert(dependency.clone());
            }
        } else {
            panic!("Cannot add dependencies as node not known: {:?}", *node);
        }
    }

    pub fn validate(&self) -> ValidationResult {
        if !self.unresolved.is_empty() {
            return ValidationResult::UnresolvedDependencies;
        } 

        // Check for circular references. It's only circular if the root edge is seen twice.
        for edge in self.edges.keys() {
            if self.visit_dependency(edge, edge) {
                return ValidationResult::CircularReference;
            }
        }

        ValidationResult::Valid
    }

    pub fn unresolved(&self) -> Vec<Node> {
        self.unresolved.iter().map(|x| x.clone()).collect()
    }

    pub fn topological_graph(&self) -> Vec<Node> {
        // First of all, put the keys into a vec that we can use to build a directed acyclic graph
        let mut graph : Vec<Node> = Vec::new();
        for edge in self.edges.keys() {
            graph.push(edge.clone());
        }
        graph.sort();

        // Create the state
        let mut state = DepthFirstSearchState::new(graph.len());

        // Do a DFS and compute preorder/postorder
        println!();
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
        if let Some(dependencies) = self.edges.get(&graph[v]) {
            for dependency in dependencies {
                // Look up location in graph
                if let Ok(w) = graph.binary_search(dependency) {
                    if !state.marked[w] {
                        self.dfs(graph, w, state);
                    }
                }
            }
        }
        state.update_post_order(v);
    }

    fn fixed_len<T>(size: usize, def: T) -> Vec<T> where T: Copy {
        let mut zero_vec: Vec<T> = Vec::with_capacity(size);
        for _ in 0..size {
            zero_vec.push(def);
        }
        zero_vec
    }

    fn visit_dependency(&self, edge: &Node, lookup: &Node) -> bool {
        if let Some(dependencies) = self.edges.get(lookup) {
            for dependency in dependencies {
                if dependency.eq(edge) {
                    return true;
                }

                if self.visit_dependency(edge, dependency) {
                    return true;
                }
            }
        }
        false
    }
}

#[test]
#[should_panic]
fn it_panics_if_adding_a_duplicate_node() {
    let mut graph = DependencyGraph::new();
    let schema_public = Node::Schema("public".to_owned());
    graph.add_node(&schema_public);
    // Clone just in case
    graph.add_node(&schema_public.clone());
}

#[test]
#[should_panic]
fn it_panics_if_adding_a_dependency_to_a_node_that_doesnt_exist() {
    let mut graph = DependencyGraph::new();
    let schema_public = Node::Schema("public".to_owned());
    let table_org = Node::Table("public.org".to_owned());
    graph.add_node(&schema_public);
    graph.add_dependency(&table_org, &schema_public);    
}

#[test]
fn it_tracks_dependencies() {
    let mut graph = DependencyGraph::new();
    let schema_public = Node::Schema("public".to_owned());
    let table_org = Node::Table("public.org".to_owned());
    let table_user = Node::Table("public.user".to_owned());
    graph.add_node(&table_org);
    graph.add_node(&table_user);
    assert_eq!(ValidationResult::Valid, graph.validate());
    graph.add_dependency(&table_org, &schema_public);
    graph.add_dependency(&table_user, &schema_public);
    assert_eq!(ValidationResult::UnresolvedDependencies, graph.validate());
    let unresolved = graph.unresolved();
    assert_eq!(1, unresolved.len());
    assert_eq!(Node::Schema("public".to_owned()), unresolved[0]);

    graph.add_node(&schema_public);
    assert_eq!(ValidationResult::Valid, graph.validate());
}

#[test]
fn it_detects_circular_dependencies() {
    let mut graph = DependencyGraph::new();
    let schema_public = Node::Schema("public".to_owned());
    let table_org = Node::Table("public.org".to_owned());
    let table_user = Node::Table("public.user".to_owned());
    graph.add_node(&schema_public);
    graph.add_node(&table_org);
    graph.add_node(&table_user);
    graph.add_dependency(&table_org, &schema_public);
    graph.add_dependency(&table_user, &schema_public);
    // Not realistic, but testing circular references nevertheless
    graph.add_dependency(&table_org, &table_user);
    graph.add_dependency(&table_user, &table_org);
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
    let schema_data = Node::Schema("data".to_owned());
    let table_versions = Node::Table("data.versions".to_owned());
    let table_coefficients = Node::Table("data.coefficients".to_owned());
    let column_versions_id = Node::Column("data.versions.id".to_owned());
    let column_coefficients_id = Node::Column("data.coefficients.id".to_owned());
    let column_coefficients_version_id = Node::Column("data.coefficients.version_id".to_owned());
    let constraint_coefficients_version_id = Node::Constraint("fk_coefficients__version_id".to_owned());

    // Note: We do this in an unordered way purposely

    // Coefficients table - needs the schema
    graph.add_node_with_dependencies(&table_coefficients, vec!(schema_data.clone()));
    // Add each column for this table, each column needs the table to exist
    graph.add_node_with_dependencies(&column_coefficients_id, vec!(table_coefficients.clone())); 
    graph.add_node_with_dependencies(&column_coefficients_version_id, vec!(table_coefficients.clone())); 
    // Add constraint - the constraint needs the columns created
    graph.add_node_with_dependencies(&constraint_coefficients_version_id, vec!(
        column_coefficients_version_id.clone(), // FK
        column_versions_id.clone(), // Reference
        ));

    // Versions table - needs the schema
    graph.add_node_with_dependencies(&table_versions, vec!(schema_data.clone()));
    // The column needs the table
    graph.add_node_with_dependencies(&column_versions_id, vec!(table_versions.clone()));

    // Add the schema also - no dependencies
    graph.add_node(&schema_data);

    // Now, let's validate to make sure it's a valid graph
    assert_eq!(ValidationResult::Valid, graph.validate());

    // Now, order it.
    let ordered = graph.topological_graph();

    // The expected order:
    let expected = [
        schema_data, // Schema is first, nothing can exist without it
        table_coefficients,
        table_versions,
        column_coefficients_id,
        column_coefficients_version_id,
        column_versions_id,
        constraint_coefficients_version_id,
    ];
    assert_eq!(expected.len(), ordered.len());
    for i in 0..expected.len() {
        assert_eq!(expected[i], ordered[i]);
    }
}