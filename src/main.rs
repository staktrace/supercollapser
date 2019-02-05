#[macro_use]
extern crate log;
extern crate env_logger;

use std::env;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::iter::FromIterator;
use std::vec::Vec;

#[derive(Debug)]
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
        // MacOS rules
        CollapseRule::new(vec![
            "(os == \"mac\")",
        ], vec![
            "(version == \"OS X 10.10.5\")",
        ]),
        CollapseRule::new(vec![
            "(os == \"mac\")",
        ], vec![
            "e10s",
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

        // Win32 rules
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"6.1.7601\")",
        ], vec![
            "e10s",
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

        // Win64 rules
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"10.0.15063\")",
        ], vec![
            "e10s",
        ]),
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"10.0.15063\")",
        ], vec![
            "(processor == \"x86_64\")",
        ]),
        CollapseRule::new(vec![
            "(os == \"win\")",
            "(version == \"10.0.15063\")",
        ], vec![
            "(bits == 64)",
        ]),

        // Win version collapsing
        CollapseRule::new(vec![
            "(os == \"win\")",
        ], vec![
            "(version == \"6.1.7601\")",
            "(version == \"10.0.15063\")",
        ]),

        // Linux rules
        CollapseRule::new(vec![
            "(os == \"linux\")",
        ], vec![
            "(version == \"Ubuntu 16.04\")",
        ]),
        CollapseRule::new(vec![
            "(os == \"linux\")",
            "(processor == \"x86_64\")",
        ], vec![
            "(bits == 64)",
        ]),
        CollapseRule::new(vec![
            "(os == \"linux\")",
            "(processor == \"x86\")",
        ], vec![
            "(bits == 32)",
        ]),
        CollapseRule::new(vec![
            "(os == \"linux\")",
            "(processor == \"x86\")",
        ], vec![
            "not webrender",
        ]),
        CollapseRule::new(vec![
            "(os == \"linux\")",
        ], vec![
            "(processor == \"x86_64\")",
            "(processor == \"x86\")",
        ]),
    ]
}

fn match_prereqs(rule: &CollapseRule, tokenset: &Vec<String>) -> bool {
    for prereq in &rule.prerequisites {
        if !tokenset.iter().any(|t| t == prereq) {
            return false;
        }
    }
    true
}

fn has_token(token: &String, tokenset: &Vec<String>) -> bool {
    tokenset.iter().any(|t| t == token)
}

fn strip_token(token: &String, tokenset: &Vec<String>) -> Vec<String> {
    tokenset.clone().into_iter().filter(|t| t != token).collect()
}

fn try_collapse(tokenset: &Vec<String>, rule: &CollapseRule) -> Option<Vec<String>> {
    if rule.alternatives.len() != 1 {
        return None;
    }
    if !match_prereqs(rule, tokenset) {
        return None;
    }
    if !has_token(&rule.alternatives[0], tokenset) {
        return None;
    }
    return Some(strip_token(&rule.alternatives[0], tokenset));
}

fn flip(token: &str) -> String {
    if token.find("not ") == Some(0) {
        token[4..].to_string()
    } else {
        "not ".to_string() + token
    }
}

fn try_collapse_flip(a: &Vec<String>, b: &Vec<String>) -> Option<Vec<String>> {
    if a.len() != b.len() {
        return None;
    }
    let mut result = Vec::new();
    let mut flipped = false;
    for tok in a {
        if b.contains(tok) {
            trace!("Token match {}", tok);
            result.push(tok.clone());
        } else if !flipped && b.contains(&flip(tok)) {
            trace!("Flipped {}", tok);
            flipped = true;
        } else {
            trace!("Token mismatch {}", tok);
            return None;
        }
    }
    Some(result)
}

fn remaining_alt<'a>(used_alt: &str, rule: &'a CollapseRule) -> Option<&'a str> {
    assert!(rule.alternatives.len() == 2);
    if rule.alternatives[0] == used_alt {
        Some(&rule.alternatives[1])
    } else if rule.alternatives[1] == used_alt {
        Some(&rule.alternatives[0])
    } else {
        None
    }
}

fn try_collapse2(a: &Vec<String>, b: &Vec<String>, rule: &CollapseRule) -> Option<Vec<String>> {
    if rule.alternatives.len() != 2 {
        return None;
    }
    if a.len() != b.len() {
        return None;
    }
    if !match_prereqs(rule, a) {
        return None;
    }
    if !match_prereqs(rule, b) {
        return None;
    }
    let mut result = Vec::new();
    let mut matched = false;
    for tok in a {
        if b.contains(tok) {
            trace!("Token match {}", tok);
            result.push(tok.clone());
            continue;
        } else if matched {
            trace!("Token mismatch {}", tok);
            return None;
        }
        if let Some(alt) = remaining_alt(tok, rule) {
            if b.contains(&String::from(alt)) {
                trace!("Matched alternatives {},{}", tok, alt);
                matched = true;
                continue;
            }
        }
    }
    Some(result)
}

fn collapse(tokensets: &mut Vec<Vec<String>>) {
    let rules = build_collapse_rules();

    let mut changed = false;
    loop {
        'outer: for i in 0..tokensets.len() {
            for rule in &rules {
                if let Some(set) = try_collapse(&tokensets[i], rule) {
                    debug!("Collapsed {:?} to {:?} via {:?}", tokensets[i], set, rule);
                    tokensets[i] = set;
                }
            }
            for j in 0..i {
                if let Some(set) = try_collapse_flip(&tokensets[i], &tokensets[j]) {
                    debug!("Collapsed {:?} and {:?} to {:?} via flip", tokensets[i], tokensets[j], set);
                    tokensets[j] = set;
                    tokensets.remove(i);
                    changed = true;
                    break 'outer;
                }
                for rule in &rules {
                    if let Some(set) = try_collapse2(&tokensets[i], &tokensets[j], rule) {
                        debug!("Collapsed {:?} and {:?} to {:?} via {:?}", tokensets[i], tokensets[j], set, rule);
                        tokensets[j] = set;
                        tokensets.remove(i);
                        changed = true;
                        break 'outer;
                    }
                }
            }
        }
        if !changed {
            break;
        }
        changed = false;
    }
/*
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
*/
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
