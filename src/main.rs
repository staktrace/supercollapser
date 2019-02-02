#[macro_use]
extern crate log;
extern crate env_logger;

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
                        ConditionNode::new_leaf("sw-e10s"),
                        ConditionNode::new_leaf("not sw-e10s"),
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
        ConditionNode::new_leaf("(os == \"android\")"),
    ]
}

static ALL_TOKENS : &'static [&'static str] = &[
    "(os == \"linux\")",
    "(os == \"win\")",
    "(os == \"mac\")",
    "(os == \"android\")",
    "webrender",
    "not webrender",
    "debug",
    "not debug",
    "(version == \"6.1.7601\")",
    "(version == \"10.0.15063\")",
    "(version == \"Ubuntu 16.04\")",
    "(version == \"OS X 10.10.5\")",
    "(bits == 64)",
    "(bits == 32)",
    "(processor == \"x86\")",
    "(processor == \"x86_64\")",
    "e10s",
    "not e10s",
    "sw-e10s",
    "not sw-e10s",
];

fn validate_tokenset(tokenset: &Vec<String>) -> bool {
    for token in tokenset {
        if !ALL_TOKENS.contains(&token.as_str()) {
            error!("Unrecognized token {}", token);
            return false;
        }
    }
    true
}

fn dump_tree(tree: &Vec<ConditionNode>, indent: usize) {
    for condition in tree {
        match condition.node_type {
            NodeType::Leaf(ref flag) => {
                debug!("{}{} => {}", "    ".repeat(indent), condition.condition, flag);
            }
            NodeType::SubConditions(ref subs) => {
                debug!("{}{}", "    ".repeat(indent), condition.condition);
                dump_tree(subs, indent + 1);
            }
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
    if matches > 1 {
        panic!("Tokenset {} matched {} condition branches!", tokens.join(","), matches);
    } else if matches == 0 {
        for condition in tree.iter_mut() {
            match condition.node_type {
                NodeType::Leaf(ref mut flag) => {
                    *flag = true;
                }
                NodeType::SubConditions(ref mut subs) => {
                    apply_tokenset_to_tree(tokens, subs);
                }
            }
        }
        return;
    }

    fn walk_down<'a>(tokens: &Vec<String>, tree: &'a mut Vec<ConditionNode>) -> Option<&'a mut Vec<ConditionNode>> {
        for condition in tree.iter_mut() {
            if tokens.contains(&condition.condition) {
                debug!("Matched {}", condition.condition);
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
    debug!("Initial tree state");
    dump_tree(&condition_tree, 1);
    for tokenset in tokensets.iter() {
        debug!("Applying tokenset {}", tokenset.join(", "));
        if !validate_tokenset(tokenset) {
            return;
        }
        apply_tokenset_to_tree(tokenset, &mut condition_tree);
        debug!("Tree state");
        dump_tree(&condition_tree, 1);
    }
    tokensets.clear();
    let mut current_conditions = vec![];
    build_tokensets(&condition_tree, tokensets, &mut current_conditions);

    let inverted_count = count_inverted_tokensets(&condition_tree);
    if inverted_count < tokensets.len() {
        info!("Can represent the inverted state with fewer conditions, consider changing the default!");
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
    env_logger::init();

    let file = File::open(env::args().skip(1).next().unwrap()).unwrap();
    let reader = BufReader::new(&file);
    let mut tokensets : Vec<Vec<String>> = Vec::new();
    let mut set_prefix = None;
    let mut set_suffix = None;
    for line in reader.lines() {
        let line = line.unwrap();
        let prefix = line.find("if ").map(|ix| line[0..ix + 3].to_string());
        let suffix = line.find(':').map(|ix| line[ix..].to_string());
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
            let tokens = line[prefix_len .. line.len() - suffix_len]
                .split(" and ")
                .map(|s| {
                    let t = s.trim();
                    if t.contains("==") && t.chars().next() != Some('(') {
                        let mut parensized = String::from("(");
                        parensized.push_str(t);
                        parensized.push(')');
                        parensized
                    } else {
                        String::from(s)
                    }
                })
                .collect();
            debug!("Collecting tokenset {:?}", tokens);
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
