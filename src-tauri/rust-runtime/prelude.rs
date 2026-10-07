// ---- SweepCode prelude: what LeetCode's Rust environment provides ----
// Glob imports never clash with a `use` the user writes themselves.
#![allow(dead_code, unused_imports, unused_mut, unused_variables, non_snake_case)]
use std::cell::*;
use std::cmp::*;
use std::collections::*;
use std::rc::*;

mod pw_types {
    use std::cell::RefCell;
    use std::rc::Rc;

    pub struct Solution;

    #[derive(PartialEq, Eq, Clone, Debug)]
    pub struct ListNode {
        pub val: i32,
        pub next: Option<Box<ListNode>>,
    }
    impl ListNode {
        #[inline]
        pub fn new(val: i32) -> Self {
            ListNode { next: None, val }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct TreeNode {
        pub val: i32,
        pub left: Option<Rc<RefCell<TreeNode>>>,
        pub right: Option<Rc<RefCell<TreeNode>>>,
    }
    impl TreeNode {
        #[inline]
        pub fn new(val: i32) -> Self {
            TreeNode { val, left: None, right: None }
        }
    }
}
use pw_types::*;
