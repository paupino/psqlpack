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

    pub fn topological_sort(&self) -> Vec<Node> {

        // The general idea here is that we're ordering the nodes first up in weighted order
        // We then loop through and remove any without dependencies and put them into a new ordered list
        // Because of weighting, those with most importance will go first.
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

        // Now go through and build my new ordered graph.
        // Pretty inefficient - perhaps we can optimize this in the future
        let mut ordered = Vec::new();
        while !weighted_graph.is_empty() {
            for &(ref node, ..) in &weighted_graph {
                if let Some(edges) = self.edges.get(&node) {
                    if edges.is_empty() {
                        ordered.push(node.clone());
                    } else {
                        let mut edgeless = true;
                        for e in edges {
                            if !ordered.contains(&e.node) {
                                edgeless = false;
                                break;
                            }
                        }
                        if edgeless {
                            ordered.push(node.clone());
                        }
                    }
                }
            }
            weighted_graph.retain(|ref x| !ordered.contains(&x.0));
        }
        ordered
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
fn it_can_output_a_topological_sort() {
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
    let ordered = graph.topological_sort();

    //TODO: Need to make this edge weighted as a constraint on a table defines that the other table has to exist first
    // (i.e. versions before coefficient)
    // The expected order:
    let expected = [
        table_versions, // Versions must be first as coefficients has a constraint against it
        table_coefficients,
        column_versions_id,
        column_coefficients_id,
        column_coefficients_version_id,
        constraint_coefficients_version_id,
    ];
    assert_eq!(expected.len(), ordered.len());
    for i in 0..expected.len() {
        assert_eq!(expected[i], ordered[i]);
    }
}
