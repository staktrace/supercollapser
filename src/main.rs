#[macro_use]
extern crate log;
extern crate env_logger;

use std::env;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::iter::FromIterator;
use std::vec::Vec;

pub struct CollapseRule {
    pub prerequisites: Vec<String>,
    pub alternatives: Vec<String>,
}

impl CollapseRule {
    pub fn new(prerequisites: Vec<&str>, alternatives: Vec<&str>) -> Self {
        CollapseRule {
            prerequisites: prerequisites.into_iter().map(String::from).collect(),
            alternatives: alternatives.into_iter().map(String::from).collect(),
        }
    }
}

fn build_collapse_rules() -> Vec<CollapseRule> {
    vec![
        CollapseRule::new(vec![
            "(os == \"mac\")",
        ], vec![
            "(version == \"OS X 10.10.5\")",
        ]),
        CollapseRule::new(vec![
            "(os == \"mac\")",
        ], vec![
            "not webrender",
        ]),
        CollapseRule::new(vec![
            "(os == \"mac\")",
        ], vec![
            "(processor == \"x86_64\")",
        ]),
        CollapseRule::new(vec![
            "(os == \"mac\")",
        ], vec![
            "(bits == 64)",
        ]),
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"6.1.7601\")",
        ], vec![
            "not webrender",
        ]),
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"6.1.7601\")",
        ], vec![
            "(processor == \"x86\")",
        ]),
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"6.1.7601\")",
        ], vec![
            "(bits == 32)",
        ]),
    ]
}

/*
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
*/

fn match_prereqs(rule: &CollapseRule, tokenset: &Vec<String>) -> bool {
    for prereq in &rule.prerequisites {
        if !tokenset.iter().any(|t| t == prereq) {
            return false;
        }
    }
    true
}

fn strip_token(token: &String, tokenset: &Vec<String>) -> Vec<String> {
    tokenset.clone().into_iter().filter(|t| t != token).collect()
}

fn collapse(tokensets: &mut Vec<Vec<String>>) {
    let rules = build_collapse_rules();

    //let mut changed = false;
    //loop {
        for rule in rules {
            let mut satisfying_prereqs = Vec::new();
            let mut i = 0;
            while i != tokensets.len() {
                if match_prereqs(&rule, &tokensets[i]) {
                    satisfying_prereqs.push(tokensets.remove(i));
                } else {
                    i += 1;
                }
            }
            if rule.alternatives.len() > satisfying_prereqs.len() {
                continue;
            }
            if rule.alternatives.len() == 1 {
                for i in 0..satisfying_prereqs.len() {
                    satisfying_prereqs[i] = strip_token(&rule.alternatives[0], &satisfying_prereqs[i]);
                }
            } else if rule.alternatives.len() == 2 {
            } else {
                unimplemented!("Add generic nCx implementation");
            }
            tokensets.extend(satisfying_prereqs.into_iter());
        }
        //if !changed {
        //    break;
        //}
        //changed = false;
    //}
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
