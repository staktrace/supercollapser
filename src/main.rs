use std::env;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::iter::FromIterator;
use std::vec::Vec;

pub struct ConditionNode {
    pub condition: String,
    pub node_type: NodeType,
}

impl ConditionNode {
    pub fn new(condition: &str, subnodes: Vec<ConditionNode>) -> Self {
        ConditionNode {
            condition: condition.to_string(),
            node_type: NodeType::SubConditions(subnodes),
        }
    }

    pub fn new_leaf(condition: &str) -> Self {
        ConditionNode {
            condition: condition.to_string(),
            node_type: NodeType::Leaf(false),
        }
    }
}

pub enum NodeType {
    Leaf(bool),
    SubConditions(Vec<ConditionNode>),
}

fn build_condition_tree() -> Vec<ConditionNode> {
    vec![
        ConditionNode::new("(os == \"linux\")", vec![
            ConditionNode::new("webrender", vec![
                ConditionNode::new_leaf("debug"),
                ConditionNode::new_leaf("not debug"),
            ]),
            ConditionNode::new("not webrender", vec![
                ConditionNode::new("(bits == 64)", vec![
                    ConditionNode::new("debug", vec![
                        ConditionNode::new_leaf("sw"),
                        ConditionNode::new_leaf("not sw"),
                    ]),
                    ConditionNode::new_leaf("not debug"),
                ]),
                ConditionNode::new("(bits == 32)", vec![
                    ConditionNode::new("debug", vec![
                        ConditionNode::new_leaf("e10s"),
                        ConditionNode::new_leaf("not e10s"),
                    ]),
                    ConditionNode::new_leaf("not debug"),
                ]),
            ]),
        ]),
        ConditionNode::new("(os == \"mac\")", vec![
            ConditionNode::new_leaf("debug"),
            ConditionNode::new_leaf("not debug"),
        ]),
        ConditionNode::new("(os == \"win\")", vec![
            ConditionNode::new("(version == \"6.1.7601\")", vec![
                ConditionNode::new_leaf("debug"),
                ConditionNode::new_leaf("not debug"),
            ]),
            ConditionNode::new("(version == \"10.0.15063\")", vec![
                ConditionNode::new("webrender", vec![
                    ConditionNode::new_leaf("debug"),
                    ConditionNode::new_leaf("not debug"),
                ]),
                ConditionNode::new("not webrender", vec![
                    ConditionNode::new_leaf("debug"),
                    ConditionNode::new_leaf("not debug"),
                ]),
            ]),
        ]),
        ConditionNode::new_leaf("(os == android)"),
    ]
}

fn mark_all_leafs(tree: &mut Vec<ConditionNode>) {
    for condition in tree {
        match condition.node_type {
            NodeType::Leaf(ref mut flag) => *flag = true,
            NodeType::SubConditions(ref mut subs) => mark_all_leafs(subs),
        }
    }
}

fn is_fully_true(tree: &Vec<ConditionNode>) -> bool {
    for condition in tree {
        match condition.node_type {
            NodeType::Leaf(ref flag) => {
                if !*flag {
                    return false;
                }
            }
            NodeType::SubConditions(ref subs) => {
                if !is_fully_true(subs) {
                    return false;
                }
            }
        }
    }
    true
}

fn is_fully_false(tree: &Vec<ConditionNode>) -> bool {
    for condition in tree {
        match condition.node_type {
            NodeType::Leaf(ref flag) => {
                if *flag {
                    return false;
                }
            }
            NodeType::SubConditions(ref subs) => {
                if !is_fully_false(subs) {
                    return false;
                }
            }
        }
    }
    true
}

fn apply_tokenset_to_tree(tokens: &Vec<String>, tree: &mut Vec<ConditionNode>) {
    let mut matches = 0;
    for condition in tree.iter() {
        if tokens.contains(&condition.condition) {
            matches += 1;
        }
    }
    if matches > 0 {
        panic!("Tokenset {} matched {} condition branches!", tokens.join(","), matches);
    } else if matches == 0 {
        mark_all_leafs(tree);
        return;
    }

    fn walk_down<'a>(tokens: &Vec<String>, tree: &'a mut Vec<ConditionNode>) -> Option<&'a mut Vec<ConditionNode>> {
        for condition in tree.iter_mut() {
            if tokens.contains(&condition.condition) {
                match condition.node_type {
                    NodeType::Leaf(ref mut flag) => {
                        *flag = true;
                        return None;
                    }
                    NodeType::SubConditions(ref mut subs) => {
                        return Some(subs);
                    }
                };
            }
        }
        panic!("We should have taken the matches == 0 branch above");
    }

    if let Some(subtree) = walk_down(tokens, tree) {
        apply_tokenset_to_tree(tokens, subtree);
    }
}

fn build_tokensets( // recursive function to generate tokensets from a tree
    tree: &Vec<ConditionNode>, // source
    tokensets: &mut Vec<Vec<String>>, // destination
    current_conditions: &mut Vec<String>, // recursive information
) {
    for condition in tree {
        match condition.node_type {
            NodeType::Leaf(ref flag) => {
                if *flag {
                    let mut tokenset = current_conditions.clone();
                    tokenset.push(condition.condition.clone());
                    tokensets.push(tokenset);
                }
            }
            NodeType::SubConditions(ref subs) => {
                if is_fully_true(subs) {
                    let mut tokenset = current_conditions.clone();
                    tokenset.push(condition.condition.clone());
                    tokensets.push(tokenset);
                } else {
                    current_conditions.push(condition.condition.clone());
                    build_tokensets(subs, tokensets, current_conditions);
                    current_conditions.pop();
                }
            }
        }
    }
}

fn count_inverted_tokensets(tree: &Vec<ConditionNode>) -> usize {
    let mut count = 0;
    for condition in tree {
        match condition.node_type {
            NodeType::Leaf(ref flag) => {
                if !flag {
                    count += 1;
                }
            }
            NodeType::SubConditions(ref subs) => {
                if is_fully_false(subs) {
                    count += 1;
                } else {
                    count += count_inverted_tokensets(subs);
                }
            }
        }
    }
    count
}

fn collapse(tokensets: &mut Vec<Vec<String>>) {
    let mut condition_tree = build_condition_tree();
    for tokenset in tokensets.iter() {
        apply_tokenset_to_tree(tokenset, &mut condition_tree);
    }
    tokensets.clear();
    let mut current_conditions = vec![];
    build_tokensets(&condition_tree, tokensets, &mut current_conditions);

    let inverted_count = count_inverted_tokensets(&condition_tree);
    if inverted_count < tokensets.len() {
        eprintln!("Can represent the inverted state with fewer conditions, consider changing the default!");
    }
}

fn emit(tokensets: &Vec<Vec<String>>, set_prefix: &Option<String>, set_suffix: &Option<String>) {
    for set in tokensets {
        let combined = Vec::from_iter(set.iter().map(|s| s.clone())).join(" and ");
        println!("{}{}{}",
                 set_prefix.as_ref().unwrap(),
                 combined,
                 set_suffix.as_ref().unwrap());
    }
}

fn main() {
    let file = File::open(env::args().skip(1).next().unwrap()).unwrap();
    let reader = BufReader::new(&file);
    let mut tokensets : Vec<Vec<String>> = Vec::new();
    let mut set_prefix = None;
    let mut set_suffix = None;
    for line in reader.lines() {
        let line = line.unwrap();
        let prefix = line.find("if ").map(|ix| line[0..ix + 3].to_string());
        let suffix = line.rfind(':').map(|ix| line[ix..].to_string());
        let part_of_set = match (&set_prefix, &prefix) {
            (&Some(ref x), &Some(ref y)) if x != y => false,
            (_, &None) => false,
            _ => true,
        } && match (&set_suffix, &suffix) {
            (&Some(ref x), &Some(ref y)) if x != y => false,
            (_, &None) => false,
            _ => true,
        };

        if !part_of_set && tokensets.len() > 0 {
            collapse(&mut tokensets);
            emit(&tokensets, &set_prefix, &set_suffix);
            tokensets.clear();
        }

        if line.trim_left().starts_with("if ") {
            set_prefix = prefix;
            set_suffix = suffix;
            let prefix_len = set_prefix.as_ref().unwrap().len();
            let suffix_len = set_suffix.as_ref().unwrap().len();
            let tokens = line[prefix_len .. line.len() - suffix_len].split(" and ").map(String::from).collect();
            //eprintln!("Collecting tokenset {:?}", tokens);
            tokensets.push(tokens);
            continue;
        } else {
            println!("{}", line);
        }
    }
    if tokensets.len() > 0 {
        collapse(&mut tokensets);
        emit(&tokensets, &set_prefix, &set_suffix);
    }
}
